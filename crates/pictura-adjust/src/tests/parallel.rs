use super::*;

// --- Parallel vs serial -----------------------------------------------------

fn spread_buffer(w: u32, h: u32) -> PixelBuffer {
    let n = (w * h) as usize;
    let mut data = vec![0u8; n * 3];
    for (i, slot) in data.iter_mut().enumerate() {
        *slot = (i as u32).wrapping_mul(2_654_435_761) as u8;
    }
    PixelBuffer {
        width: w,
        height: h,
        channels: 3,
        data: data.into(),
    }
}

fn identity_cube() -> Lut3d {
    let mut points = Vec::with_capacity(8);
    for b in 0..2 {
        for g in 0..2 {
            for r in 0..2 {
                points.push([r as f32, g as f32, b as f32]);
            }
        }
    }
    Lut3d { size: 2, points }
}

fn every_kernel() -> Vec<(&'static str, Adjustment)> {
    let mut ranges = [SelectiveRange::default(); 9];
    ranges[0] = SelectiveRange {
        c: 12,
        m: -4,
        y: 0,
        k: 6,
    };
    vec![
        (
            "levels",
            Adjustment::Levels(LevelsParams {
                input_black: 10,
                input_white: 240,
                gamma: 1.3,
                output_black: 5,
                output_white: 250,
                red: Some(LevelsChannel {
                    input_black: 0,
                    input_white: 255,
                    gamma: 1.1,
                    output_black: 0,
                    output_white: 255,
                }),
                green: None,
                blue: None,
            }),
        ),
        (
            "curves",
            Adjustment::Curves(CurvesParams {
                points: vec![(0, 0), (128, 150), (255, 255)],
                red: None,
                green: None,
                blue: None,
            }),
        ),
        (
            "brightness_contrast",
            Adjustment::BrightnessContrast(BrightnessContrastParams {
                brightness: 20,
                contrast: 15,
                use_legacy: false,
            }),
        ),
        (
            "exposure",
            Adjustment::Exposure(ExposureParams {
                exposure: 0.7,
                offset: 0.02,
                gamma: 1.1,
            }),
        ),
        (
            "hue_saturation",
            Adjustment::HueSaturation(HueSaturationParams {
                hue: 25,
                saturation: 30,
                lightness: -10,
                ranges: Vec::new(),
            }),
        ),
        (
            "black_white",
            Adjustment::BlackWhite(BlackWhiteParams {
                red: 40.0,
                yellow: 60.0,
                green: 40.0,
                cyan: 60.0,
                blue: 20.0,
                magenta: 80.0,
                tint: true,
                tint_color: [120, 90, 40],
            }),
        ),
        (
            "photo_filter",
            Adjustment::PhotoFilter(PhotoFilterParams {
                color: [236, 138, 0],
                density: 25.0,
                preserve_luminosity: true,
            }),
        ),
        (
            "channel_mixer",
            Adjustment::ChannelMixer(ChannelMixerParams {
                monochrome: false,
                red: [110.0, -10.0, 0.0],
                green: [0.0, 100.0, 0.0],
                blue: [0.0, -10.0, 110.0],
                constant: [0.0, 0.0, 0.0],
            }),
        ),
        (
            "vibrance",
            Adjustment::Vibrance(VibranceParams {
                vibrance: 30,
                saturation: 10,
            }),
        ),
        (
            "shadows_highlights",
            Adjustment::ShadowsHighlights(ShadowsHighlightsParams {
                shadows_amount: 40.0,
                highlights_amount: 30.0,
            }),
        ),
        (
            "color_balance",
            Adjustment::ColorBalance(ColorBalanceParams {
                shadows: [10.0, 0.0, -10.0],
                midtones: [0.0, 5.0, 0.0],
                highlights: [-5.0, 0.0, 5.0],
                preserve_luminosity: true,
            }),
        ),
        (
            "selective_color",
            Adjustment::SelectiveColor(SelectiveColorParams {
                method: SelectiveColorMethod::Relative,
                ranges,
            }),
        ),
        ("auto_color", Adjustment::Auto(AutoKind::Color)),
        ("auto_contrast", Adjustment::Auto(AutoKind::Contrast)),
        ("invert", Adjustment::Invert),
        ("posterize", Adjustment::Posterize(4)),
        ("threshold", Adjustment::Threshold(120)),
        ("desaturate", Adjustment::Desaturate),
        ("equalize", Adjustment::Equalize),
        (
            "gradient_map",
            Adjustment::GradientMap(GradientMapParams {
                stops: vec![
                    GradientStop {
                        location: 0,
                        color: [0, 0, 0],
                    },
                    GradientStop {
                        location: 4096,
                        color: [255, 200, 80],
                    },
                ],
                reverse: false,
                transparency: Vec::new(),
            }),
        ),
        (
            "color_lookup",
            Adjustment::ColorLookup(ColorLookupParams {
                kind: ColorLookupKind::ThreeDLut,
                lookup: Some(identity_cube()),
            }),
        ),
        (
            "replace_color",
            Adjustment::ReplaceColor(ReplaceColorParams {
                samples: vec![ReplaceColorSample {
                    x: 10,
                    y: 10,
                    rgb: [128, 64, 32],
                }],
                fuzziness: 60.0,
                localized: false,
                hue: 20.0,
                saturation: 15.0,
                lightness: 5.0,
            }),
        ),
        (
            "match_color",
            Adjustment::MatchColor(MatchColorParams {
                target: MatchColorStats {
                    mean: [120.0, 110.0, 100.0],
                    std: [40.0, 35.0, 30.0],
                },
                source: MatchColorStats {
                    mean: [90.0, 100.0, 130.0],
                    std: [25.0, 30.0, 45.0],
                },
                neutralize: false,
                luminance: 100.0,
                color_intensity: 100.0,
                fade: 0.0,
            }),
        ),
    ]
}

