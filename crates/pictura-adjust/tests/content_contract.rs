use pictura_adjust::*;
use pictura_core::PixelBuffer;

fn levels() -> LevelsParams {
    LevelsParams {
        input_black: 0,
        input_white: 255,
        gamma: 1.0,
        output_black: 0,
        output_white: 255,
        red: None,
        green: None,
        blue: None,
    }
}

fn neutral_adjustments() -> Vec<(Adjustment, u8)> {
    let stops = vec![
        GradientStop {
            location: 0,
            color: [0; 3],
        },
        GradientStop {
            location: 4096,
            color: [255; 3],
        },
    ];
    vec![
        (Adjustment::Levels(levels()), 128),
        (
            Adjustment::Curves(CurvesParams {
                points: vec![(0, 0), (255, 255)],
                red: None,
                green: None,
                blue: None,
                black: None,
            }),
            128,
        ),
        (
            Adjustment::BrightnessContrast(BrightnessContrastParams {
                brightness: 0,
                contrast: 0,
                use_legacy: false,
            }),
            128,
        ),
        (
            Adjustment::Exposure(ExposureParams {
                exposure: 0.0,
                offset: 0.0,
                gamma: 1.0,
            }),
            128,
        ),
        (
            Adjustment::HueSaturation(HueSaturationParams {
                hue: 0,
                saturation: 0,
                lightness: 0,
                ranges: Vec::new(),
            }),
            128,
        ),
        (
            Adjustment::BlackWhite(BlackWhiteParams {
                red: 40.0,
                yellow: 60.0,
                green: 40.0,
                cyan: 60.0,
                blue: 20.0,
                magenta: 80.0,
                tint: false,
                tint_color: [0; 3],
            }),
            128,
        ),
        (
            Adjustment::PhotoFilter(PhotoFilterParams {
                color: [255, 180, 80],
                density: 0.0,
                preserve_luminosity: false,
            }),
            128,
        ),
        (Adjustment::ChannelMixer(ChannelMixerParams::default()), 128),
        (
            Adjustment::Vibrance(VibranceParams {
                vibrance: 0,
                saturation: 0,
            }),
            128,
        ),
        (
            Adjustment::ShadowsHighlights(ShadowsHighlightsParams {
                shadows_amount: 0.0,
                highlights_amount: 0.0,
                local: None,
            }),
            128,
        ),
        (
            Adjustment::ColorBalance(ColorBalanceParams {
                shadows: [0.0; 3],
                midtones: [0.0; 3],
                highlights: [0.0; 3],
                preserve_luminosity: false,
            }),
            128,
        ),
        (
            Adjustment::SelectiveColor(SelectiveColorParams::default()),
            128,
        ),
        (Adjustment::Auto(AutoKind::Tone), 128),
        (Adjustment::Auto(AutoKind::Contrast), 128),
        (Adjustment::Auto(AutoKind::Color), 128),
        (Adjustment::Invert, 127),
        (Adjustment::Posterize(255), 128),
        (Adjustment::Threshold(127), 255),
        (Adjustment::Desaturate, 128),
        (Adjustment::Equalize, 128),
        (
            Adjustment::GradientMap(GradientMapParams {
                stops,
                reverse: false,
                transparency: Vec::new(),
            }),
            128,
        ),
        (
            Adjustment::ColorLookup(ColorLookupParams {
                kind: ColorLookupKind::ThreeDLut,
                lookup: None,
            }),
            128,
        ),
        (
            Adjustment::ReplaceColor(ReplaceColorParams {
                samples: vec![ReplaceColorSample {
                    x: -1,
                    y: -1,
                    rgb: [128; 3],
                }],
                fuzziness: 40.0,
                localized: false,
                hue: 0.0,
                saturation: 0.0,
                lightness: 0.0,
            }),
            128,
        ),
        (
            Adjustment::MatchColor(MatchColorParams {
                target: MatchColorStats::ZERO,
                source: MatchColorStats::ZERO,
                neutralize: false,
                luminance: 100.0,
                color_intensity: 100.0,
                fade: 0.0,
            }),
            128,
        ),
    ]
}

fn buffer(channels: u8) -> PixelBuffer {
    let mut data = vec![128; 3];
    if channels == 4 {
        data.push(31);
    }
    PixelBuffer {
        width: 1,
        height: 1,
        channels,
        data: data.into(),
    }
}

