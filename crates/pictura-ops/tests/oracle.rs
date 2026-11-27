//! ImageMagick differential oracle for `pictura_ops` (task M10-B).
//!
//! This is a *sanity* oracle, not a parity oracle. The exact resample
//! kernels and rotation sampler are closed; ImageMagick implements the same
//! family of operations and is used as an independent reference. Each op with a
//! faithful operator is diffed against the ImageMagick result with
//! `pictura_testkit::compare`; the tolerance and the reason for it are in
//! `crates/pictura-ops/tests/README.md`.
//!
//! The right-angle turns, flips and canvas placement are exact integer remaps
//! and are expected bit-identical. The resamplers match at their faithful
//! ImageMagick filter (`point` / `triangle` / `catrom`), and `rotate_arbitrary`
//! is compared only on the central region because ImageMagick pads the rotated
//! bounding box and its edge sampler diverges from Pictura's bilinear map.
//!
//! Regenerate/inspect a result manually with `scripts/ops_oracle.py`; see
//! `tests/README.md`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use pictura_core::PixelBuffer;
use pictura_ops::{
    flip_horizontal, flip_vertical, resize, resize_canvas, rotate180, rotate90_ccw, rotate90_cw,
    rotate_arbitrary, Anchor, Resample,
};
use pictura_testkit::{compare, oracle};

/// One row of the op -> ImageMagick mapping table.
struct Mapping {
    op: &'static str,
    im: &'static str,
    tolerance: u8,
    note: &'static str,
}

const MAPPING: &[Mapping] = &[
    Mapping {
        op: "resize Nearest",
        im: "-filter point -resize WxH!",
        tolerance: 0,
        note: "point sample; measured max delta 0 (16->32 upscale)",
    },
    Mapping {
        op: "resize Bilinear",
        im: "-filter triangle -resize WxH!",
        tolerance: 0,
        note: "2x2 tent; measured max delta 0 (16->32). IM widens the kernel when downscaling, \
               so 16->8 measures 37 (no-equivalent at scale < 1)",
    },
    Mapping {
        op: "resize Bicubic",
        im: "-define filter:b=0 -define filter:c=0.75 -filter cubic -resize WxH!",
        tolerance: 1,
        note: "Mitchell-Netravali cubic(0, 0.75) (Keys a = -0.75), the publicly documented \
               the reference Bicubic; matches within tolerance 1 (16->32). Catmull-Rom (`catrom`) \
               is the wrong operator here",
    },
    Mapping {
        op: "resize_canvas",
        im: "-background 'rgba(r,g,b,a)' -gravity G -extent WxH",
        tolerance: 0,
        note: "exact for all nine anchors, grow 20x18 and shrink 12x10; measured max delta 0 \
               (3-channel)",
    },
    Mapping {
        op: "rotate90_cw",
        im: "-rotate 90",
        tolerance: 0,
        note: "exact integer remap on a non-square 16x12; measured max delta 0",
    },
    Mapping {
        op: "rotate90_ccw",
        im: "-rotate 270",
        tolerance: 0,
        note: "exact integer remap; measured max delta 0",
    },
    Mapping {
        op: "rotate180",
        im: "-rotate 180",
        tolerance: 0,
        note: "exact integer remap; measured max delta 0",
    },
    Mapping {
        op: "flip_horizontal",
        im: "-flop",
        tolerance: 0,
        note: "exact integer remap; measured max delta 0",
    },
    Mapping {
        op: "flip_vertical",
        im: "-flip",
        tolerance: 0,
        note: "exact integer remap; measured max delta 0",
    },
    Mapping {
        op: "rotate_arbitrary 30 deg",
        im: "-filter triangle -background 'rgba(0,0,0,0)' -rotate 30",
        tolerance: 8,
        note: "bilinear; IM pads the bbox by one row/col per side (24x24 vs Pictura 22x22 at 30 \
               deg). Central 12x12 measured max delta 8, mean 0.6; edges are background/bbox \
               no-equivalent, and IM diverges more at other angles (45 deg measured max 132)",
    },
];

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/ops_oracle.py")
}

