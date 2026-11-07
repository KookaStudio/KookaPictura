//! ImageMagick differential oracle for `pictura_adjust::apply` (task M4-C).
//!
//! ImageMagick implements a handful of the same adjustments. This is a *sanity*
//! oracle, not a parity oracle: the exact integer math is closed and the
//! ImageMagick operators only approximate some reference paths. Each supported
//! adjustment is diffed against the ImageMagick result with
//! `pictura_testkit::compare`; the tolerance and the reason for it are in the
//! table below and in `tests/README.md`.
//!
//! As of task M4-C, only adjustments with a faithful ImageMagick operator are
//! run differentially (`Levels`, `Invert`, `Desaturate`). The rest are covered
//! by ImageMagick-independent property/known-value tests; the divergences that
//! ruled out a differential test are recorded in the table below and in
//! `tests/README.md`.
//!
//! Regenerate/inspect a result manually with `scripts/adjust_oracle.py`; see
//! `tests/README.md`.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use pictura_adjust::{
    apply, Adjustment, BrightnessContrastParams, ChannelMixerParams, GradientMapParams,
    GradientStop, HueSaturationParams, LevelsParams,
};
use pictura_core::PixelBuffer;
use pictura_testkit::compare;

/// Side length of the raw planar RGB8 test image.
const SIZE: u32 = 8;
const CHANNELS: u8 = 3;

/// One row of the `Adjustment` -> ImageMagick mapping table. `im` is the
/// equivalent operator (documentation; the actual invocation lives with each
/// test) or `None` when there is no faithful equivalent.
struct Mapping {
    adjustment: &'static str,
    im: Option<&'static str>,
    tolerance: u8,
    note: &'static str,
}

/// One row per classified `Adjustment` variant: the eighteen destructive
/// variants plus the two generative fills the spec names (`GradientFill`,
/// `PatternFill`).
/// `SolidFill` is the third refused fill; it has no `apply`, so it has no
/// differential/classification row. Its refusal is covered by
/// `apply_refuses_solid_fill_without_mutating` and the alpha-preservation test.
const MAPPING: &[Mapping] = &[
    Mapping {
        adjustment: "Levels",
        im: Some("-level B%,W%,g +level Ob%,Ow%"),
        tolerance: 1,
        note: "same input-then-output remap; only rounding differs",
    },
    Mapping {
        adjustment: "Curves",
        im: None,
        tolerance: 0,
        note: "arbitrary control-point curve; no IM operator",
    },
    Mapping {
        adjustment: "BrightnessContrast",
        im: None,
        tolerance: 0,
        note: "IM -brightness-contrast algorithm differs (observed max delta 14)",
    },
    Mapping {
        adjustment: "Exposure",
        im: None,
        tolerance: 0,
        note: "PS applies in linear light; IM -evaluate/-gamma run encoded",
    },
    Mapping {
        adjustment: "HueSaturation",
        im: None,
        tolerance: 0,
        note: "IM -modulate uses its own HSL space, not PS HSL (observed max delta 45)",
    },
    Mapping {
        adjustment: "BlackWhite",
        im: None,
        tolerance: 0,
        note: "hue-sector decomposition; no IM operator",
    },
    Mapping {
        adjustment: "PhotoFilter",
        im: None,
        tolerance: 0,
        note: "no faithful IM operator",
    },
    Mapping {
        adjustment: "GradientMap",
        im: None,
        tolerance: 0,
        note: "no faithful IM operator",
    },
    Mapping {
        adjustment: "GradientFill",
        im: None,
        tolerance: 0,
        note: "generative fill composited over the layer rect; no IM operator",
    },
    Mapping {
        adjustment: "PatternFill",
        im: None,
        tolerance: 0,
        note: "generative tiled fill composited over the layer rect; no IM operator",
    },
    Mapping {
        adjustment: "ChannelMixer",
        im: None,
        tolerance: 0,
        note: "IM -color-matrix takes fractions, PS uses percent weights (observed max delta 252)",
    },
    Mapping {
        adjustment: "Vibrance",
        im: None,
        tolerance: 0,
        note: "no faithful IM operator",
    },
    Mapping {
        adjustment: "ColorBalance",
        im: None,
        tolerance: 0,
        note: "no faithful IM operator",
    },
    Mapping {
        adjustment: "SelectiveColor",
        im: None,
        tolerance: 0,
        note: "no faithful IM operator; profile-free integer CMYK round-trip",
    },
    Mapping {
        adjustment: "Auto",
        im: None,
        tolerance: 0,
        note: "closed Auto Color Correction solver",
    },
    Mapping {
        adjustment: "Invert",
        im: Some("-negate"),
        tolerance: 0,
        note: "identical M - v per channel",
    },
    Mapping {
        adjustment: "Posterize",
        im: None,
        tolerance: 0,
        note: "IM -posterize bins on an adjacent level (observed max delta 85)",
    },
    Mapping {
        adjustment: "Threshold",
        im: None,
        tolerance: 0,
        note: "IM -threshold uses Rec.709 luma; PS uses Rec.601 (observed max delta 255)",
    },
    Mapping {
        adjustment: "Desaturate",
        im: Some("-modulate 100,0,100"),
        tolerance: 1,
        note: "IM modulate here computes HSL (min+max)/2, matching PS",
    },
    Mapping {
        adjustment: "ColorLookup",
        im: None,
        tolerance: 0,
        note: "arbitrary .cube trilinear sample; no IM operator",
    },
];