#[test]
fn parallel_matches_serial() {
    use std::sync::atomic::Ordering;

    use crate::common::{FORCE_SERIAL, PARALLEL_RUNS};

    // Wider than one chunk (8192 pixels) so the parallel path splits.
    let source = spread_buffer(200, 60);
    for (label, adjustment) in every_kernel() {
        FORCE_SERIAL.store(true, Ordering::Relaxed);
        let mut serial = source.clone();
        apply(&adjustment, &mut serial).unwrap();
        FORCE_SERIAL.store(false, Ordering::Relaxed);

        let before = PARALLEL_RUNS.load(Ordering::Relaxed);
        let mut parallel = source.clone();
        apply(&adjustment, &mut parallel).unwrap();
        assert!(
            PARALLEL_RUNS.load(Ordering::Relaxed) > before,
            "{label} did not take the parallel path"
        );
        assert_eq!(serial, parallel, "{label}: parallel output differs");
    }
}

#[test]
#[ignore = "profiling: times each parallel kernel against its serial branch"]
fn parallel_speedup_profile() {
    use std::sync::atomic::Ordering;
    use std::time::Instant;

    use crate::common::FORCE_SERIAL;

    let source = spread_buffer(4000, 3000);
    for (label, adjustment) in every_kernel() {
        let mut buf = source.clone();
        FORCE_SERIAL.store(true, Ordering::Relaxed);
        let start = Instant::now();
        apply(&adjustment, &mut buf).unwrap();
        let serial = start.elapsed();
        FORCE_SERIAL.store(false, Ordering::Relaxed);

        let mut buf = source.clone();
        let start = Instant::now();
        apply(&adjustment, &mut buf).unwrap();
        let parallel = start.elapsed();

        eprintln!(
            "pictura perf: adjust {label} serial={serial:?} parallel={parallel:?} speedup={:.2}x",
            serial.as_secs_f64() / parallel.as_secs_f64()
        );
    }
}
