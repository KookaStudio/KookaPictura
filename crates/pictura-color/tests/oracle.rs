//! ImageMagick differential oracle for `pictura_color::convert` (task M3-B).
//!
//! ImageMagick links Little CMS 2, so `magick ... -profile src.icc -profile
//! dst.icc` is an independent check of our plumbing (channel order, stride,
//! bit depth, intent/BPC flags), not of the transform itself. The oracle lives
//! in `scripts/color_oracle.py`; the exact flags, tolerances and the measured
//! ImageMagick/lcms2 divergence are documented in `tests/README.md`.
//!
//! The differential tests build the profile bytes from the M3-A API
//! (`Profile::srgb`, `Profile::adobe_rgb`, `to_icc`). The smoke tests below
//! always run; the differential tests skip when ImageMagick is absent.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use pictura_testkit::oracle;

/// Side length of the raw RGBA8 test image used throughout.
const SIZE: u32 = 8;

/// Oracle script, relative to this crate.
fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/color_oracle.py")
}

/// Unique scratch directory per call so tests can run in parallel.
fn scratch_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "pictura-color-oracle-{}-{tag}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 8x8 interleaved RGBA8 image: pure primaries, white/black/gray, then a ramp.
fn test_image() -> Vec<u8> {
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
    let mut data = Vec::with_capacity((SIZE * SIZE * 4) as usize);
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
        data.extend_from_slice(&[r, g, b, 255]);
    }
    data
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
        .arg("--help")
        .output()
        .expect("run color_oracle.py --help");
    assert!(
        help.status.success(),
        "`--help` failed:\n{}",
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
        .expect("run color_oracle.py version");
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

/// Exercises the script's `convert` path end to end. Uses the distro ICC
/// profiles; skipped when they or ImageMagick are absent.
#[test]
fn oracle_script_converts_with_system_profiles() {
    const SRGB: &str = "/usr/share/color/icc/colord/sRGB.icc";
    const ADOBE: &str = "/usr/share/color/icc/colord/AdobeRGB1998.icc";
    if let Err(skip) = oracle::require(oracle::Tool::Magick) {
        skip.report();
        return;
    }
    if !PathBuf::from(SRGB).is_file() || !PathBuf::from(ADOBE).is_file() {
        eprintln!("skipping: system sRGB/AdobeRGB ICC profiles not installed");
        return;
    }
    let dir = scratch_dir("smoke");
    let input = dir.join("in.rgba");
    let output = dir.join("out.rgba");
    std::fs::write(&input, test_image()).unwrap();
    let result = oracle::Command::new("python3")
        .arg(script())
        .args([
            "convert", "--src", SRGB, "--dst", ADOBE, "--intent", "relative", "--bpc", "--size",
            "8x8",
        ])
        .arg(&input)
        .arg(&output)
        .output()
        .expect("run color_oracle.py convert");
    assert!(
        result.status.success(),
        "convert failed:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        std::fs::read(&output).unwrap().len(),
        (SIZE * SIZE * 4) as usize
    );
    let _ = std::fs::remove_dir_all(&dir);
}

mod differential {
    //! API-dependent oracle tests.

    use super::*;
    use pictura_color::{convert, Intent, Profile};
    use pictura_testkit::compare;
    use std::fs;

    /// Per-sample allowance. ImageMagick and lcms2 agreed byte-for-byte on the
    /// profiles tested (see tests/README.md); 3 leaves room for 16-bit
    /// intermediate rounding in other lcms2 builds.
    const TOLERANCE: u8 = 3;

    fn intent_arg(intent: Intent) -> &'static str {
        match intent {
            Intent::Perceptual => "perceptual",
            Intent::RelativeColorimetric => "relative",
            Intent::Saturation => "saturation",
            Intent::AbsoluteColorimetric => "absolute",
        }
    }

    /// Run the ImageMagick oracle for `input` and return the raw RGBA8 result.
    fn oracle(src_icc: &[u8], dst_icc: &[u8], input: &[u8], intent: Intent, bpc: bool) -> Vec<u8> {
        let dir = scratch_dir("differential");
        let src = dir.join("src.icc");
        let dst = dir.join("dst.icc");
        let input_path = dir.join("in.rgba");
        let output_path = dir.join("out.rgba");
        fs::write(&src, src_icc).unwrap();
        fs::write(&dst, dst_icc).unwrap();
        fs::write(&input_path, input).unwrap();
        let mut cmd = oracle::Command::new("python3");
        cmd.arg(script())
            .args(["convert", "--src"])
            .arg(&src)
            .arg("--dst")
            .arg(&dst)
            .args(["--intent", intent_arg(intent), "--size", "8x8"]);
        if bpc {
            cmd.arg("--bpc");
        }
        cmd.arg(&input_path).arg(&output_path);
        let result = cmd.output().expect("run color_oracle.py convert");
        assert!(
            result.status.success(),
            "oracle failed:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let bytes = fs::read(&output_path).unwrap();
        let _ = fs::remove_dir_all(&dir);
        bytes
    }

    /// Diff `convert` against the oracle and report the max delta.
    fn compare_with_oracle(src: &Profile, dst: &Profile, intent: Intent, bpc: bool, label: &str) {
        if let Err(skip) = oracle::require(oracle::Tool::Magick) {
            skip.report();
            return;
        }
        let input = test_image();
        let ours = convert(src, dst, &input, SIZE, SIZE, 4, 8, intent, bpc)
            .expect("pictura_color::convert");
        let reference = oracle(&src.to_icc(), &dst.to_icc(), &input, intent, bpc);
        let diff = compare(&ours, &reference, TOLERANCE).unwrap_or_else(|e| panic!("{label}: {e}"));
        assert!(
            diff.is_empty(),
            "{label}: {} of {} samples over tolerance {TOLERANCE} (max delta {})",
            diff.differing,
            diff.samples,
            diff.max_delta
        );
    }

    #[test]
    fn srgb_to_adobe_rgb_relative_bpc_matches_imagemagick() {
        compare_with_oracle(
            &Profile::srgb(),
            &Profile::adobe_rgb(),
            Intent::RelativeColorimetric,
            true,
            "sRGB->AdobeRGB relative+BPC",
        );
    }

    #[test]
    fn srgb_to_adobe_rgb_perceptual_matches_imagemagick() {
        compare_with_oracle(
            &Profile::srgb(),
            &Profile::adobe_rgb(),
            Intent::Perceptual,
            false,
            "sRGB->AdobeRGB perceptual",
        );
    }

    #[test]
    fn srgb_to_pro_photo_relative_bpc_matches_imagemagick() {
        compare_with_oracle(
            &Profile::srgb(),
            &Profile::pro_photo(),
            Intent::RelativeColorimetric,
            true,
            "sRGB->ProPhoto relative+BPC",
        );
    }

    /// Engine-independent ground truth: the sRGB primaries a correct
    /// sRGB->AdobeRGB relative-colorimetric conversion must produce. Derived
    /// from lcms2 2.19 (`transicc -t1`) and rounded, hence the tolerance.
    #[test]
    fn srgb_to_adobe_rgb_known_primaries() {
        let src = Profile::srgb();
        let dst = Profile::adobe_rgb();
        let colors: [(u8, u8, u8, [u8; 3]); 6] = [
            (0, 0, 0, [0, 0, 0]),
            (255, 255, 255, [255, 255, 255]),
            (255, 0, 0, [219, 2, 0]),
            (0, 255, 0, [144, 255, 60]),
            (0, 0, 255, [0, 2, 250]),
            (128, 128, 128, [127, 127, 127]),
        ];
        let mut input = Vec::new();
        let mut expected = Vec::new();
        for (r, g, b, want) in colors {
            input.extend_from_slice(&[r, g, b, 255]);
            expected.extend_from_slice(&want);
        }
        let got = convert(
            &src,
            &dst,
            &input,
            colors.len() as u32,
            1,
            4,
            8,
            Intent::RelativeColorimetric,
            true,
        )
        .expect("pictura_color::convert");
        for (i, chunk) in got.chunks(4).enumerate() {
            let want = &expected[i * 3..i * 3 + 3];
            let got = &chunk[..3];
            let delta = got
                .iter()
                .zip(want)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap_or(0);
            assert!(
                delta <= TOLERANCE,
                "pixel {i} ({got:?}) differs from known AdobeRGB value {want:?} by {delta}"
            );
        }
    }
}