/// Adjustments the table marks as having no faithful ImageMagick equivalent.
const NO_EQUIVALENT: [&str; 17] = [
    "BlackWhite",
    "PhotoFilter",
    "GradientMap",
    "GradientFill",
    "PatternFill",
    "Vibrance",
    "ColorBalance",
    "SelectiveColor",
    "ColorLookup",
    "Auto",
    "Curves",
    "Exposure",
    "BrightnessContrast",
    "HueSaturation",
    "ChannelMixer",
    "Posterize",
    "Threshold",
];

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/adjust_oracle.py")
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

/// Unique scratch directory per call so tests can run in parallel.
fn scratch_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "pictura-adjust-oracle-{}-{tag}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn test_image_interleaved() -> Vec<u8> {
    const SPECIAL: [(u8, u8, u8); 8] = [
        (0, 0, 0),
        (255, 255, 255),
        (255, 0, 0),
        (0, 255, 0),
        (0, 0, 255),
        (128, 128, 128),
        (64, 128, 192),
        (200, 100, 50),
    ];
    let mut data = Vec::with_capacity((SIZE * SIZE * 3) as usize);
    for i in 0..(SIZE * SIZE) as usize {
        let (r, g, b) = if let Some(&(r, g, b)) = SPECIAL.get(i) {
            (r, g, b)
        } else {
            let (x, y) = ((i as u32) % SIZE, (i as u32) / SIZE);
            (
                ((x * 36) % 256) as u8,
                ((y * 36) % 256) as u8,
                (((x + y) * 18) % 256) as u8,
            )
        };
        data.extend_from_slice(&[r, g, b]);
    }
    data
}

/// Interleaved RGB8 -> planar RGB (the `PixelBuffer` layout `apply` consumes).
fn planarize(interleaved: &[u8]) -> Vec<u8> {
    let pixels = (SIZE * SIZE) as usize;
    let mut planar = vec![0u8; interleaved.len()];
    for i in 0..pixels {
        for c in 0..CHANNELS as usize {
            planar[c * pixels + i] = interleaved[i * CHANNELS as usize + c];
        }
    }
    planar
}

fn test_image_planar() -> Vec<u8> {
    planarize(&test_image_interleaved())
}

/// The `SIZE`x`SIZE` test image as a `PixelBuffer`.
fn test_image_buffer() -> PixelBuffer {
    PixelBuffer {
        width: SIZE,
        height: SIZE,
        channels: CHANNELS,
        data: test_image_planar().into(),
    }
}

/// Build a planar 3-channel buffer from interleaved pixels (`w = len`, `h = 1`).
fn buffer_of(px: &[[u8; 3]]) -> PixelBuffer {
    let n = px.len();
    let mut data = vec![0u8; n * 3];
    for (i, p) in px.iter().enumerate() {
        data[i] = p[0];
        data[n + i] = p[1];
        data[2 * n + i] = p[2];
    }
    PixelBuffer {
        width: n as u32,
        height: 1,
        channels: CHANNELS,
        data: data.into(),
    }
}

