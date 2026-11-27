//! The oracle harness itself: script presence, ImageMagick availability, the
//! planar round-trip, and the perturbation guard.

use pictura_core::PixelBuffer;
use pictura_filters::{apply, Filter};
use pictura_testkit::{compare, oracle};

use crate::common::{oracle, script, test_image_planar, CHANNELS, SIZE};

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
        .expect("run filter_oracle.py apply --help");
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
        .expect("run filter_oracle.py version");
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
    if let Err(skip) = oracle::require(oracle::Tool::Magick) {
        skip.report();
        return;
    }
    let input = test_image_planar();
    let output = oracle(&["--im-args=-negate"], &input);
    assert_eq!(output.len(), input.len());
    let expected: Vec<u8> = input.iter().map(|v| 255 - v).collect();
    assert_eq!(output, expected);
}

/// The differential harness must actually catch a wrong filter output. Apply a
/// faithful filter, check it matches, then perturb a scratch copy and check the
/// comparison fails; revert and confirm it matches again.
#[test]
fn differential_harness_detects_perturbation() {
    if let Err(skip) = oracle::require(oracle::Tool::Magick) {
        skip.report();
        return;
    }
    let original = test_image_planar();
    let reference = oracle(
        &["--op", "solarize", "--threshold-percent", "50"],
        &original,
    );
    let mut buf = PixelBuffer {
        width: SIZE,
        height: SIZE,
        channels: CHANNELS,
        data: original.into(),
    };
    apply(&Filter::Solarize, &mut buf).expect("apply");
    assert!(
        compare(&buf.data, &reference, 0)
            .expect("lengths")
            .is_empty(),
        "sanity: Solarize must match the oracle before perturbation"
    );

    let mut perturbed = buf.clone();
    perturbed.data[0] = perturbed.data[0].wrapping_add(37);
    assert!(
        !compare(&perturbed.data, &reference, 0)
            .expect("lengths")
            .is_empty(),
        "a perturbed filter output must not compare equal"
    );

    perturbed.data[0] = perturbed.data[0].wrapping_sub(37);
    assert!(
        compare(&perturbed.data, &reference, 0)
            .expect("lengths")
            .is_empty(),
        "reverting the scratch copy must restore the match"
    );
}
