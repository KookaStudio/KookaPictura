//! Native-depth tonal application: the 8-bit parity pin, the precision rules
//! for `u16`/`f32` stores, and the unsupported ceiling.

use pictura_adjust::{
    apply, apply_native, parse_cube, AdjustError, Adjustment, AutoKind, BlackWhiteParams,
    BrightnessContrastParams, ChannelMixerParams, ColorBalanceParams, ColorLookupKind,
    ColorLookupParams, CurvesParams, ExposureParams, GradientMapParams, GradientStop,
    HueSaturationParams, LevelsParams, MatchColorParams, MatchColorStats, PhotoFilterParams,
    SelectiveColorMethod, SelectiveColorParams, SelectiveRange, ShadowsHighlightsParams,
    VibranceParams,
};
use pictura_core::{PixelBuffer, Samples};

const W: u32 = 8;
const H: u32 = 8;

/// An inverting size-2 `.CUBE`, i.e. a parsed non-identity lookup.
fn inverted_cube() -> Vec<u8> {
    let mut s = String::from("LUT_3D_SIZE 2\n");
    for b in 0..2 {
        for g in 0..2 {
            for r in 0..2 {
                s.push_str(&format!("{} {} {}\n", 1 - r, 1 - g, 1 - b));
            }
        }
    }
    s.into_bytes()
}

fn color_lookup_params() -> ColorLookupParams {
    ColorLookupParams {
        kind: ColorLookupKind::ThreeDLut,
        lookup: parse_cube(&inverted_cube()),
    }
}

fn fixed_rgb() -> PixelBuffer {
    let n = (W * H) as usize;
    let mut data = vec![0u8; n * 3];
    for i in 0..n {
        data[i] = ((i * 37 + 11) % 256) as u8;
        data[n + i] = ((i * 53 + 29) % 256) as u8;
        data[2 * n + i] = ((i * 91 + 7) % 256) as u8;
    }
    // A 1:2:3 pixel makes the desaturate average land on 1.5, a rounding edge.
    data[0] = 1;
    data[n] = 2;
    data[2 * n] = 3;
    PixelBuffer {
        width: W,
        height: H,
        channels: 3,
        data: data.into(),
    }
}