/// `steps` neutral greys spanning 0..=255.
fn gray_ramp(steps: u32) -> PixelBuffer {
    let px: Vec<[u8; 3]> = (0..steps)
        .map(|i| {
            let v = (i as f64 * 255.0 / (steps - 1) as f64).round() as u8;
            [v, v, v]
        })
        .collect();
    buffer_of(&px)
}

fn color_at(buf: &PixelBuffer, i: usize) -> [u8; 3] {
    let n = buf.pixel_count();
    [buf.data[i], buf.data[n + i], buf.data[2 * n + i]]
}

fn apply_owned(adjustment: &Adjustment, mut buf: PixelBuffer) -> PixelBuffer {
    apply(adjustment, &mut buf).expect("pictura_adjust::apply");
    buf
}

/// Run `scripts/adjust_oracle.py apply` over the planar test image and return
/// the raw planar result. `extra` holds the operator arguments.
fn oracle(extra: &[&str], input_planar: &[u8]) -> Vec<u8> {
    let dir = scratch_dir("oracle");
    let input_path = dir.join("in.rgb");
    let output_path = dir.join("out.rgb");
    std::fs::write(&input_path, input_planar).unwrap();
    let result = Command::new("python3")
        .arg(script())
        .arg("apply")
        .args(["--size", "8x8", "--planar"])
        .args(extra)
        .arg(&input_path)
        .arg(&output_path)
        .output()
        .expect("run adjust_oracle.py apply");
    assert!(
        result.status.success(),
        "oracle failed:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = std::fs::read(&output_path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    bytes
}

/// Diff `apply` against the ImageMagick oracle. Skips when `magick` is absent.
fn differential(adjustment: Adjustment, extra: &[&str], tolerance: u8, label: &str) {
    if !magick_available() {
        eprintln!("skipping {label}: `magick` not on PATH");
        return;
    }
    let original = test_image_planar();
    let reference = oracle(extra, &original);
    let mut buf = PixelBuffer {
        width: SIZE,
        height: SIZE,
        channels: CHANNELS,
        data: original.into(),
    };
    apply(&adjustment, &mut buf).expect("pictura_adjust::apply");
    let diff = compare(&buf.data, &reference, tolerance).unwrap_or_else(|e| panic!("{label}: {e}"));
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
    let help = Command::new("python3")
        .arg(&path)
        .arg("apply")
        .arg("--help")
        .output()
        .expect("run adjust_oracle.py apply --help");
    assert!(
        help.status.success(),
        "`apply --help` failed:\n{}",
        String::from_utf8_lossy(&help.stderr)
    );
}

#[test]
fn imagemagick_runs() {
    if !magick_available() {
        eprintln!("skipping: `magick` not on PATH");
        return;
    }
    let out = Command::new("python3")
        .arg(script())
        .arg("version")
        .output()
        .expect("run adjust_oracle.py version");
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

/// The oracle itself, independent of `apply`: `-negate` must produce `M - v`
/// for every planar sample (this also proves the planar round-trip).
#[test]
fn oracle_negate_matches_expected_bytes() {
    if !magick_available() {
        eprintln!("skipping: `magick` not on PATH");
        return;
    }
    let input = test_image_planar();
    let output = oracle(&["--im-args=-negate"], &input);
    assert_eq!(output.len(), input.len());
    let expected: Vec<u8> = input.iter().map(|v| 255 - v).collect();
    assert_eq!(output, expected);
}

#[test]
fn mapping_marks_no_equivalent_operators() {
    assert_eq!(
        MAPPING.len(),
        20,
        "one row per classified variant (18 destructive + GradientFill + PatternFill)"
    );
    let none: Vec<&str> = MAPPING
        .iter()
        .filter(|m| m.im.is_none())
        .map(|m| m.adjustment)
        .collect();
    let mut expected = NO_EQUIVALENT.to_vec();
    expected.sort_unstable();
    let mut none = none;
    none.sort_unstable();
    assert_eq!(none, expected);
    for m in MAPPING {
        assert!(!m.note.is_empty(), "{}: empty note", m.adjustment);
        if m.im.is_none() {
            assert_eq!(
                m.tolerance, 0,
                "{}: no equivalent must not set tolerance",
                m.adjustment
            );
        } else {
            assert!(m.tolerance <= 8, "{}: tolerance out of range", m.adjustment);
        }
    }
}

#[test]
fn levels_matches_imagemagick() {
    differential(
        Adjustment::Levels(LevelsParams {
            input_black: 0,
            input_white: 255,
            gamma: 2.0,
            output_black: 0,
            output_white: 255,
            red: None,
            green: None,
            blue: None,
        }),
        &[
            "--op",
            "levels",
            "--black",
            "0",
            "--white",
            "255",
            "--gamma",
            "2.0",
            "--out-black",
            "0",
            "--out-white",
            "255",
        ],
        1,
        "Levels gamma 2.0",
    );
}

/// IM has no faithful BrightnessContrast equivalent (algorithm diverges, max
/// delta 14). Guard the PS behavior directly instead.
#[test]
fn brightness_contrast_legacy_properties() {
    let neutral = BrightnessContrastParams {
        brightness: 0,
        contrast: 0,
        use_legacy: true,
    };
    let original = test_image_buffer();
    let out = apply_owned(&Adjustment::BrightnessContrast(neutral), original.clone());
    assert_eq!(out.data, original.data, "neutral params must be identity");

    let ramp = gray_ramp(256);
    let lo = apply_owned(
        &Adjustment::BrightnessContrast(BrightnessContrastParams {
            brightness: 0,
            contrast: 0,
            use_legacy: true,
        }),
        ramp.clone(),
    );
    let hi = apply_owned(
        &Adjustment::BrightnessContrast(BrightnessContrastParams {
            brightness: 40,
            contrast: 0,
            use_legacy: true,
        }),
        ramp,
    );
    for i in 0..lo.pixel_count() {
        assert!(
            hi.data[i] >= lo.data[i],
            "brightness +40 must not darken sample {i} ({} < {})",
            hi.data[i],
            lo.data[i]
        );
    }

    let brightened = apply_owned(
        &Adjustment::BrightnessContrast(BrightnessContrastParams {
            brightness: 20,
            contrast: 0,
            use_legacy: true,
        }),
        original.clone(),
    );
    let n = original.pixel_count();
    let mean = |d: &[u8]| d.iter().map(|&v| v as f64).sum::<f64>() / n as f64;
    assert!(
        mean(&brightened.data) > mean(&original.data),
        "brightness +20 must raise the mean"
    );
}

/// IM `-modulate` uses a different HSL space (max delta 45). Guard PS behavior
/// with properties instead.
#[test]
fn hue_saturation_properties() {
    let original = test_image_buffer();
    let out = apply_owned(
        &Adjustment::HueSaturation(HueSaturationParams {
            hue: 0,
            saturation: 0,
            lightness: 0,
            ranges: Vec::new(),
        }),
        original.clone(),
    );
    assert_eq!(out.data, original.data, "0/0/0 must be identity");

    let n = original.pixel_count();
    let mean_spread = |buf: &PixelBuffer| {
        (0..n)
            .map(|i| {
                let p = color_at(buf, i);
                *p.iter().max().unwrap() as f64 - *p.iter().min().unwrap() as f64
            })
            .sum::<f64>()
            / n as f64
    };
    let saturated = apply_owned(
        &Adjustment::HueSaturation(HueSaturationParams {
            hue: 0,
            saturation: 50,
            lightness: 0,
            ranges: Vec::new(),
        }),
        original.clone(),
    );
    assert!(
        mean_spread(&saturated) > mean_spread(&original),
        "saturation +50 must increase mean channel spread ({} vs {})",
        mean_spread(&saturated),
        mean_spread(&original)
    );
}

/// IM `-color-matrix` takes fractions while PS uses percent weights (max delta
/// 252). Guard the PS contract directly instead.
#[test]
fn channel_mixer_identity_and_green_blend() {
    let original = test_image_buffer();
    let identity = apply_owned(
        &Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: false,
            red: [100.0, 0.0, 0.0],
            green: [0.0, 100.0, 0.0],
            blue: [0.0, 0.0, 100.0],
            constant: [0.0, 0.0, 0.0],
            ..Default::default()
        }),
        original.clone(),
    );
    assert_eq!(
        identity.data, original.data,
        "identity matrix must be a no-op"
    );

    let mut buf = buffer_of(&[[50, 100, 200], [255, 0, 0], [0, 255, 128]]);
    apply(
        &Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: false,
            red: [100.0, 0.0, 0.0],
            green: [0.0, 50.0, 50.0],
            blue: [0.0, 0.0, 100.0],
            constant: [0.0, 0.0, 0.0],
            ..Default::default()
        }),
        &mut buf,
    )
    .unwrap();
    assert_eq!(color_at(&buf, 0), [50, 150, 200]);
    assert_eq!(color_at(&buf, 1), [255, 0, 0]);
    assert_eq!(
        color_at(&buf, 2),
        [0, 192, 128],
        "0.5*255 + 0.5*128 rounds up"
    );
}

