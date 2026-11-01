//! ImageMagick differential oracle for `pictura_render::composite_rgba`.
//!
//! The committed fixtures under `tests/fixtures/` are raw interleaved RGBA8
//! 8x8 layers produced by ImageMagick (`scripts/im_compose.py`), an independent
//! implementation of the SVG/W3C blend operators. This test rebuilds the same
//! two-layer `Document` in Rust and diffs the compositor against them.
//!
//! Only the 19 separable modes ImageMagick implements with the same formula are
//! checked here. The mode -> operator mapping, the unsupported modes and the
//! tolerances are documented in `tests/README.md`. Until task M2-A replaces the
//! `composite_rgba` stub, every test that calls it is `#[ignore]`d; enable them
//! once the compositor lands.
//!
//! Regenerate the references with `python3 scripts/im_compose.py gen`.

use std::path::PathBuf;
use std::process::Command;

use pictura_core::{
    BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer, LockFlags, PixelBuffer,
    PsdRect,
};
use pictura_testkit::compare;

const SIZE: u32 = 8;
const SCENES: [&str; 2] = ["solid", "ramp"];

/// Modes ImageMagick implements with the same formula as the W3C/reference
/// definition: `(PSD mode, reference suffix, tolerance)`. Mirrors the
/// `MAPPING` table in `scripts/im_compose.py`; the eight modes with no usable
/// ImageMagick equivalent are absent by design.
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

const NORMAL_ONLY: &[(BlendMode, &str, u8)] = &[(BlendMode::Normal, "Normal", 0)];

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/im_compose.py")
}

fn read_fixture(name: &str) -> Vec<u8> {
    let path = fixtures_dir().join(name);
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}); run `python3 scripts/im_compose.py gen`",
            path.display()
        )
    })
}

/// Split interleaved RGBA8 into planar per-channel vectors.
fn planar(interleaved: &[u8]) -> [Vec<u8>; 4] {
    let pixels = (SIZE * SIZE) as usize;
    let mut planes = [
        Vec::with_capacity(pixels),
        Vec::with_capacity(pixels),
        Vec::with_capacity(pixels),
        Vec::with_capacity(pixels),
    ];
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

/// Rebuild the scene as a document: opaque base below, `mode` source with alpha
/// above, matching the two layers ImageMagick composited.
fn scene(base: &[u8], src: &[u8], mode: BlendMode) -> Document {
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        layer("base", BlendMode::Normal, base, false),
        layer("src", mode, src, true),
    ]);
    doc
}

/// Flatten a planar RGBA `PixelBuffer` back to the interleaved layout IM wrote.
fn interleaved(buf: &PixelBuffer) -> Vec<u8> {
    let pixels = buf.pixel_count();
    assert_eq!(buf.channels, 4, "composite_rgba must return RGBA");
    let mut out = vec![0u8; pixels * 4];
    for i in 0..pixels {
        for c in 0..4usize {
            out[i * 4 + c] = buf.data[c * pixels + i];
        }
    }
    out
}

/// Every reference the mapping claims to support must be committed.
#[test]
fn fixtures_cover_all_supported_modes() {
    let expected = (SIZE * SIZE * 4) as usize;
    for scene_name in SCENES {
        for (_, suffix, _) in MODES {
            let name = format!("{scene_name}_{suffix}.rgba");
            let bytes = read_fixture(&name);
            assert_eq!(bytes.len(), expected, "{name}: raw RGBA8 length");
        }
    }
    for name in ["alpha_base.rgba", "alpha_src.rgba", "alpha_Normal.rgba"] {
        assert_eq!(
            read_fixture(name).len(),
            expected,
            "{name}: raw RGBA8 length"
        );
    }
}

/// The generator is committed and runnable.
#[test]
fn reference_script_is_present() {
    let path = script();
    assert!(path.is_file(), "missing generator at {}", path.display());
}