fn covered() -> Vec<Adjustment> {
    vec![
        Adjustment::Invert,
        Adjustment::Desaturate,
        Adjustment::Levels(LevelsParams {
            input_black: 10,
            input_white: 240,
            gamma: 1.3,
            output_black: 5,
            output_white: 250,
            red: None,
            green: None,
            blue: None,
        }),
        Adjustment::Curves(CurvesParams {
            points: vec![(0, 0), (64, 80), (192, 180), (255, 255)],
            red: Some(vec![(0, 0), (128, 100), (255, 255)]),
            green: None,
            blue: Some(vec![(0, 10), (255, 245)]),
        }),
        Adjustment::BrightnessContrast(BrightnessContrastParams {
            brightness: 30,
            contrast: 20,
            use_legacy: false,
        }),
        Adjustment::BrightnessContrast(BrightnessContrastParams {
            brightness: -40,
            contrast: -15,
            use_legacy: true,
        }),
        Adjustment::Exposure(ExposureParams {
            exposure: 0.8,
            offset: 0.05,
            gamma: 1.1,
        }),
        Adjustment::Posterize(7),
        Adjustment::Threshold(120),
        Adjustment::GradientMap(GradientMapParams {
            stops: vec![
                GradientStop {
                    location: 0,
                    color: [10, 0, 40],
                },
                GradientStop {
                    location: 2048,
                    color: [200, 100, 0],
                },
                GradientStop {
                    location: 4096,
                    color: [255, 255, 255],
                },
            ],
            reverse: false,
            transparency: Vec::new(),
        }),
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
            reverse: true,
            transparency: Vec::new(),
        }),
        Adjustment::HueSaturation(HueSaturationParams {
            hue: 25,
            saturation: 35,
            lightness: -12,
            ranges: Vec::new(),
        }),
        Adjustment::Vibrance(VibranceParams {
            vibrance: 45,
            saturation: -20,
        }),
        Adjustment::ShadowsHighlights(ShadowsHighlightsParams {
            shadows_amount: 70.0,
            highlights_amount: 40.0,
        }),
        Adjustment::ColorBalance(ColorBalanceParams {
            shadows: [12.0, -6.0, 4.0],
            midtones: [-9.0, 8.0, -3.0],
            highlights: [6.0, 5.0, -11.0],
            preserve_luminosity: false,
        }),
        Adjustment::ColorBalance(ColorBalanceParams {
            shadows: [12.0, -6.0, 4.0],
            midtones: [-9.0, 8.0, -3.0],
            highlights: [6.0, 5.0, -11.0],
            preserve_luminosity: true,
        }),
        Adjustment::BlackWhite(BlackWhiteParams {
            red: 40.0,
            yellow: -20.0,
            green: 10.0,
            cyan: 5.0,
            blue: -30.0,
            magenta: 15.0,
            tint: false,
            tint_color: [0, 0, 0],
        }),
        Adjustment::BlackWhite(BlackWhiteParams {
            red: 30.0,
            yellow: 10.0,
            green: -15.0,
            cyan: 25.0,
            blue: 5.0,
            magenta: -10.0,
            tint: true,
            tint_color: [210, 130, 45],
        }),
        Adjustment::PhotoFilter(PhotoFilterParams {
            color: [255, 200, 150],
            density: 25.0,
            preserve_luminosity: false,
        }),
        Adjustment::PhotoFilter(PhotoFilterParams {
            color: [80, 140, 235],
            density: 40.0,
            preserve_luminosity: true,
        }),
        Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: false,
            red: [120.0, -20.0, 10.0],
            green: [10.0, 110.0, -15.0],
            blue: [-10.0, 20.0, 130.0],
            constant: [5.0, -3.0, 2.0],
        }),
        Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: true,
            red: [30.0, 59.0, 11.0],
            green: [0.0, 0.0, 0.0],
            blue: [0.0, 0.0, 0.0],
            constant: [0.0, 0.0, 0.0],
        }),
        Adjustment::SelectiveColor(SelectiveColorParams {
            method: SelectiveColorMethod::Relative,
            ranges: [
                SelectiveRange {
                    c: 20,
                    m: -10,
                    y: 5,
                    k: 0,
                },
                SelectiveRange {
                    c: 0,
                    m: 15,
                    y: -8,
                    k: 3,
                },
                SelectiveRange {
                    c: -12,
                    m: 0,
                    y: 0,
                    k: 0,
                },
                SelectiveRange::default(),
                SelectiveRange {
                    c: 0,
                    m: 0,
                    y: 10,
                    k: 0,
                },
                SelectiveRange::default(),
                SelectiveRange {
                    c: 5,
                    m: -4,
                    y: 2,
                    k: 1,
                },
                SelectiveRange {
                    c: -3,
                    m: 2,
                    y: -1,
                    k: 0,
                },
                SelectiveRange {
                    c: 0,
                    m: 0,
                    y: 0,
                    k: -5,
                },
            ],
        }),
        Adjustment::SelectiveColor(SelectiveColorParams {
            method: SelectiveColorMethod::Absolute,
            ranges: [
                SelectiveRange {
                    c: 10,
                    m: 0,
                    y: -5,
                    k: 0,
                },
                SelectiveRange::default(),
                SelectiveRange {
                    c: 0,
                    m: -8,
                    y: 6,
                    k: 0,
                },
                SelectiveRange {
                    c: 7,
                    m: 0,
                    y: 0,
                    k: 0,
                },
                SelectiveRange::default(),
                SelectiveRange {
                    c: 0,
                    m: 0,
                    y: -4,
                    k: 0,
                },
                SelectiveRange {
                    c: -6,
                    m: 3,
                    y: 0,
                    k: 2,
                },
                SelectiveRange::default(),
                SelectiveRange {
                    c: 0,
                    m: 0,
                    y: 0,
                    k: 4,
                },
            ],
        }),
        Adjustment::Auto(AutoKind::Tone),
        Adjustment::Auto(AutoKind::Contrast),
        Adjustment::Auto(AutoKind::Color),
        Adjustment::ColorLookup(color_lookup_params()),
    ]
}