#[test]
fn invert_matches_imagemagick() {
    differential(Adjustment::Invert, &["--im-args=-negate"], 0, "Invert");
}

/// IM `-posterize` bins on an adjacent level (max delta 85). Guard the allowed
/// output set and the 8-bit identity instead.
#[test]
fn posterize_outputs_allowed_levels() {
    for levels in [2u8, 3, 4, 8] {
        let buf = apply_owned(&Adjustment::Posterize(levels), test_image_buffer());
        let allowed: Vec<u8> = (0..levels)
            .map(|q| (q as f64 * 255.0 / (levels as f64 - 1.0)).round() as u8)
            .collect();
        for (i, &v) in buf.data.iter().enumerate() {
            assert!(
                allowed.contains(&v),
                "posterize({levels}) emitted {v} at sample {i}, not in {allowed:?}"
            );
        }
    }

    let original = test_image_buffer();
    let out = apply_owned(&Adjustment::Posterize(255), original.clone());
    assert_eq!(out.data, original.data, "255 levels is the 8-bit identity");
}

/// IM `-threshold` uses Rec.709 luma while PS uses Rec.601 (max delta 255).
/// Guard binarization and monotonicity in the level instead.
#[test]
fn threshold_binary_and_monotone() {
    let buf = apply_owned(&Adjustment::Threshold(120), test_image_buffer());
    for (i, &v) in buf.data.iter().enumerate() {
        assert!(v == 0 || v == 255, "threshold emitted {v} at sample {i}");
    }

    let ramp = gray_ramp(256);
    let mut previous: Option<Vec<u8>> = None;
    for level in [1u8, 32, 64, 128, 192, 255] {
        let out = apply_owned(&Adjustment::Threshold(level), ramp.clone());
        if let Some(prev) = &previous {
            for (i, &p) in prev.iter().enumerate() {
                assert!(
                    !(p == 0 && out.data[i] == 255),
                    "black pixel {i} turned white when level rose to {level}"
                );
            }
        }
        previous = Some(out.data.to_vec());
    }
}

