//! M12-B structural oracle for `pictura_render::document_ops`.
//!
//! These tests check *structure*, not pixels: after each document operation the
//! layer stack must still serialize through `write_psd` → `read_psd` with the
//! same dimensions, layer count, names, and rects, and the cached
//! `doc.composite` must equal `composite_rgba(&doc)`. The exact orientation and
//! canvas remaps are checked for identity, and a `psd-tools` check (skipped when
//! the tool is absent) confirms an independent PSD reader sees the same
//! dimensions and layer count. See `tests/README.md` for the mapping and
//! tolerances.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use pictura_codec::{read_psd, write_psd};
use pictura_core::{
    BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LayerMask, LockFlags,
    PsdRect,
};
use pictura_ops::{Anchor, Resample};
use pictura_render::{
    composite_rgba, flip_document, resize_canvas_document, resize_document, rotate_document,
    rotate_document_in,
};
use pictura_testkit::compare;

// --- document builders (same layer-builder style as the unit tests) ---------

fn rect(top: i32, left: i32, bottom: i32, right: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
    }
}

fn plane(base: u8, n: usize) -> Vec<u8> {
    (0..n).map(|i| base.wrapping_add(i as u8)).collect()
}

fn pixel_layer(name: &str, r: PsdRect, mask: Option<LayerMask>) -> Layer {
    let n = (r.width().max(0) * r.height().max(0)) as usize;
    Layer {
        name: name.into(),
        rect: r,
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask,
        adjustment: None,
        channels: vec![
            Channel {
                id: 0,
                data: plane(0, n).into(),
            },
            Channel {
                id: 1,
                data: plane(60, n).into(),
            },
            Channel {
                id: 2,
                data: plane(120, n).into(),
            },
            Channel {
                id: -1,
                data: plane(200, n).into(),
            },
        ],
        children: Vec::new(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

fn group(name: &str, children: Vec<Layer>) -> Layer {
    Layer {
        name: name.into(),
        rect: rect(0, 0, 0, 0),
        blend: BlendMode::PassThrough,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: Vec::new(),
        children,
        is_group: true,
        background: false,
        ..Default::default()
    }
}

/// Non-square 6×4 document: a full layer, a masked layer, and a group with one
/// child (exercises the group recursion), plus a document-level channel. The
/// composite is canonical so full-document equality is a valid identity check.
fn sample_doc() -> Document {
    let mut doc = Document::new(6, 4, ColorMode::Rgb, BitDepth::Eight);
    let mask = LayerMask {
        rect: rect(1, 1, 3, 5),
        default_color: 255,
        disabled: false,
        flags: 0,
        data: Some(plane(10, 8).into()),
        ..Default::default()
    };
    doc.layers = vec![
        pixel_layer("base", rect(0, 0, 4, 6), None),
        pixel_layer("masked", rect(1, 1, 3, 5), Some(mask)),
        group("grp", vec![pixel_layer("inner", rect(0, 0, 2, 3), None)]),
    ];
    doc.channels = vec![Channel {
        id: -1,
        data: plane(0, 24).into(),
    }];
    doc.composite = composite_rgba(&doc);
    doc
}

/// Plain two-layer document for the external `psd-tools` check.
fn plain_doc() -> Document {
    let mut doc = Document::new(8, 6, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![
        pixel_layer("bottom", rect(0, 0, 6, 8), None),
        pixel_layer("top", rect(0, 0, 3, 4), None),
    ];
    doc.composite = composite_rgba(&doc);
    doc
}

// --- structural helpers -----------------------------------------------------

fn round_trip(doc: &Document) -> Document {
    read_psd(&write_psd(doc).expect("write_psd")).expect("read_psd")
}

/// Separate from the assertion so the mismatch path can be tested directly
/// (`dimension_check_bites`) instead of only being exercised when correct.
fn check_dims(back: &Document, expected_w: u32, expected_h: u32) -> Result<(), String> {
    if back.width != expected_w || back.height != expected_h {
        return Err(format!(
            "expected {expected_w}x{expected_h}, read {}x{}",
            back.width, back.height
        ));
    }
    Ok(())
}

/// Depth-first `(name, rect)` list, groups before their children.
fn flattened(layers: &[Layer]) -> Vec<(String, PsdRect)> {
    fn walk(layers: &[Layer], out: &mut Vec<(String, PsdRect)>) {
        for layer in layers {
            out.push((layer.name.clone(), layer.rect));
            walk(&layer.children, out);
        }
    }
    let mut out = Vec::new();
    walk(layers, &mut out);
    out
}

/// Composite consistency plus structural survival through `write_psd`/`read_psd`.
fn assert_structural_oracle(doc: &Document, label: &str) {
    assert_eq!(
        doc.composite,
        composite_rgba(doc),
        "{label}: cached composite diverged from composite_rgba"
    );

    let back = round_trip(doc);
    check_dims(&back, doc.width, doc.height).unwrap_or_else(|e| panic!("{label}: {e}"));
    assert_eq!(
        back.layers.len(),
        doc.layers.len(),
        "{label}: top-level layer count"
    );
    assert_eq!(
        flattened(&back.layers),
        flattened(&doc.layers),
        "{label}: layer names/rects changed across write_psd/read_psd"
    );
}

// --- structural + composite tests, one per operation ------------------------

#[test]
fn resize_document_preserves_structure_and_composite() {
    let mut doc = sample_doc();
    resize_document(&mut doc, 12, 8, Resample::Nearest).unwrap();
    assert_eq!((doc.width, doc.height), (12, 8));
    assert_structural_oracle(&doc, "resize_document");
}

#[test]
fn resize_canvas_document_preserves_structure_and_composite() {
    let mut doc = sample_doc();
    resize_canvas_document(&mut doc, 8, 6, Anchor::Center).unwrap();
    assert_eq!((doc.width, doc.height), (8, 6));
    assert_structural_oracle(&doc, "resize_canvas_document");
}

#[test]
fn rotate_document_preserves_structure_and_composite() {
    let mut doc = sample_doc();
    rotate_document(&mut doc, 1).unwrap();
    assert_eq!((doc.width, doc.height), (4, 6));
    assert_structural_oracle(&doc, "rotate_document");
}

#[test]
fn flip_document_preserves_structure_and_composite() {
    let mut doc = sample_doc();
    flip_document(&mut doc, true);
    assert_structural_oracle(&doc, "flip_document");
}

// --- document-scope arbitrary rotation (rotate_document_in) -----------------

#[test]
fn rotate_document_in_right_angle_matches_exact_remap() {
    let mut a = sample_doc();
    let mut b = sample_doc();
    assert!(rotate_document_in(&mut a, 90.0, (3.0, 2.0)));
    rotate_document(&mut b, 1).unwrap();
    assert_eq!(
        a, b,
        "90° about the centre must equal the exact quarter-turn remap"
    );
}

#[test]
fn rotate_document_in_matches_rotate_in_kernel() {
    // One full-canvas layer: the doc-level resample is the buffer kernel.
    let mut doc = Document::new(6, 4, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![pixel_layer("only", rect(0, 0, 4, 6), None)];
    doc.composite = composite_rgba(&doc);
    let before = doc.layers[0].channels[0].data.to_vec();

    assert!(rotate_document_in(&mut doc, 30.0, (3.0, 2.0)));
    assert_structural_oracle(&doc, "rotate_document_in");

    let mut src = pictura_core::PixelBuffer::new(6, 4, 1);
    src.data.copy_from_slice(&before);
    let want = pictura_ops::rotate_arbitrary(&src, 30.0, [0, 0, 0, 0]).unwrap();
    assert_eq!(
        &doc.layers[0].channels[0].data[..],
        &want.data[..],
        "rotated channel must match the oracled rotate_in kernel exactly"
    );
}

// --- exactness --------------------------------------------------------------

#[test]
fn four_quarter_turns_are_identity() {
    let before = sample_doc();
    let mut doc = before.clone();
    for _ in 0..4 {
        rotate_document(&mut doc, 1).unwrap();
    }
    assert_eq!(doc, before, "4 × 90° must restore the document");
}

#[test]
fn flip_twice_is_identity() {
    for horizontal in [true, false] {
        let before = sample_doc();
        let mut doc = before.clone();
        flip_document(&mut doc, horizontal);
        flip_document(&mut doc, horizontal);
        assert_eq!(doc, before, "horizontal={horizontal}");
    }
}

/// Grow then shrink back to the original size with the opposite anchor: the
/// document dimensions return to the original (content is not expected to — the
/// content-preserving case is the centered identity below).
#[test]
fn canvas_grow_then_shrink_opposite_anchor_restores_dimensions() {
    let mut doc = sample_doc();
    let (w0, h0) = (doc.width, doc.height);
    resize_canvas_document(&mut doc, w0 + 2, h0 + 2, Anchor::TopLeft).unwrap();
    resize_canvas_document(&mut doc, w0, h0, Anchor::BottomRight).unwrap();
    assert_eq!((doc.width, doc.height), (w0, h0));
    assert_structural_oracle(&doc, "canvas grow/shrink");
}

#[test]
fn canvas_center_grow_then_shrink_restores_document() {
    let before = sample_doc();
    let mut doc = before.clone();
    resize_canvas_document(
        &mut doc,
        before.width + 2,
        before.height + 2,
        Anchor::Center,
    )
    .unwrap();
    resize_canvas_document(&mut doc, before.width, before.height, Anchor::Center).unwrap();
    assert_eq!(doc, before, "centered grow then shrink must be identity");
}

// --- proof the dimension check bites ----------------------------------------

#[test]
fn dimension_check_bites() {
    let mut doc = sample_doc();
    resize_document(&mut doc, 12, 8, Resample::Nearest).unwrap();
    let back = round_trip(&doc);

    assert!(check_dims(&back, 12, 8).is_ok(), "correct size must pass");
    assert!(check_dims(&back, 13, 8).is_err(), "wrong width must fail");
    assert!(check_dims(&back, 12, 9).is_err(), "wrong height must fail");
}

// --- independent psd-tools check --------------------------------------------

fn psd_tools_available() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("python3")
            .args(["-c", "import psd_tools"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

/// Write `doc` to a scratch file and have `scripts/validate_output.py` open it
/// with `psd-tools`, then assert the header reports our dimensions and layer
/// count. Returns early with a message when `python3`/`psd_tools` is missing.
fn assert_psd_tools_sees(doc: &Document, tag: &str) {
    if !psd_tools_available() {
        eprintln!("skipping psd-tools check for {tag}: python3 + psd_tools not available");
        return;
    }

    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/validate_output.py");
    let dir = scratch_dir(tag);
    let path = dir.join(format!("{tag}.psd"));
    std::fs::write(&path, write_psd(doc).unwrap()).unwrap();

    let out = Command::new("python3")
        .arg(&script)
        .arg(&path)
        .output()
        .expect("run scripts/validate_output.py");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        out.status.success(),
        "validate_output.py failed for {tag}:\n{stdout}\n{stderr}"
    );

    // Header: `<path>: <W>x<H> mode=<MODE> depth=<N> layers=<K>`.
    let header = stdout
        .lines()
        .next()
        .expect("validate_output.py header line");
    eprintln!("psd-tools {tag}: {header}");
    let (_, rest) = header
        .split_once(": ")
        .unwrap_or_else(|| panic!("unexpected header: {header}"));
    let mut tokens = rest.split_whitespace();
    let dims = tokens.next().expect("dims token");
    let (pw, ph) = dims.split_once('x').expect("WxH token");
    assert_eq!(
        pw.parse::<u32>().unwrap(),
        doc.width,
        "psd-tools width ({tag})"
    );
    assert_eq!(
        ph.parse::<u32>().unwrap(),
        doc.height,
        "psd-tools height ({tag})"
    );
    let layers = tokens
        .find_map(|t| t.strip_prefix("layers="))
        .expect("layers token")
        .parse::<usize>()
        .unwrap();
    assert_eq!(layers, doc.layers.len(), "psd-tools layer count ({tag})");
}

#[test]
fn psd_tools_sees_resized_document() {
    let mut doc = plain_doc();
    resize_document(&mut doc, 16, 12, Resample::Nearest).unwrap();
    assert_psd_tools_sees(&doc, "resized");
}

#[test]
fn psd_tools_sees_rotated_document() {
    let mut doc = plain_doc();
    rotate_document(&mut doc, 1).unwrap();
    assert_psd_tools_sees(&doc, "rotated");
}

/// Unique scratch directory per call so tests can run in parallel.
fn scratch_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "pictura-render-doc-oracle-{}-{tag}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// --- ImageMagick differential oracle for rotate_document_in -----------------

fn ops_oracle_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/ops_oracle.py")
}

fn magick_available() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("magick")
            .arg("-version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

/// The central `k×k` region of a planar buffer, channel planes preserved.
fn center_crop(data: &[u8], width: u32, height: u32, channels: u8, k: u32) -> Vec<u8> {
    assert!(k <= width && k <= height, "crop larger than the buffer");
    let ch = channels as usize;
    let (kw, w) = (k as usize, width as usize);
    let ox = (width - k) / 2;
    let oy = (height - k) / 2;
    let plane = w * height as usize;
    let mut out = vec![0u8; kw * kw * ch];
    for c in 0..ch {
        for y in 0..kw {
            for x in 0..kw {
                out[c * kw * kw + y * kw + x] =
                    data[c * plane + (y + oy as usize) * w + (x + ox as usize)];
            }
        }
    }
    out
}

fn oracle_rotate(input_planar: &[u8], size: &str, angle: &str) -> (Vec<u8>, (u32, u32)) {
    let dir = scratch_dir("rotate-in-oracle");
    let input_path = dir.join("in.rgb");
    let output_path = dir.join("out.rgb");
    std::fs::write(&input_path, input_planar).unwrap();
    let result = Command::new("python3")
        .arg(ops_oracle_script())
        .arg("apply")
        .args(["--size", size, "--channels", "3", "--planar"])
        .args([
            "--op",
            "rotate_arbitrary",
            "--angle",
            angle,
            "--filter",
            "triangle",
            "--background",
            "0,0,0,0",
        ])
        .arg(&input_path)
        .arg(&output_path)
        .output()
        .expect("run ops_oracle.py apply");
    assert!(
        result.status.success(),
        "oracle failed:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = String::from_utf8_lossy(&result.stdout);
    let dims = text.split_whitespace().last().expect("output size");
    let (w, h) = dims.split_once('x').expect("WxH");
    let bytes = std::fs::read(&output_path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    (bytes, (w.parse().unwrap(), h.parse().unwrap()))
}

#[test]
fn rotate_document_in_matches_imagemagick_central_region() {
    if !magick_available() {
        eprintln!("skipping rotate_document_in oracle: `magick` not on PATH");
        return;
    }
    let (w, h) = (16u32, 16u32);
    let plane = (w * h) as usize;
    // Same smooth ramp + 4x4 checker the ops oracle uses, so the central
    // region has no hard edge that would defeat the triangle-vs-bilinear diff.
    let mut planar = vec![0u8; plane * 3];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            planar[i] = (x * 16) as u8;
            planar[plane + i] = (y * 16) as u8;
            planar[2 * plane + i] = if (x / 4 + y / 4) % 2 == 0 { 40 } else { 210 };
        }
    }

    let mut layer = pixel_layer("only", rect(0, 0, h as i32, w as i32), None);
    for c in 0..3usize {
        layer.channels[c].data = planar[c * plane..(c + 1) * plane].to_vec().into();
    }
    let mut doc = Document::new(w, h, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = vec![layer];
    doc.composite = composite_rgba(&doc);

    let (reference, (iw, ih)) = oracle_rotate(&planar, "16x16", "30");
    assert!(
        iw >= w && ih >= h,
        "IM {iw}x{ih} smaller than Pictura {w}x{h}"
    );

    assert!(rotate_document_in(
        &mut doc,
        30.0,
        (w as f64 / 2.0, h as f64 / 2.0)
    ));
    let rplane = (doc.width * doc.height) as usize;
    let mut rotated = vec![0u8; rplane * 3];
    for c in 0..3usize {
        rotated[c * rplane..(c + 1) * rplane].copy_from_slice(&doc.layers[0].channels[c].data);
    }

    // Isolate: the doc-side must equal the oracled kernel exactly.
    {
        let mut src = pictura_core::PixelBuffer::new(w, h, 3);
        src.data.copy_from_slice(&planar);
        let kernel = pictura_ops::rotate_arbitrary(&src, 30.0, [0, 0, 0, 0]).unwrap();
        assert_eq!(kernel.width, doc.width, "dims");
        assert_eq!(kernel.height, doc.height, "dims");
        assert_eq!(&rotated[..], &kernel.data[..], "doc-side vs kernel");
    }

    const K: u32 = 12;
    let want = center_crop(&rotated, doc.width, doc.height, 3, K);
    let got = center_crop(&reference, iw, ih, 3, K);
    let diff = compare(&want, &got, 8).expect("buffer lengths agree");
    assert!(
        diff.is_empty(),
        "rotate_document_in 30 deg (central {K}x{K}): {} of {} over tolerance 8 (max {})",
        diff.differing,
        diff.samples,
        diff.max_delta
    );
}