#[test]
fn native_u8_equals_apply_for_every_covered_adjustment() {
    let base = fixed_rgb();
    for adjustment in covered() {
        let mut expected = base.clone();
        apply(&adjustment, &mut expected).unwrap();
        let mut store = Samples::U8(base.data.clone());
        apply_native(&adjustment, &mut store, W as usize, H as usize, 3).unwrap();
        assert_eq!(store, Samples::U8(expected.data), "{adjustment:?}");
    }
}

#[test]
fn native_u8_leaves_alpha_untouched() {
    let n = (W * H) as usize;
    let mut data = fixed_rgb().data;
    data.extend((0..n).map(|i| (i * 17 + 3) as u8));
    let mut expected = PixelBuffer {
        width: W,
        height: H,
        channels: 4,
        data: data.clone(),
    };
    apply(&Adjustment::Invert, &mut expected).unwrap();
    let mut store = Samples::U8(data);
    apply_native(&Adjustment::Invert, &mut store, W as usize, H as usize, 4).unwrap();
    assert_eq!(store, Samples::U8(expected.data));
}

#[test]
fn depth16_native_edit_is_not_the_widened_byte() {
    let mut store = Samples::U16(
        vec![
            1000, 5000, 20000, 40000, 60000, 12345, 1, 255, 256, 257, 32768, 65535,
        ]
        .into(),
    );
    let levels = Adjustment::Levels(LevelsParams {
        input_black: 0,
        input_white: 255,
        gamma: 1.0,
        output_black: 0,
        output_white: 200,
        red: None,
        green: None,
        blue: None,
    });
    apply_native(&levels, &mut store, 4, 1, 3).unwrap();
    let Samples::U16(v) = &store else { panic!() };
    assert!(
        v.iter().any(|s| (s >> 8) != (s & 0xff)),
        "expected a sample whose low byte is not the high byte: {v:?}"
    );
}

#[test]
fn depth32_native_edit_keeps_values_above_one() {
    let mut store = Samples::F32(vec![1.0, 0.5, 0.25, 0.0, 0.75, 1.0].into());
    let exposure = Adjustment::Exposure(ExposureParams {
        exposure: 2.0,
        offset: 0.0,
        gamma: 1.0,
    });
    apply_native(&exposure, &mut store, 2, 1, 3).unwrap();
    let Samples::F32(v) = &store else { panic!() };
    assert!(
        v.iter().any(|&s| s > 1.0),
        "expected an HDR sample above 1.0: {v:?}"
    );
}

#[test]
fn depth16_color_adjustment_is_not_the_widened_byte() {
    let mut store = Samples::U16(
        vec![
            1000, 5000, 20000, 40000, 60000, 12345, 1, 255, 256, 257, 32768, 65535,
        ]
        .into(),
    );
    let vibrance = Adjustment::Vibrance(VibranceParams {
        vibrance: 70,
        saturation: 0,
    });
    apply_native(&vibrance, &mut store, 4, 1, 3).unwrap();
    let Samples::U16(v) = &store else { panic!() };
    assert!(
        v.iter().any(|s| (s >> 8) != (s & 0xff)),
        "expected a color-adjusted sample whose low byte is not the high byte: {v:?}"
    );
}

#[test]
fn native_exposure_linear() {
    let mut store = Samples::F32(vec![0.5; 3].into());
    let exposure = Adjustment::Exposure(ExposureParams {
        exposure: 1.0,
        offset: 0.0,
        gamma: 1.0,
    });
    apply_native(&exposure, &mut store, 1, 1, 3).unwrap();
    let Samples::F32(v) = &store else { panic!() };
    assert!((v[0] - 1.0).abs() < 1e-6, "{v:?}");
}

#[test]
fn native_posterize_16bit() {
    let levels = [0u16, 16384, 32768, 49151, 65535];
    let mut store = Samples::U16(
        levels
            .iter()
            .flat_map(|&v| [v; 3])
            .collect::<Vec<_>>()
            .into(),
    );
    apply_native(&Adjustment::Posterize(5), &mut store, 5, 1, 3).unwrap();
    let Samples::U16(v) = &store else { panic!() };
    for (i, &expected) in levels.iter().enumerate() {
        assert_eq!(v[i * 3], expected, "level {i}");
    }
    let mut identity = store.clone();
    apply_native(&Adjustment::Posterize(255), &mut identity, 5, 1, 3).unwrap();
    assert_eq!(identity, store);
}