/// Regenerating the references reproduces the committed bytes exactly. Skipped
/// with a message when `magick`/`python3` are unavailable.
#[test]
fn imagemagick_fixtures_reproduce() {
    if Command::new("magick").arg("-version").output().is_err() {
        eprintln!("skipping: `magick` not on PATH");
        return;
    }
    let output = Command::new("python3")
        .arg(script())
        .arg("check")
        .output()
        .expect("run scripts/im_compose.py check");
    assert!(
        output.status.success(),
        "fixture check failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn solid_scene_matches_imagemagick() {
    compare_scene("solid", MODES);
}

#[test]
fn ramp_scene_matches_imagemagick() {
    compare_scene("ramp", MODES);
}

/// Partial source alpha: ImageMagick's blend operators only agree with the W3C
/// source-over mix for Normal, so this scene checks source-over alone.
#[test]
fn alpha_scene_normal_matches_imagemagick() {
    compare_scene("alpha", NORMAL_ONLY);
}

fn compare_scene(scene_name: &str, modes: &[(BlendMode, &str, u8)]) {
    let base = read_fixture(&format!("{scene_name}_base.rgba"));
    let src = read_fixture(&format!("{scene_name}_src.rgba"));
    for (mode, suffix, tolerance) in modes {
        let reference = read_fixture(&format!("{scene_name}_{suffix}.rgba"));
        let doc = scene(&base, &src, *mode);
        let actual = interleaved(&pictura_render::composite_rgba(&doc));
        let diff = compare(&actual, &reference, *tolerance)
            .unwrap_or_else(|e| panic!("{scene_name}/{suffix}: {e}"));
        assert!(
            diff.is_empty(),
            "{scene_name}/{suffix}: {} samples over tolerance {tolerance} \
             (max delta {}, mean {:.3})",
            diff.differing,
            diff.max_delta,
            diff.mean_delta()
        );
    }
}

/// Every fixture scene through the active backend on a device: the GPU result
/// must stay within one LSB of the CPU oracle (never of ImageMagick), or the
/// document must have declined to the CPU with identical bytes.
#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn fixture_scenes_match_oracle_on_active_backend() {
    assert!(
        pictura_render::gpu_available(),
        "no usable GPU device; this test is #[ignore]d and needs one (software-gpu in CI)"
    );
    let mut on_gpu = 0;
    for (scene_name, modes) in [("solid", MODES), ("ramp", MODES), ("alpha", NORMAL_ONLY)] {
        let base = read_fixture(&format!("{scene_name}_base.rgba"));
        let src = read_fixture(&format!("{scene_name}_src.rgba"));
        for (mode, suffix, _) in modes {
            let doc = scene(&base, &src, *mode);
            let cpu = pictura_render::composite_rgba(&doc);
            let (active, backend) = pictura_render::composite_active(&doc, true);
            match backend {
                pictura_render::Backend::Gpu => {
                    on_gpu += 1;
                    for (i, (a, b)) in cpu.data.iter().zip(active.data.iter()).enumerate() {
                        assert!(
                            a.abs_diff(*b) <= 1,
                            "{scene_name}/{suffix} byte {i}: cpu {a} vs gpu {b}"
                        );
                    }
                }
                pictura_render::Backend::Cpu => {
                    assert_eq!(active, cpu, "{scene_name}/{suffix}: declined bytes");
                }
            }
        }
    }
    assert!(on_gpu > 0, "a separable fixture scene must run on the GPU");
}

/// A fixture scene the GPU declines (Dissolve) falls back to the CPU oracle.
#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn declined_fixture_scene_equals_oracle() {
    assert!(
        pictura_render::gpu_available(),
        "no usable GPU device; this test is #[ignore]d and needs one (software-gpu in CI)"
    );
    let base = read_fixture("ramp_base.rgba");
    let src = read_fixture("ramp_src.rgba");
    let doc = scene(&base, &src, BlendMode::Dissolve);
    let (active, backend) = pictura_render::composite_active(&doc, true);
    assert_eq!(backend, pictura_render::Backend::Cpu);
    assert_eq!(active, pictura_render::composite_rgba(&doc));
}