/// Unique scratch directory per call so tests can run in parallel.
fn scratch_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "pictura-ops-oracle-{}-{tag}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Deterministic `width`x`height` planar RGB8 image: two ramps plus a 4x4
/// block checker so every op sees gradients, hard edges, and a non-square case.
fn test_image_planar(width: u32, height: u32) -> Vec<u8> {
    let pc = (width * height) as usize;
    let mut data = vec![0u8; pc * 3];
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize;
            data[i] = (x * 16) as u8;
            data[pc + i] = (y * 16) as u8;
            data[2 * pc + i] = if (x / 4 + y / 4) % 2 == 0 { 40 } else { 210 };
        }
    }
    data
}

fn test_image_buffer(width: u32, height: u32) -> PixelBuffer {
    PixelBuffer {
        width,
        height,
        channels: 3,
        data: test_image_planar(width, height).into(),
    }
}

/// Run `scripts/ops_oracle.py apply` and return `(planar result, (W, H))`.
/// `extra` holds the named op and its flags; the script prints the IM-sized
/// `WxH` on stdout.
fn oracle_at(
    extra: &[&str],
    input_planar: &[u8],
    size: &str,
    channels: u8,
) -> (Vec<u8>, (u32, u32)) {
    let dir = scratch_dir("oracle");
    let input_path = dir.join("in.rgb");
    let output_path = dir.join("out.rgb");
    std::fs::write(&input_path, input_planar).unwrap();
    let channels_arg = channels.to_string();
    let result = oracle::Command::new("python3")
        .arg(script())
        .arg("apply")
        .args(["--size", size, "--channels", &channels_arg, "--planar"])
        .args(extra)
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
    let dims = text
        .split_whitespace()
        .last()
        .expect("oracle printed an output size");
    let (w, h) = dims.split_once('x').expect("oracle size is WxH");
    let bytes = std::fs::read(&output_path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    (
        bytes,
        (w.parse().expect("width"), h.parse().expect("height")),
    )
}

/// The central `k`x`k` region of a planar buffer (channel planes preserved).
fn center_crop(data: &[u8], width: u32, height: u32, channels: u8, k: u32) -> Vec<u8> {
    assert!(k <= width && k <= height, "crop larger than the buffer");
    let ox = (width - k) / 2;
    let oy = (height - k) / 2;
    let (kw, w) = (k as usize, width as usize);
    let c = channels as usize;
    let mut out = vec![0u8; kw * kw * c];
    for ch in 0..c {
        for y in 0..kw {
            for x in 0..kw {
                out[ch * kw * kw + y * kw + x] =
                    data[ch * w * height as usize + (y + oy as usize) * w + (x + ox as usize)];
            }
        }
    }
    out
}

/// Diff `actual` against the ImageMagick oracle, asserting the dimensions match.
/// Skips (with a message) when `magick` is absent; never `#[ignore]`d.
fn differential_exact(
    label: &str,
    actual: &PixelBuffer,
    extra: &[&str],
    input: &[u8],
    size: &str,
    tolerance: u8,
) {
    if let Err(skip) = oracle::require(oracle::Tool::Magick) {
        skip.report();
        return;
    }
    let (reference, (iw, ih)) = oracle_at(extra, input, size, actual.channels);
    assert_eq!(
        (actual.width, actual.height),
        (iw, ih),
        "{label}: dimension mismatch"
    );
    let diff = compare(&actual.data, &reference, tolerance).expect("buffer lengths agree");
    assert!(
        diff.is_empty(),
        "{label}: {} of {} samples over tolerance {tolerance} (max delta {})",
        diff.differing,
        diff.samples,
        diff.max_delta
    );
}

#[test]
fn reference_script_is_present() {
    let path = script();
    assert!(
        path.is_file(),
        "missing oracle script at {}",
        path.display()
    );
    let help = oracle::Command::new("python3")
        .arg(&path)
        .arg("apply")
        .arg("--help")
        .output()
        .expect("run ops_oracle.py apply --help");
    assert!(
        help.status.success(),
        "`apply --help` failed:\n{}",
        String::from_utf8_lossy(&help.stderr)
    );
}

#[test]
fn imagemagick_runs() {
    if let Err(skip) = oracle::require(oracle::Tool::Magick) {
        skip.report();
        return;
    }
    let out = oracle::Command::new("python3")
        .arg(script())
        .arg("version")
        .output()
        .expect("run ops_oracle.py version");
    assert!(
        out.status.success(),
        "`version` failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("ImageMagick"),
        "unexpected version output:\n{text}"
    );
}

#[test]
fn mapping_documents_ops_and_tolerances() {
    assert_eq!(MAPPING.len(), 10, "one mapping row per differential");
    for m in MAPPING {
        assert!(!m.note.is_empty(), "{}: empty note", m.op);
        assert!(
            !m.im.is_empty() && m.tolerance <= 8,
            "{}: bad ImageMagick row",
            m.op
        );
    }
}

/// The oracle itself, independent of the ops: `-negate` must produce `255 - v`
/// for every planar sample (this also proves the planar round-trip).
#[test]
fn oracle_negate_matches_expected_bytes() {
    if let Err(skip) = oracle::require(oracle::Tool::Magick) {
        skip.report();
        return;
    }
    let input = test_image_planar(16, 16);
    let (output, dims) = oracle_at(&["--im-args=-negate"], &input, "16x16", 3);
    assert_eq!(dims, (16, 16));
    let expected: Vec<u8> = input.iter().map(|v| 255 - v).collect();
    assert_eq!(output, expected);
}

#[test]
fn resize_nearest_matches_imagemagick() {
    let src = test_image_buffer(16, 16);
    let out = resize(&src, 32, 32, Resample::Nearest).expect("resize");
    differential_exact(
        "resize Nearest 16->32",
        &out,
        &[
            "--op", "resize", "--width", "32", "--height", "32", "--filter", "point",
        ],
        &src.data,
        "16x16",
        0,
    );
}

#[test]
fn resize_bilinear_matches_imagemagick() {
    let src = test_image_buffer(16, 16);
    let out = resize(&src, 32, 32, Resample::Bilinear).expect("resize");
    differential_exact(
        "resize Bilinear 16->32",
        &out,
        &[
            "--op", "resize", "--width", "32", "--height", "32", "--filter", "triangle",
        ],
        &src.data,
        "16x16",
        0,
    );
}

#[test]
fn resize_bicubic_matches_imagemagick() {
    let src = test_image_buffer(16, 16);
    let out = resize(&src, 32, 32, Resample::Bicubic).expect("resize");
    differential_exact(
        "resize Bicubic 16->32",
        &out,
        &[
            "--op",
            "resize",
            "--width",
            "32",
            "--height",
            "32",
            "--filter",
            "cubic",
            "--im-args",
            "-define filter:b=0 -define filter:c=0.75",
        ],
        &src.data,
        "16x16",
        1,
    );
}

#[test]
fn resize_canvas_matches_imagemagick() {
    if let Err(skip) = oracle::require(oracle::Tool::Magick) {
        skip.report();
        return;
    }
    let src = test_image_buffer(16, 16);
    let anchors = [
        (Anchor::TopLeft, "northwest"),
        (Anchor::TopCenter, "north"),
        (Anchor::TopRight, "northeast"),
        (Anchor::MiddleLeft, "west"),
        (Anchor::Center, "center"),
        (Anchor::MiddleRight, "east"),
        (Anchor::BottomLeft, "southwest"),
        (Anchor::BottomCenter, "south"),
        (Anchor::BottomRight, "southeast"),
    ];
    for (anchor, gravity) in anchors {
        for (w, h) in [(20u32, 18u32), (12, 10)] {
            let out = resize_canvas(&src, w, h, anchor, [10, 20, 30, 255]).expect("resize_canvas");
            let width = w.to_string();
            let height = h.to_string();
            let label = format!("resize_canvas {anchor:?} {w}x{h}");
            differential_exact(
                &label,
                &out,
                &[
                    "--op",
                    "resize_canvas",
                    "--width",
                    &width,
                    "--height",
                    &height,
                    "--gravity",
                    gravity,
                    "--background",
                    "10,20,30,255",
                ],
                &src.data,
                "16x16",
                0,
            );
        }
    }
}

#[test]
fn rotate90_cw_matches_imagemagick() {
    let src = test_image_buffer(16, 12);
    let out = rotate90_cw(&src);
    differential_exact(
        "rotate90_cw",
        &out,
        &["--op", "rotate90_cw"],
        &src.data,
        "16x12",
        0,
    );
}

#[test]
fn rotate90_ccw_matches_imagemagick() {
    let src = test_image_buffer(16, 12);
    let out = rotate90_ccw(&src);
    differential_exact(
        "rotate90_ccw",
        &out,
        &["--op", "rotate90_ccw"],
        &src.data,
        "16x12",
        0,
    );
}

#[test]
fn rotate180_matches_imagemagick() {
    let src = test_image_buffer(16, 12);
    let out = rotate180(&src);
    differential_exact(
        "rotate180",
        &out,
        &["--op", "rotate180"],
        &src.data,
        "16x12",
        0,
    );
}

#[test]
fn flip_horizontal_matches_imagemagick() {
    let src = test_image_buffer(16, 12);
    let out = flip_horizontal(&src);
    differential_exact(
        "flip_horizontal",
        &out,
        &["--op", "flip_horizontal"],
        &src.data,
        "16x12",
        0,
    );
}

#[test]
fn flip_vertical_matches_imagemagick() {
    let src = test_image_buffer(16, 12);
    let out = flip_vertical(&src);
    differential_exact(
        "flip_vertical",
        &out,
        &["--op", "flip_vertical"],
        &src.data,
        "16x12",
        0,
    );
}

/// ImageMagick's `-rotate` pads the rotated bounding box by one row/column per
/// side (24x24 vs Pictura's 22x22 at 30 deg) and its edge sampler diverges from
/// Pictura's bilinear map. Compare only the central 12x12, where the rotated
/// source is intact; the measured max delta is 8 (mean 0.6).
#[test]
fn rotate_arbitrary_matches_imagemagick() {
    if let Err(skip) = oracle::require(oracle::Tool::Magick) {
        skip.report();
        return;
    }
    let src = test_image_buffer(16, 16);
    let out = rotate_arbitrary(&src, 30.0, [0, 0, 0, 0]).expect("rotate_arbitrary");
    let (reference, (iw, ih)) = oracle_at(
        &[
            "--op",
            "rotate_arbitrary",
            "--angle",
            "30",
            "--filter",
            "triangle",
            "--background",
            "0,0,0,0",
        ],
        &src.data,
        "16x16",
        3,
    );
    assert!(
        iw >= out.width && ih >= out.height,
        "rotate_arbitrary: IM {iw}x{ih} smaller than Pictura {}x{}",
        out.width,
        out.height
    );
    const K: u32 = 12;
    let want = center_crop(&out.data, out.width, out.height, out.channels, K);
    let got = center_crop(&reference, iw, ih, 3, K);
    let diff = compare(&want, &got, 8).expect("buffer lengths agree");
    assert!(
        diff.is_empty(),
        "rotate_arbitrary 30 deg (central {K}x{K}): {} of {} samples over tolerance 8 (max delta {}); \
         IM padded to {iw}x{ih} vs Pictura {}x{}",
        diff.differing,
        diff.samples,
        diff.max_delta,
        out.width,
        out.height
    );
}

/// The differential harness must catch a wrong op output: apply an exact op,
/// check it matches, perturb a scratch copy and check the comparison fails,
/// then revert and confirm it matches again.
#[test]
fn differential_harness_detects_perturbation() {
    if let Err(skip) = oracle::require(oracle::Tool::Magick) {
        skip.report();
        return;
    }
    let src = test_image_buffer(16, 12);
    let out = flip_horizontal(&src);
    let (reference, dims) = oracle_at(&["--op", "flip_horizontal"], &src.data, "16x12", 3);
    assert_eq!(dims, (16, 12));
    assert!(
        compare(&out.data, &reference, 0)
            .expect("lengths")
            .is_empty(),
        "sanity: flip_horizontal must match the oracle before perturbation"
    );

    let mut perturbed = out.clone();
    perturbed.data[0] = perturbed.data[0].wrapping_add(37);
    assert!(
        !compare(&perturbed.data, &reference, 0)
            .expect("lengths")
            .is_empty(),
        "a perturbed op output must not compare equal"
    );

    perturbed.data[0] = perturbed.data[0].wrapping_sub(37);
    assert!(
        compare(&perturbed.data, &reference, 0)
            .expect("lengths")
            .is_empty(),
        "reverting the scratch copy must restore the match"
    );
}
