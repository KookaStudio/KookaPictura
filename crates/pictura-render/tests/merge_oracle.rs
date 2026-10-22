//! External-oracle check for `pictura_render::merge_scope` (Merge Down).
//!
//! The committed `tests/fixtures/alpha_*.rgba` files are raw interleaved RGBA8
//! 8x8 layers produced by ImageMagick (`scripts/im_compose.py`), an independent
//! implementation of source-over compositing. A Merge Down of the same two
//! layers must reproduce the ImageMagick composite. The live variant regenerates
//! the reference with `magick` and self-skips when the tool is absent.
//!
//! The blend-mode variants merge the same pair with a source layer that
//! carries a blend mode, and diff the result against the committed
//! `<scene>_<Mode>.rgba` references: the merge composites the source through
//! the document compositor, so the source's own blend mode governs it and the
//! merged pixels must match ImageMagick's operator for that mode.

use std::path::PathBuf;
use std::process::Command;

use pictura_core::{
    BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LockFlags, PsdRect,
};
use pictura_render::{merge_scope, MergeScope};

const SIZE: u32 = 8;

/// The modes ImageMagick implements with the same formula as the W3C
/// reference: `(PSD mode, reference suffix, tolerance)`. Mirrors the table in
/// `scripts/im_compose.py` and in `tests/oracle.rs`; the eight modes with no
/// usable ImageMagick equivalent are covered by hand-computed W3C unit tests
/// in `pictura-render` instead.
const MODES: &[(BlendMode, &str, u8)] = &[
    (BlendMode::Normal, "Normal", 0),
    (BlendMode::Darken, "Darken", 0),
    (BlendMode::Multiply, "Multiply", 0),
    (BlendMode::ColorBurn, "ColorBurn", 0),
    (BlendMode::LinearBurn, "LinearBurn", 0),
    (BlendMode::Lighten, "Lighten", 0),
    (BlendMode::Screen, "Screen", 0),
    (BlendMode::ColorDodge, "ColorDodge", 0),
    (BlendMode::LinearDodge, "LinearDodge", 0),
    (BlendMode::Overlay, "Overlay", 0),
    (BlendMode::HardLight, "HardLight", 0),
    (BlendMode::VividLight, "VividLight", 1),
    (BlendMode::LinearLight, "LinearLight", 0),
    (BlendMode::PinLight, "PinLight", 0),
    (BlendMode::HardMix, "HardMix", 0),
    (BlendMode::Difference, "Difference", 0),
    (BlendMode::Exclusion, "Exclusion", 0),
    (BlendMode::Subtract, "Subtract", 0),
    (BlendMode::Divide, "Divide", 0),
];

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/im_compose.py")
}

fn read_fixture(name: &str) -> Vec<u8> {
    let path = fixtures_dir().join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}); run `python3 scripts/im_compose.py gen`",
            path.display()
        )
    });
    assert_eq!(
        bytes.len(),
        (SIZE * SIZE * 4) as usize,
        "{name}: raw RGBA8 length"
    );
    bytes
}

/// Split interleaved RGBA8 into planar per-channel vectors.
fn planar(interleaved: &[u8]) -> [Vec<u8>; 4] {
    let pixels = (SIZE * SIZE) as usize;
    let mut planes = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for i in 0..pixels {
        for (c, plane) in planes.iter_mut().enumerate() {
            plane.push(interleaved[i * 4 + c]);
        }
    }
    planes
}

fn layer(name: &str, blend: BlendMode, interleaved: &[u8], with_alpha: bool) -> Layer {
    let [r, g, b, a] = planar(interleaved);
    let mut channels = vec![
        Channel {
            id: 0,
            data: r.into(),
        },
        Channel {
            id: 1,
            data: g.into(),
        },
        Channel {
            id: 2,
            data: b.into(),
        },
    ];
    if with_alpha {
        channels.push(Channel {
            id: -1,
            data: a.into(),
        });
    }
    Layer {
        name: name.into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: SIZE as i32,
            right: SIZE as i32,
        },
        blend,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels,
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

/// A uniform opaque layer of one colour.
fn uniform(name: &str, blend: BlendMode, colour: (u8, u8, u8)) -> Layer {
    let mut bytes = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for _ in 0..SIZE * SIZE {
        bytes.extend_from_slice(&[colour.0, colour.1, colour.2, 255]);
    }
    layer(name, blend, &bytes, true)
}

/// A layer's channels as interleaved RGBA8.
fn interleaved(layer: &Layer) -> Vec<u8> {
    let n = (layer.rect.width() * layer.rect.height()) as usize;
    let plane = |id: i16| layer.channels.iter().find(|c| c.id == id).unwrap();
    let (r, g, b, a) = (plane(0), plane(1), plane(2), plane(-1));
    let mut out = Vec::with_capacity(n * 4);
    for i in 0..n {
        out.extend_from_slice(&[r.data[i], g.data[i], b.data[i], a.data[i]]);
    }
    out
}

/// Merge Down a base and a source layer and return the merged node as
/// interleaved RGBA8. The base carries no alpha channel, so it is opaque.
fn merged_pair(base: &[u8], src: &[u8], mode: BlendMode) -> Vec<u8> {
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        layer("base", BlendMode::Normal, base, false),
        layer("src", mode, src, true),
    ]);
    merge_scope(&mut doc, MergeScope::Down("1")).expect("merge down");
    interleaved(&doc.layers[0])
}