#[test]
fn native_bw_channel_mixer() {
    let mut store = Samples::U16(
        (0..12)
            .map(|i| (i * 5000) as u16)
            .collect::<Vec<_>>()
            .into(),
    );
    let bw = Adjustment::BlackWhite(BlackWhiteParams {
        red: 120.0,
        yellow: -30.0,
        green: 60.0,
        cyan: 10.0,
        blue: -50.0,
        magenta: 25.0,
        tint: true,
        tint_color: [210, 130, 45],
    });
    apply_native(&bw, &mut store, 4, 1, 3).unwrap();
    let Samples::U16(v) = &store else { panic!() };
    assert!(
        v.iter().any(|s| (s >> 8) != (s & 0xff)),
        "B&W must not stay on the 8-bit grid: {v:?}"
    );

    let mut store = Samples::U16(
        (0..12)
            .map(|i| (i * 5000) as u16)
            .collect::<Vec<_>>()
            .into(),
    );
    let mix = Adjustment::ChannelMixer(ChannelMixerParams {
        monochrome: false,
        red: [120.0, -20.0, 10.0],
        green: [10.0, 110.0, -15.0],
        blue: [-10.0, 20.0, 130.0],
        constant: [5.0, -3.0, 2.0],
    });
    apply_native(&mix, &mut store, 4, 1, 3).unwrap();
    let Samples::U16(v) = &store else { panic!() };
    assert!(
        v.iter().any(|s| (s >> 8) != (s & 0xff)),
        "Channel Mixer must not stay on the 8-bit grid: {v:?}"
    );
}

#[test]
fn unsupported_adjustment_is_refused_without_mutation() {
    let mut store = Samples::U16(vec![10, 20, 30, 40, 50, 60].into());
    let before = store.clone();
    let fill = Adjustment::SolidFill([1, 2, 3, 4]);
    let err = apply_native(&fill, &mut store, 2, 1, 3).unwrap_err();
    assert!(matches!(err, AdjustError::Unsupported(_)));
    assert_eq!(store, before);
}

#[test]
fn match_color_is_refused_at_native_depth() {
    // Dialog-only, like Replace Color: no PSD encoding, no native arm.
    let mut store = Samples::U16(vec![10, 20, 30, 40, 50, 60].into());
    let before = store.clone();
    let p = MatchColorParams {
        target: MatchColorStats::ZERO,
        source: MatchColorStats::ZERO,
        neutralize: false,
        luminance: 100.0,
        color_intensity: 100.0,
        fade: 0.0,
    };
    let err = apply_native(&Adjustment::MatchColor(p), &mut store, 2, 1, 3).unwrap_err();
    assert!(matches!(err, AdjustError::Unsupported(_)));
    assert_eq!(store, before);
}

#[test]
fn depth16_color_lookup_keeps_precision() {
    let input = vec![
        1000, 5000, 20000, 40000, 60000, 12345, 1, 255, 256, 257, 32768, 65535,
    ];
    let mut store = Samples::U16(input.clone().into());
    apply_native(
        &Adjustment::ColorLookup(color_lookup_params()),
        &mut store,
        4,
        1,
        3,
    )
    .unwrap();
    let Samples::U16(v) = &store else { panic!() };
    assert!(
        v.iter()
            .zip(&input)
            .any(|(out, inp)| *out != (*inp >> 8) * 257),
        "expected a LUT sample that is not the 8-bit widening of its input: {v:?}"
    );
}

#[test]
fn unparseable_color_lookup_is_a_noop_at_native_depth() {
    let mut store = Samples::U16(vec![10, 20, 30, 40, 50, 60].into());
    let before = store.clone();
    let params = ColorLookupParams {
        kind: ColorLookupKind::AbstractProfile,
        lookup: None,
    };
    apply_native(&Adjustment::ColorLookup(params), &mut store, 2, 1, 3).unwrap();
    assert_eq!(store, before);
}
