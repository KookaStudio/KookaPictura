use super::*;
use crate::AdjustError;

fn identity() -> ExposureGamma {
    ExposureGamma {
        exposure_ev: 0.0,
        gamma: 1.0,
    }
}

fn non_uniform_rgb() -> Vec<f32> {
    // Planar: one R, G, and B plane of four pixels.
    vec![
        0.2, 1.6, 0.1, 2.0, // R
        0.4, 0.5, 0.3, 0.7, // G
        0.8, 0.25, 0.6, 0.3, // B
    ]
}

#[test]
fn identity_is_neutral() {
    let src = [0.0f32, 0.25, 0.5, 1.0, 2.0];
    let out = exposure_gamma(&src, identity()).unwrap();
    for (a, b) in src.iter().zip(&out) {
        assert!((a - b).abs() < 1e-6, "{a} -> {b}");
    }
}

#[test]
fn documented_formula_above_one_is_unclamped() {
    // (2.0 * 2^1)^(1/2) = 2.0
    let out = exposure_gamma(
        &[2.0],
        ExposureGamma {
            exposure_ev: 1.0,
            gamma: 2.0,
        },
    )
    .unwrap();
    assert!((out[0] - 2.0).abs() < 1e-5, "got {}", out[0]);
}

#[test]
fn zero_and_negative_are_defined() {
    let out = exposure_gamma(
        &[0.0, -0.5, -2.0],
        ExposureGamma {
            exposure_ev: 0.0,
            gamma: 2.0,
        },
    )
    .unwrap();
    assert_eq!(out[0], 0.0);
    assert!(out[1] < 0.0 && out[1].is_finite(), "{:?}", out);
    assert!(out[2] < 0.0 && out[2].is_finite(), "{:?}", out);
}

#[test]
fn bad_parameters_are_rejected() {
    for gamma in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            exposure_gamma(
                &[1.0],
                ExposureGamma {
                    exposure_ev: 0.0,
                    gamma
                }
            ),
            Err(AdjustError::InvalidParams(_))
        ));
    }
    for exposure_ev in [f64::NAN, f64::INFINITY] {
        assert!(matches!(
            exposure_gamma(
                &[1.0],
                ExposureGamma {
                    exposure_ev,
                    gamma: 1.0
                }
            ),
            Err(AdjustError::InvalidParams(_))
        ));
    }
}

#[test]
fn each_method_changes_samples() {
    let src = non_uniform_rgb();
    let methods = [
        HdrToneMap::ExposureGamma(ExposureGamma {
            exposure_ev: 1.0,
            gamma: 1.0,
        }),
        HdrToneMap::EqualizeHistogram,
        HdrToneMap::HighlightCompression,
        HdrToneMap::LocalAdaptation(LocalAdaptationParams {
            radius: 2.0,
            ..Default::default()
        }),
    ];
    for method in methods {
        let out = tone_map(&src, 4, 1, 3, method).unwrap();
        assert_ne!(out, src, "{method:?} must change the samples");
        assert!(
            out.iter().all(|v| v.is_finite()),
            "{method:?} produced a non-finite sample"
        );
    }
}

#[test]
fn equalize_histogram_keeps_uniform_input_finite() {
    let src = vec![0.5f32; 12];
    let out = tone_map(&src, 4, 1, 3, HdrToneMap::EqualizeHistogram).unwrap();
    assert_eq!(out, src);
}

#[test]
fn highlight_compression_keeps_midtones_and_maps_white() {
    let out = tone_map(&[0.5, 4.0], 2, 1, 1, HdrToneMap::HighlightCompression).unwrap();
    assert!((out[0] - 0.5).abs() < 1e-6, "mid-tone changed: {}", out[0]);
    assert!(out[1] > 0.9 && out[1] < 1.0, "highlight: {}", out[1]);
}

#[test]
fn bad_local_adaptation_parameters_are_refused() {
    let src = non_uniform_rgb();
    let bad = [
        LocalAdaptationParams {
            radius: 0.0,
            ..Default::default()
        },
        LocalAdaptationParams {
            strength: f64::NAN,
            ..Default::default()
        },
    ];
    for params in bad {
        assert!(matches!(
            tone_map(&src, 4, 1, 3, HdrToneMap::LocalAdaptation(params)),
            Err(AdjustError::InvalidParams(_))
        ));
    }
}