#[test]
fn merge_down_matches_committed_imagemagick_fixture() {
    let reference = read_fixture("alpha_Normal.rgba");
    let base = read_fixture("alpha_base.rgba");
    let src = read_fixture("alpha_src.rgba");
    let diff =
        pictura_testkit::compare(&merged_pair(&base, &src, BlendMode::Normal), &reference, 0)
            .expect("equal-length RGBA buffers");
    assert!(
        diff.is_empty(),
        "merged pair diverged from ImageMagick: {} samples, max delta {}",
        diff.differing,
        diff.max_delta
    );
}

/// A Merge Down of a source layer with a non-`Normal` blend mode must equal
/// ImageMagick's composite for that mode: the merge composites the source
/// through the document compositor, so the source's own blend mode governs it.
#[test]
fn merge_down_matches_imagemagick_for_every_supported_mode() {
    for scene in ["solid", "ramp"] {
        let base = read_fixture(&format!("{scene}_base.rgba"));
        let src = read_fixture(&format!("{scene}_src.rgba"));
        for (mode, suffix, tolerance) in MODES {
            let reference = read_fixture(&format!("{scene}_{suffix}.rgba"));
            let actual = merged_pair(&base, &src, *mode);
            let diff = pictura_testkit::compare(&actual, &reference, *tolerance)
                .expect("equal-length RGBA buffers");
            assert!(
                diff.is_empty(),
                "{scene}/{suffix}: {} samples over tolerance {tolerance} \
                 (max delta {})",
                diff.differing,
                diff.max_delta
            );
        }
    }
}

/// The source's blend mode is baked into the merged pixels, not carried onto
/// the merged node: `Multiply((40, 60, 80), (120, 130, 140))` is
/// `(19, 31, 44)`, and the merged node keeps the *target's* blend mode and
/// opacity. The target's blend is `Screen` so the assertion cannot pass by
/// accident: a node that hardcoded `Normal` would fail it.
#[test]
fn merge_down_bakes_the_sources_blend_into_the_pixels() {
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        uniform("target", BlendMode::Screen, (40, 60, 80)),
        uniform("source", BlendMode::Multiply, (120, 130, 140)),
    ]);
    merge_scope(&mut doc, MergeScope::Down("1")).expect("merge down");
    let node = &doc.layers[0];
    assert_eq!(node.blend, BlendMode::Screen, "the target's blend mode");
    assert_eq!(node.opacity, 255, "the target's opacity");
    let expected: Vec<u8> = (0..SIZE * SIZE).flat_map(|_| [19, 31, 44, 255]).collect();
    assert_eq!(
        interleaved(node),
        expected,
        "the source's Multiply is baked into the pixels"
    );
}

#[test]
fn merge_down_matches_live_imagemagick() {
    if Command::new("magick").arg("-version").output().is_err() {
        eprintln!("skipping live ImageMagick merge oracle: `magick` not on PATH");
        return;
    }

    // The `alpha` scene checks source-over with a partial-alpha source; the
    // `ramp` scene is fully opaque, which is what ImageMagick's blend
    // operators agree with.
    for (scene, operator, mode) in [
        ("alpha", "Over", BlendMode::Normal),
        ("ramp", "Multiply", BlendMode::Multiply),
    ] {
        let dir = std::env::temp_dir().join(format!("pictura-merge-oracle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join(format!("{scene}_{operator}.rgba"));

        let status = Command::new("python3")
            .arg(script())
            .arg("compose")
            .arg(operator)
            .arg(fixtures_dir().join(format!("{scene}_base.rgba")))
            .arg(fixtures_dir().join(format!("{scene}_src.rgba")))
            .arg(&out)
            .status()
            .expect("run scripts/im_compose.py compose");
        assert!(status.success(), "im_compose.py compose failed");

        let reference = std::fs::read(&out).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        let base = read_fixture(&format!("{scene}_base.rgba"));
        let src = read_fixture(&format!("{scene}_src.rgba"));
        let diff = pictura_testkit::compare(&merged_pair(&base, &src, mode), &reference, 0)
            .expect("equal-length RGBA buffers");
        assert!(
            diff.is_empty(),
            "{scene}/{operator}: {} samples differ (max delta {})",
            diff.differing,
            diff.max_delta
        );
    }
}