#[test]
fn every_destructive_variant_applies_to_valid_rgb_and_rgba() {
    let cases = neutral_adjustments();
    assert_eq!(cases.len(), 24);
    for (adjustment, expected) in cases {
        for channels in [3, 4] {
            let mut image = buffer(channels);
            apply(&adjustment, &mut image)
                .unwrap_or_else(|error| panic!("{adjustment:?}: {error}"));
            let mut samples = vec![expected; 3];
            if channels == 4 {
                samples.push(31);
            }
            assert_eq!(image.data.as_slice(), samples, "{adjustment:?}");
        }
    }
}

#[test]
fn invalid_adjustment_parameters_leave_buffers_unchanged() {
    let mut invalid = vec![
        Adjustment::Posterize(1),
        Adjustment::Threshold(0),
        Adjustment::Curves(CurvesParams {
            points: vec![(0, 0)],
            red: None,
            green: None,
            blue: None,
            black: None,
        }),
        Adjustment::GradientMap(GradientMapParams {
            stops: vec![],
            reverse: false,
            transparency: vec![],
        }),
        Adjustment::ColorLookup(ColorLookupParams {
            kind: ColorLookupKind::ThreeDLut,
            lookup: Some(Lut3d {
                size: 1,
                points: vec![[0.0; 3]],
            }),
        }),
    ];
    for (adjustment, _) in neutral_adjustments() {
        let mut value = adjustment;
        match &mut value {
            Adjustment::Levels(p) => p.gamma = 0.0,
            Adjustment::BrightnessContrast(p) => p.brightness = 200,
            Adjustment::Exposure(p) => p.gamma = f64::NAN,
            Adjustment::HueSaturation(p) => p.hue = 200,
            Adjustment::BlackWhite(p) => p.red = 999.0,
            Adjustment::PhotoFilter(p) => p.density = 150.0,
            Adjustment::ChannelMixer(p) => p.constant[0] = 999.0,
            Adjustment::Vibrance(p) => p.vibrance = 500,
            Adjustment::ShadowsHighlights(p) => p.shadows_amount = f64::INFINITY,
            Adjustment::ColorBalance(p) => p.midtones[0] = f64::NAN,
            Adjustment::SelectiveColor(p) => p.ranges[0].c = 200,
            Adjustment::ReplaceColor(p) => p.fuzziness = f64::NAN,
            Adjustment::MatchColor(p) => p.fade = 200.0,
            _ => continue,
        }
        invalid.push(value);
    }
    for adjustment in invalid {
        for channels in [3, 4] {
            let mut image = buffer(channels);
            let original = image.clone();
            assert!(
                matches!(
                    apply(&adjustment, &mut image),
                    Err(AdjustError::InvalidParams(_))
                ),
                "{adjustment:?}"
            );
            assert_eq!(image, original, "{adjustment:?}");
        }
    }
}

#[test]
fn unsupported_buffers_are_refused_without_mutation() {
    for channels in [0, 1, 2, 5] {
        let mut image = PixelBuffer {
            width: 1,
            height: 1,
            channels,
            data: vec![19; usize::from(channels)].into(),
        };
        let original = image.clone();
        assert!(apply(&Adjustment::Invert, &mut image).is_err());
        assert_eq!(image, original);
    }
    let mut image = buffer(4);
    let original = image.clone();
    assert!(apply_with_coverage(&Adjustment::Invert, &mut image, Some(&[])).is_err());
    assert_eq!(image, original);
}

#[test]
fn content_reexports_are_identical_to_core_types() {
    fn core_adjustment(value: pictura_core::Adjustment) -> Adjustment {
        value
    }
    fn core_gradient(value: pictura_core::GradientFillParams) -> GradientFillParams {
        value
    }
    fn core_match(value: pictura_core::MatchColorParams) -> MatchColorParams {
        value
    }
    assert_eq!(
        core_adjustment(Adjustment::Invert),
        pictura_core::Adjustment::Invert
    );
    let gradient = GradientFillParams {
        stops: Vec::new(),
        reverse: true,
        kind: GradientKind::Radial,
        angle_deg: 45.0,
        scale: 60.0,
    };
    assert_eq!(core_gradient(gradient.clone()), gradient);
    let params = MatchColorParams {
        target: MatchColorStats::ZERO,
        source: MatchColorStats::ZERO,
        neutralize: false,
        luminance: 100.0,
        color_intensity: 100.0,
        fade: 0.0,
    };
    assert_eq!(core_match(params), params);
}