#[test]
fn desaturate_matches_imagemagick() {
    differential(
        Adjustment::Desaturate,
        &["--op", "desaturate"],
        1,
        "Desaturate",
    );
}

/// IM has no faithful Gradient Map operator. Guard the identity/reverse/clamp
/// contract directly instead.
#[test]
fn gradient_map_identity_reverse_and_clamp() {
    let bw = |reverse| {
        Adjustment::GradientMap(GradientMapParams {
            stops: vec![
                GradientStop {
                    location: 0,
                    color: [0, 0, 0],
                },
                GradientStop {
                    location: 4096,
                    color: [255, 255, 255],
                },
            ],
            reverse,
            transparency: Vec::new(),
        })
    };
    let ramp = gray_ramp(256);
    let identity = apply_owned(&bw(false), ramp.clone());
    for i in 0..ramp.pixel_count() {
        let d = identity.data[i] as i16 - ramp.data[i] as i16;
        assert!(d.abs() <= 1, "black-to-white ramp must be the identity");
    }

    let reversed = apply_owned(&bw(true), buffer_of(&[[0, 0, 0], [255, 255, 255]]));
    assert_eq!(color_at(&reversed, 0), [255, 255, 255]);
    assert_eq!(color_at(&reversed, 1), [0, 0, 0]);

    let clamped = apply_owned(
        &Adjustment::GradientMap(GradientMapParams {
            stops: vec![
                GradientStop {
                    location: 1024,
                    color: [0, 0, 255],
                },
                GradientStop {
                    location: 3072,
                    color: [255, 255, 0],
                },
            ],
            reverse: false,
            transparency: Vec::new(),
        }),
        buffer_of(&[[0, 0, 0], [255, 255, 255]]),
    );
    assert_eq!(
        color_at(&clamped, 0),
        [0, 0, 255],
        "below first stop clamps"
    );
    assert_eq!(
        color_at(&clamped, 1),
        [255, 255, 0],
        "above last stop clamps"
    );
}
