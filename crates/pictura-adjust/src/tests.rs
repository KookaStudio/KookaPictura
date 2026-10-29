use super::*;

use crate::common::{luma, rgb_to_hsl, srgb_to_linear};
use pictura_core::PixelBuffer;

mod gradient_map;
mod hue_ranges;
mod levels;
mod parallel;

fn buf3(w: u32, h: u32, px: &[[u8; 3]]) -> PixelBuffer {
    let n = (w * h) as usize;
    assert_eq!(px.len(), n);
    let mut data = vec![0u8; n * 3];
    for (i, p) in px.iter().enumerate() {
        data[i] = p[0];
        data[n + i] = p[1];
        data[2 * n + i] = p[2];
    }
    PixelBuffer {
        width: w,
        height: h,
        channels: 3,
        data: data.into(),
    }
}

fn buf4(px: &[[u8; 4]]) -> PixelBuffer {
    let n = px.len();
    let mut data = vec![0u8; n * 4];
    for (i, p) in px.iter().enumerate() {
        data[i] = p[0];
        data[n + i] = p[1];
        data[2 * n + i] = p[2];
        data[3 * n + i] = p[3];
    }
    PixelBuffer {
        width: n as u32,
        height: 1,
        channels: 4,
        data: data.into(),
    }
}

fn px3(b: &PixelBuffer, i: usize) -> [u8; 3] {
    let n = b.pixel_count();
    [b.data[i], b.data[n + i], b.data[2 * n + i]]
}

fn ramp_rgb(from: u8, to: u8, steps: usize) -> PixelBuffer {
    let px: Vec<[u8; 3]> = (0..steps)
        .map(|i| {
            let v = from as f64 + (to as f64 - from as f64) * i as f64 / (steps - 1) as f64;
            let v = v.round() as u8;
            [v, v, v]
        })
        .collect();
    buf3(steps as u32, 1, &px)
}

const BW_DEFAULT: BlackWhiteParams = BlackWhiteParams {
    red: 40.0,
    yellow: 60.0,
    green: 40.0,
    cyan: 60.0,
    blue: 20.0,
    magenta: 80.0,
    tint: false,
    tint_color: [0, 0, 0],
};

// --- Level / curves / exposure / brightness-contrast -------------------

#[test]
fn levels_identity_and_clamps() {
    let mut b = buf3(2, 1, &[[0, 0, 0], [255, 255, 255]]);
    apply(
        &Adjustment::Levels(LevelsParams {
            input_black: 0,
            input_white: 255,
            gamma: 1.0,
            output_black: 0,
            output_white: 255,
            red: None,
            green: None,
            blue: None,
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0), [0, 0, 0]);
    assert_eq!(px3(&b, 1), [255, 255, 255]);
}

#[test]
fn levels_black_white_point_and_output_range() {
    let mut b = buf3(
        4,
        1,
        &[[3, 3, 3], [124, 124, 124], [243, 243, 243], [250, 250, 250]],
    );
    apply(
        &Adjustment::Levels(LevelsParams {
            input_black: 5,
            input_white: 243,
            gamma: 1.0,
            output_black: 20,
            output_white: 235,
            red: None,
            green: None,
            blue: None,
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0)[0], 20);
    assert_eq!(px3(&b, 2)[0], 235);
    assert_eq!(px3(&b, 3)[0], 235);
    for i in 0..4 {
        let v = px3(&b, i)[0];
        assert!((20..=235).contains(&v), "value {v} out of output range");
    }
}

#[test]
fn levels_gamma_lightens_midtones() {
    let mut b = buf3(1, 1, &[[124, 124, 124]]);
    apply(
        &Adjustment::Levels(LevelsParams {
            input_black: 0,
            input_white: 255,
            gamma: 2.0,
            output_black: 0,
            output_white: 255,
            red: None,
            green: None,
            blue: None,
        }),
        &mut b,
    )
    .unwrap();
    assert!(px3(&b, 0)[0] > 124, "gamma>1 should lighten");
}

#[test]
fn levels_rejects_bad_params() {
    let p = LevelsParams {
        input_black: 200,
        input_white: 100,
        gamma: 1.0,
        output_black: 0,
        output_white: 255,
        red: None,
        green: None,
        blue: None,
    };
    let mut b = buf3(1, 1, &[[0, 0, 0]]);
    assert!(apply(&Adjustment::Levels(p), &mut b).is_err());
    let p = LevelsParams {
        input_black: 0,
        input_white: 255,
        gamma: 0.0,
        output_black: 0,
        output_white: 255,
        red: None,
        green: None,
        blue: None,
    };
    assert!(apply(&Adjustment::Levels(p), &mut b).is_err());
}

#[test]
fn curves_identity_and_control_point() {
    let mut b = ramp_rgb(0, 255, 256);
    let orig = b.clone();
    apply(
        &Adjustment::Curves(CurvesParams {
            points: vec![(0, 0), (255, 255)],
            red: None,
            green: None,
            blue: None,
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(b, orig, "two-point identity must be bit-exact");
    let mut b = buf3(1, 1, &[[100, 100, 100]]);
    apply(
        &Adjustment::Curves(CurvesParams {
            points: vec![(0, 0), (100, 200), (255, 255)],
            red: None,
            green: None,
            blue: None,
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0)[0], 200);
}

#[test]
fn curves_monotone_and_validation() {
    let mut b = ramp_rgb(0, 255, 256);
    apply(
        &Adjustment::Curves(CurvesParams {
            points: vec![(0, 0), (64, 32), (192, 224), (255, 255)],
            red: None,
            green: None,
            blue: None,
        }),
        &mut b,
    )
    .unwrap();
    for i in 1..256 {
        assert!(px3(&b, i)[0] >= px3(&b, i - 1)[0], "curve must be monotone");
    }
    let mut b = buf3(1, 1, &[[0, 0, 0]]);
    assert!(apply(
        &Adjustment::Curves(CurvesParams {
            points: vec![(0, 0)],
            red: None,
            green: None,
            blue: None,
        }),
        &mut b
    )
    .is_err());
    assert!(apply(
        &Adjustment::Curves(CurvesParams {
            points: vec![(0, 0), (0, 10), (255, 255)],
            red: None,
            green: None,
            blue: None,
        }),
        &mut b
    )
    .is_err());
}

#[test]
fn curves_per_channel_touches_only_its_plane() {
    let mut b = buf3(1, 1, &[[100, 150, 200]]);
    apply(
        &Adjustment::Curves(CurvesParams {
            points: vec![(0, 0), (255, 255)],
            red: Some(vec![(0, 0), (128, 255), (255, 255)]),
            green: None,
            blue: None,
        }),
        &mut b,
    )
    .unwrap();
    let p = px3(&b, 0);
    assert!(p[0] > 100, "red follows its curve: {p:?}");
    assert_eq!(p[1], 150, "green is identity");
    assert_eq!(p[2], 200, "blue is identity");
}

#[test]
fn curves_per_channel_only_with_identity_composite() {
    let mut b = buf3(2, 1, &[[10, 20, 30], [200, 100, 50]]);
    apply(
        &Adjustment::Curves(CurvesParams {
            points: vec![(0, 0), (255, 255)],
            red: None,
            green: Some(vec![(0, 0), (128, 255), (255, 255)]),
            blue: None,
        }),
        &mut b,
    )
    .unwrap();
    for (i, p) in [px3(&b, 0), px3(&b, 1)].into_iter().enumerate() {
        let source = [[10u8, 20, 30], [200, 100, 50]][i];
        assert_eq!(p[0], source[0], "red is identity on sample {i}");
        assert_eq!(p[2], source[2], "blue is identity on sample {i}");
        assert!(p[1] > source[1], "green follows its curve on sample {i}");
    }
}

#[test]
fn curves_composite_applies_after_per_channel() {
    // Red 100 is pulled to ~50 by its channel curve, then the composite lifts
    // 50 to ~78; green 100 is only touched by the composite, to ~156.
    let mut b = buf3(1, 1, &[[100, 100, 100]]);
    apply(
        &Adjustment::Curves(CurvesParams {
            points: vec![(0, 0), (128, 200), (255, 255)],
            red: Some(vec![(0, 0), (100, 50), (255, 255)]),
            green: None,
            blue: None,
        }),
        &mut b,
    )
    .unwrap();
    let p = px3(&b, 0);
    assert!(p[0] < p[1], "per-channel red then composite: {p:?}");
    assert_eq!(p[1], p[2], "green and blue share the composite result");
}

#[test]
fn curves_rejects_bad_per_channel() {
    let mut b = buf3(1, 1, &[[10, 10, 10]]);
    assert!(apply(
        &Adjustment::Curves(CurvesParams {
            points: vec![(0, 0), (255, 255)],
            red: Some(vec![(0, 0), (0, 255)]),
            green: None,
            blue: None,
        }),
        &mut b
    )
    .is_err());
}

#[test]
fn bc_identity_both_modes_and_legacy_clip() {
    let b = ramp_rgb(0, 255, 256);
    let orig = b.clone();
    for legacy in [false, true] {
        let mut c = b.clone();
        apply(
            &Adjustment::BrightnessContrast(BrightnessContrastParams {
                brightness: 0,
                contrast: 0,
                use_legacy: legacy,
            }),
            &mut c,
        )
        .unwrap();
        assert_eq!(c, orig);
    }
    let mut b = buf3(2, 1, &[[200, 200, 200], [250, 250, 250]]);
    apply(
        &Adjustment::BrightnessContrast(BrightnessContrastParams {
            brightness: 20,
            contrast: 0,
            use_legacy: true,
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0)[0], 220);
    assert_eq!(px3(&b, 1)[0], 255, "legacy must clip");
}

#[test]
fn bc_modern_contrast_is_soft_and_monotone() {
    let mut b = ramp_rgb(0, 255, 256);
    apply(
        &Adjustment::BrightnessContrast(BrightnessContrastParams {
            brightness: 0,
            contrast: 50,
            use_legacy: false,
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0)[0], 0);
    assert_eq!(px3(&b, 255)[0], 255);
    assert!(px3(&b, 64)[0] < 64, "shadows should darken");
    assert!(px3(&b, 192)[0] > 192, "highlights should lighten");
    for i in 1..256 {
        assert!(px3(&b, i)[0] >= px3(&b, i - 1)[0]);
    }
}

#[test]
fn exposure_identity_and_ev_gain() {
    let mut b = ramp_rgb(0, 255, 256);
    let orig = b.clone();
    apply(
        &Adjustment::Exposure(ExposureParams {
            exposure: 0.0,
            offset: 0.0,
            gamma: 1.0,
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(b, orig, "identity exposure must be bit-exact");

    let mut b = buf3(1, 1, &[[128, 128, 128]]);
    apply(
        &Adjustment::Exposure(ExposureParams {
            exposure: 1.0,
            offset: 0.0,
            gamma: 1.0,
        }),
        &mut b,
    )
    .unwrap();
    let out_lin = srgb_to_linear(px3(&b, 0)[0] as f64 / 255.0);
    let in_lin = srgb_to_linear(128.0 / 255.0);
    assert!(
        (out_lin - 2.0 * in_lin).abs() < 0.02,
        "one EV doubles linear light"
    );
}

#[test]
fn exposure_rejects_bad_gamma() {
    let mut b = buf3(1, 1, &[[10, 10, 10]]);
    assert!(apply(
        &Adjustment::Exposure(ExposureParams {
            exposure: 0.0,
            offset: 0.0,
            gamma: 0.0,
        }),
        &mut b
    )
    .is_err());
}

// --- Invert / posterize / threshold / desaturate -----------------------

#[test]
fn invert_known_and_involution() {
    let mut b = buf3(3, 1, &[[0, 5, 200], [255, 255, 255], [1, 2, 3]]);
    apply(&Adjustment::Invert, &mut b).unwrap();
    assert_eq!(px3(&b, 0), [255, 250, 55]);
    assert_eq!(px3(&b, 1), [0, 0, 0]);
    apply(&Adjustment::Invert, &mut b).unwrap();
    assert_eq!(px3(&b, 0), [0, 5, 200]);
    assert_eq!(px3(&b, 2), [1, 2, 3]);
}

#[test]
fn posterize_levels_and_idempotence() {
    let mut b = buf3(1, 1, &[[64, 64, 64]]);
    apply(&Adjustment::Posterize(4), &mut b).unwrap();
    assert_eq!(px3(&b, 0), [85, 85, 85]);
    let mut b = buf3(
        5,
        1,
        &[
            [0, 0, 0],
            [85, 85, 85],
            [170, 170, 170],
            [200, 200, 200],
            [255, 255, 255],
        ],
    );
    apply(&Adjustment::Posterize(4), &mut b).unwrap();
    assert_eq!(px3(&b, 0)[0], 0);
    assert_eq!(px3(&b, 1)[0], 85);
    assert_eq!(px3(&b, 2)[0], 170);
    assert_eq!(px3(&b, 3)[0], 170);
    assert_eq!(px3(&b, 4)[0], 255);
    let once = b.clone();
    apply(&Adjustment::Posterize(4), &mut b).unwrap();
    assert_eq!(b, once, "posterize must be idempotent");
}

#[test]
fn posterize_two_and_identity_and_invalid() {
    let mut b = buf3(
        4,
        1,
        &[[0, 0, 0], [127, 127, 127], [128, 128, 128], [255, 255, 255]],
    );
    apply(&Adjustment::Posterize(2), &mut b).unwrap();
    assert_eq!(px3(&b, 0)[0], 0);
    assert_eq!(px3(&b, 1)[0], 0);
    assert_eq!(px3(&b, 2)[0], 255);
    assert_eq!(px3(&b, 3)[0], 255);

    let mut b = ramp_rgb(0, 255, 256);
    let orig = b.clone();
    apply(&Adjustment::Posterize(255), &mut b).unwrap();
    assert_eq!(b, orig, "255 levels is the 8-bit identity");

    assert!(apply(&Adjustment::Posterize(0), &mut b).is_err());
    assert!(apply(&Adjustment::Posterize(1), &mut b).is_err());
}

#[test]
fn threshold_binarizes_and_extremes() {
    let mut b = buf3(
        4,
        1,
        &[[255, 0, 0], [0, 0, 0], [255, 255, 255], [128, 128, 128]],
    );
    apply(&Adjustment::Threshold(128), &mut b).unwrap();
    assert_eq!(px3(&b, 0), [0, 0, 0], "red luma 76 is below 128");
    assert_eq!(px3(&b, 1), [0, 0, 0]);
    assert_eq!(px3(&b, 2), [255, 255, 255]);
    assert_eq!(px3(&b, 3), [0, 0, 0], "equality falls to black");

    let mut b = buf3(2, 1, &[[0, 0, 0], [255, 255, 255]]);
    apply(&Adjustment::Threshold(255), &mut b).unwrap();
    assert_eq!(px3(&b, 0), [0, 0, 0]);
    assert_eq!(px3(&b, 1), [0, 0, 0]);
    assert!(apply(&Adjustment::Threshold(0), &mut b).is_err());
}

#[test]
fn equalize_spreads_the_levels_present_and_keeps_greys_neutral() {
    let mut b = buf3(3, 1, &[[100, 100, 100], [120, 120, 120], [140, 140, 140]]);
    apply(&Adjustment::Equalize, &mut b).unwrap();
    assert_eq!(px3(&b, 0), [0, 0, 0]);
    assert_eq!(px3(&b, 1), [128, 128, 128]);
    assert_eq!(px3(&b, 2), [255, 255, 255]);
    // A flat image has nothing to spread.
    let mut flat = buf3(2, 1, &[[60, 60, 60], [60, 60, 60]]);
    apply(&Adjustment::Equalize, &mut flat).unwrap();
    assert_eq!(px3(&flat, 0), [60, 60, 60]);
}

#[test]
fn desaturate_known_and_neutral() {
    let mut b = buf3(2, 1, &[[12, 104, 22], [77, 77, 77]]);
    apply(&Adjustment::Desaturate, &mut b).unwrap();
    assert_eq!(px3(&b, 0), [58, 58, 58]);
    assert_eq!(px3(&b, 1), [77, 77, 77]);
}

#[test]
fn desaturate_matches_hue_saturation_minus_100() {
    let src = [[12, 104, 22], [200, 100, 50], [3, 250, 128]];
    let mut a = buf3(3, 1, &src);
    let mut c = buf3(3, 1, &src);
    apply(&Adjustment::Desaturate, &mut a).unwrap();
    apply(
        &Adjustment::HueSaturation(HueSaturationParams {
            hue: 0,
            saturation: -100,
            lightness: 0,
            ranges: Vec::new(),
        }),
        &mut c,
    )
    .unwrap();
    for i in 0..3 {
        for k in 0..3 {
            let d = px3(&a, i)[k] as i16 - px3(&c, i)[k] as i16;
            assert!(d.abs() <= 1, "desaturate must match H/S -100 within 1 LSB");
        }
    }
}

// --- Hue/saturation and vibrance --------------------------------------

#[test]
fn hue_saturation_identity_and_desaturate() {
    let mut b = buf3(1, 1, &[[200, 100, 50]]);
    let orig = b.clone();
    apply(
        &Adjustment::HueSaturation(HueSaturationParams {
            hue: 0,
            saturation: 0,
            lightness: 0,
            ranges: Vec::new(),
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(b, orig, "neutral H/S must be bit-exact");

    apply(
        &Adjustment::HueSaturation(HueSaturationParams {
            hue: 0,
            saturation: -100,
            lightness: 0,
            ranges: Vec::new(),
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0), [125, 125, 125], "S=-100 keeps HSL lightness");
}

#[test]
fn hue_saturation_lightness_and_validation() {
    let mut b = buf3(2, 1, &[[100, 100, 100], [50, 50, 50]]);
    apply(
        &Adjustment::HueSaturation(HueSaturationParams {
            hue: 0,
            saturation: 0,
            lightness: 50,
            ranges: Vec::new(),
        }),
        &mut b,
    )
    .unwrap();
    assert!(px3(&b, 0)[0] > 100, "positive lightness adds white");
    assert!(px3(&b, 1)[0] > 50);
    assert!(apply(
        &Adjustment::HueSaturation(HueSaturationParams {
            hue: 200,
            saturation: 0,
            lightness: 0,
            ranges: Vec::new(),
        }),
        &mut b
    )
    .is_err());
}

#[test]
fn vibrance_identity_and_diminishing_boost() {
    let mut b = buf3(2, 1, &[[150, 100, 100], [255, 50, 50]]);
    let orig = b.clone();
    apply(
        &Adjustment::Vibrance(VibranceParams {
            vibrance: 0,
            saturation: 0,
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(b, orig);

    let mut b = buf3(2, 1, &[[150, 100, 100], [255, 50, 50]]);
    apply(
        &Adjustment::Vibrance(VibranceParams {
            vibrance: 50,
            saturation: 0,
        }),
        &mut b,
    )
    .unwrap();
    let (_, s_lo, _) = rgb_to_hsl(
        px3(&b, 0)[0] as f64 / 255.0,
        px3(&b, 0)[1] as f64 / 255.0,
        px3(&b, 0)[2] as f64 / 255.0,
    );
    let (_, s_hi, _) = rgb_to_hsl(
        px3(&b, 1)[0] as f64 / 255.0,
        px3(&b, 1)[1] as f64 / 255.0,
        px3(&b, 1)[2] as f64 / 255.0,
    );
    assert!(s_lo > 0.2, "low-saturation patch should gain");
    assert!(s_hi <= 1.0 + 1e-9);
}

#[test]
fn vibrance_saturation_minus_100_is_gray() {
    let mut b = buf3(1, 1, &[[200, 100, 50]]);
    apply(
        &Adjustment::Vibrance(VibranceParams {
            vibrance: 0,
            saturation: -100,
        }),
        &mut b,
    )
    .unwrap();
    let p = px3(&b, 0);
    assert_eq!(p[0], p[1]);
    assert_eq!(p[1], p[2]);
    assert!(apply(
        &Adjustment::Vibrance(VibranceParams {
            vibrance: 500,
            saturation: 0,
        }),
        &mut b
    )
    .is_err());
}

// --- Black & white / photo filter / channel mixer / color balance ------

#[test]
fn black_white_neutral_and_red() {
    let mut b = buf3(2, 1, &[[100, 100, 100], [200, 0, 0]]);
    apply(&Adjustment::BlackWhite(BW_DEFAULT), &mut b).unwrap();
    assert_eq!(px3(&b, 0), [100, 100, 100]);
    assert_eq!(px3(&b, 1), [80, 80, 80], "pure red uses the 40% red weight");
}

#[test]
fn black_white_tint_and_validation() {
    let mut p = BW_DEFAULT.clone();
    p.tint = true;
    p.tint_color = [200, 150, 100];
    let mut b = buf3(1, 1, &[[100, 100, 100]]);
    apply(&Adjustment::BlackWhite(p.clone()), &mut b).unwrap();
    let out = px3(&b, 0);
    assert!(out[0] > out[2], "sepia tint should make R > B");

    let mut bad = BW_DEFAULT.clone();
    bad.red = 999.0;
    assert!(apply(&Adjustment::BlackWhite(bad), &mut b).is_err());
}

#[test]
fn photo_filter_warms_and_preserves_luminosity() {
    let mut b = buf3(2, 1, &[[128, 128, 128], [64, 64, 64]]);
    apply(
        &Adjustment::PhotoFilter(PhotoFilterParams {
            color: [255, 180, 80],
            density: 25.0,
            preserve_luminosity: true,
        }),
        &mut b,
    )
    .unwrap();
    for i in 0..2 {
        let out = px3(&b, i);
        assert!(out[0] > out[2], "warming filter should increase R over B");
        let y = luma(out[0] as f64, out[1] as f64, out[2] as f64);
        let orig = if i == 0 { 128.0 } else { 64.0 };
        assert!((y - orig).abs() < 3.0, "luminosity should be preserved");
    }
}

#[test]
fn photo_filter_density_zero_is_noop_and_validates() {
    let mut b = buf3(1, 1, &[[10, 20, 30]]);
    let orig = b.clone();
    apply(
        &Adjustment::PhotoFilter(PhotoFilterParams {
            color: [255, 0, 0],
            density: 0.0,
            preserve_luminosity: false,
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(b, orig);
    assert!(apply(
        &Adjustment::PhotoFilter(PhotoFilterParams {
            color: [255, 0, 0],
            density: 150.0,
            preserve_luminosity: false,
        }),
        &mut b
    )
    .is_err());
}

#[test]
fn channel_mixer_known_example_and_identity() {
    let mut b = buf3(1, 1, &[[50, 100, 200]]);
    apply(
        &Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: false,
            red: [50.0, 100.0, 0.0],
            green: [0.0, 100.0, 0.0],
            blue: [0.0, 0.0, 100.0],
            constant: [0.0, 0.0, 0.0],
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0), [125, 100, 200]);

    let mut b = buf3(2, 1, &[[10, 20, 30], [200, 100, 50]]);
    let orig = b.clone();
    apply(
        &Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: false,
            red: [100.0, 0.0, 0.0],
            green: [0.0, 100.0, 0.0],
            blue: [0.0, 0.0, 100.0],
            constant: [0.0, 0.0, 0.0],
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(b, orig);
}

#[test]
fn channel_mixer_monochrome_and_constant_and_validation() {
    let mut b = buf3(1, 1, &[[255, 0, 0]]);
    apply(
        &Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: true,
            red: [40.0, 40.0, 20.0],
            green: [40.0, 40.0, 20.0],
            blue: [40.0, 40.0, 20.0],
            constant: [0.0, 0.0, 0.0],
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0), [102, 102, 102]);

    let mut b = buf3(1, 1, &[[100, 100, 100]]);
    apply(
        &Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: true,
            red: [40.0, 40.0, 20.0],
            green: [40.0, 40.0, 20.0],
            blue: [40.0, 40.0, 20.0],
            constant: [10.0, 10.0, 10.0],
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0)[0], 126, "constant adds 10% white");

    assert!(apply(
        &Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: false,
            red: [999.0, 0.0, 0.0],
            green: [0.0, 100.0, 0.0],
            blue: [0.0, 0.0, 100.0],
            constant: [0.0, 0.0, 0.0],
        }),
        &mut b
    )
    .is_err());
}

#[test]
fn color_balance_midtone_red_and_luminosity() {
    let params = ColorBalanceParams {
        shadows: [0.0, 0.0, 0.0],
        midtones: [100.0, 0.0, 0.0],
        highlights: [0.0, 0.0, 0.0],
        preserve_luminosity: false,
    };
    let mut b = buf3(2, 1, &[[0, 0, 0], [128, 128, 128]]);
    apply(&Adjustment::ColorBalance(params.clone()), &mut b).unwrap();
    assert_eq!(px3(&b, 0), [0, 0, 0], "midtones must not touch black");
    assert!(px3(&b, 1)[0] > 128, "mid grey shifts toward red");

    let mut b = buf3(1, 1, &[[128, 128, 128]]);
    let params_lum = ColorBalanceParams {
        preserve_luminosity: true,
        ..params
    };
    apply(&Adjustment::ColorBalance(params_lum), &mut b).unwrap();
    let out = px3(&b, 0);
    let y = luma(out[0] as f64, out[1] as f64, out[2] as f64);
    assert!((y - 128.0).abs() < 3.0, "luminosity preserved");
}

#[test]
fn color_balance_validation() {
    let mut b = buf3(1, 1, &[[10, 10, 10]]);
    assert!(apply(
        &Adjustment::ColorBalance(ColorBalanceParams {
            shadows: [0.0, 0.0, 0.0],
            midtones: [200.0, 0.0, 0.0],
            highlights: [0.0, 0.0, 0.0],
            preserve_luminosity: false,
        }),
        &mut b
    )
    .is_err());
}

// --- Selective Color ---------------------------------------------------

/// An adjustment with one non-zero range; indices are reds…blacks.
fn selective(
    method: SelectiveColorMethod,
    index: usize,
    c: i16,
    m: i16,
    y: i16,
    k: i16,
) -> Adjustment {
    let mut ranges = [SelectiveRange::default(); 9];
    ranges[index] = SelectiveRange { c, m, y, k };
    Adjustment::SelectiveColor(SelectiveColorParams { method, ranges })
}

#[test]
fn selective_color_zero_ranges_is_identity() {
    let mut b = buf3(3, 1, &[[0, 0, 0], [200, 100, 50], [255, 255, 255]]);
    let before = b.clone();
    apply(
        &Adjustment::SelectiveColor(SelectiveColorParams::default()),
        &mut b,
    )
    .unwrap();
    assert_eq!(b, before, "all-zero ranges must be a bit-exact identity");
}

#[test]
fn selective_color_relative_reds_magenta() {
    let mut b = buf3(1, 1, &[[200, 100, 50]]);
    apply(
        &selective(SelectiveColorMethod::Relative, 0, 0, 50, 0, 0),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0), [200, 61, 51]);
}

#[test]
fn selective_color_absolute_reds_yellow() {
    let mut b = buf3(1, 1, &[[200, 100, 50]]);
    apply(
        &selective(SelectiveColorMethod::Absolute, 0, 0, 0, 100, 0),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0), [200, 101, 1]);
}

#[test]
fn selective_color_relative_blacks_black() {
    let mut b = buf3(1, 1, &[[0, 0, 0]]);
    apply(
        &selective(SelectiveColorMethod::Relative, 8, 0, 0, 0, -100),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0), [255, 255, 255]);
}

#[test]
fn selective_color_absolute_whites_cyan() {
    let mut b = buf3(1, 1, &[[255, 255, 255]]);
    apply(
        &selective(SelectiveColorMethod::Absolute, 6, 100, 0, 0, 0),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0), [1, 255, 255]);
}

#[test]
fn selective_color_validation() {
    let mut b = buf3(1, 1, &[[10, 10, 10]]);
    assert!(apply(
        &selective(SelectiveColorMethod::Relative, 0, 101, 0, 0, 0),
        &mut b
    )
    .is_err());
}

// --- Gradient Map ------------------------------------------------------
fn bw_stops() -> Vec<GradientStop> {
    vec![
        GradientStop {
            location: 0,
            color: [0, 0, 0],
        },
        GradientStop {
            location: 4096,
            color: [255, 255, 255],
        },
    ]
}

#[test]
fn gradient_map_black_white_identity_on_greys() {
    let mut b = ramp_rgb(0, 255, 256);
    let orig = b.clone();
    apply(
        &Adjustment::GradientMap(GradientMapParams {
            stops: bw_stops(),
            reverse: false,
            transparency: Vec::new(),
        }),
        &mut b,
    )
    .unwrap();
    for i in 0..256 {
        for c in 0..3 {
            let d = px3(&b, i)[c] as i16 - px3(&orig, i)[c] as i16;
            assert!(d.abs() <= 1, "grey {i} channel {c} drifted by {d}");
        }
    }
}

#[test]
fn gradient_map_reverse_flips() {
    let mut b = buf3(2, 1, &[[0, 0, 0], [255, 255, 255]]);
    apply(
        &Adjustment::GradientMap(GradientMapParams {
            stops: bw_stops(),
            reverse: true,
            transparency: Vec::new(),
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0), [255, 255, 255], "black maps near white");
    assert_eq!(px3(&b, 1), [0, 0, 0], "white maps near black");
}

#[test]
fn gradient_map_interior_stop_is_honored() {
    let stops = vec![
        GradientStop {
            location: 0,
            color: [0, 0, 0],
        },
        GradientStop {
            location: 2048,
            color: [255, 0, 0],
        },
        GradientStop {
            location: 4096,
            color: [255, 255, 255],
        },
    ];
    // A mid-grey samples just past location 2048, so it reads the red interior
    // stop instead of the grey the two outer stops alone would give.
    let mut b = buf3(1, 1, &[[128, 128, 128]]);
    apply(
        &Adjustment::GradientMap(GradientMapParams {
            stops,
            reverse: false,
            transparency: Vec::new(),
        }),
        &mut b,
    )
    .unwrap();
    let out = px3(&b, 0);
    assert!(
        out[0] > 240,
        "mid grey should be near the red stop: {out:?}"
    );
    assert!(out[1] < 12 && out[2] < 12, "not a grey blend: {out:?}");
}

#[test]
fn gradient_map_clamps_outside_stops() {
    let stops = vec![
        GradientStop {
            location: 1024,
            color: [0, 0, 255],
        },
        GradientStop {
            location: 3072,
            color: [255, 255, 0],
        },
    ];
    let mut b = buf3(3, 1, &[[0, 0, 0], [32, 32, 32], [255, 255, 255]]);
    apply(
        &Adjustment::GradientMap(GradientMapParams {
            stops,
            reverse: false,
            transparency: Vec::new(),
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0), [0, 0, 255], "below first stop clamps");
    assert_eq!(px3(&b, 1), [0, 0, 255], "still below first stop clamps");
    assert_eq!(px3(&b, 2), [255, 255, 0], "above last stop clamps");
}

#[test]
fn gradient_map_rejects_invalid_stops() {
    let mut b = buf3(1, 1, &[[10, 10, 10]]);
    let one = GradientMapParams {
        stops: vec![GradientStop {
            location: 0,
            color: [0, 0, 0],
        }],
        reverse: false,
        transparency: Vec::new(),
    };
    assert!(apply(&Adjustment::GradientMap(one), &mut b).is_err());

    let decreasing = GradientMapParams {
        stops: vec![
            GradientStop {
                location: 4096,
                color: [0, 0, 0],
            },
            GradientStop {
                location: 0,
                color: [255, 255, 255],
            },
        ],
        reverse: false,
        transparency: Vec::new(),
    };
    assert!(apply(&Adjustment::GradientMap(decreasing), &mut b).is_err());

    let out_of_range = GradientMapParams {
        stops: vec![
            GradientStop {
                location: 0,
                color: [0, 0, 0],
            },
            GradientStop {
                location: 5000,
                color: [255, 255, 255],
            },
        ],
        reverse: false,
        transparency: Vec::new(),
    };
    assert!(apply(&Adjustment::GradientMap(out_of_range), &mut b).is_err());
}

// --- Auto --------------------------------------------------------------

#[test]
fn auto_tone_stretches_per_channel() {
    let mut b = ramp_rgb(60, 200, 100);
    apply(&Adjustment::Auto(AutoKind::Tone), &mut b).unwrap();
    assert!(px3(&b, 0)[0] <= 2, "dark end maps to black");
    assert!(px3(&b, 99)[0] >= 253, "light end maps to white");
}

#[test]
fn auto_contrast_joint_no_cast() {
    let px: Vec<[u8; 3]> = (0..100)
        .map(|i| {
            let v = 60 + (140 * i / 99) as u8;
            [v, v.saturating_add(0), v]
        })
        .collect();
    let mut b = buf3(100, 1, &px);
    apply(&Adjustment::Auto(AutoKind::Contrast), &mut b).unwrap();
    for i in 0..100 {
        let p = px3(&b, i);
        assert_eq!(p[0], p[1]);
        assert_eq!(p[1], p[2]);
    }
}

#[test]
fn auto_color_neutralizes_midtones() {
    let px: Vec<[u8; 3]> = (0..100)
        .map(|i| {
            let v = 60.0 + 140.0 * i as f64 / 99.0;
            [
                (v + 15.0).round() as u8,
                v.round() as u8,
                (v - 15.0).round() as u8,
            ]
        })
        .collect();
    let mut b = buf3(100, 1, &px);
    apply(&Adjustment::Auto(AutoKind::Color), &mut b).unwrap();
    let (mut sr, mut sg, mut sb) = (0.0f64, 0.0f64, 0.0f64);
    for i in 0..100 {
        let p = px3(&b, i);
        sr += p[0] as f64;
        sg += p[1] as f64;
        sb += p[2] as f64;
    }
    let spread = (sr - sg).abs().max((sg - sb).abs());
    assert!(spread < 300.0, "channels should be pulled toward neutral");
}

// --- Contract / invariants --------------------------------------------

#[test]
fn alpha_is_never_modified() {
    let base = buf4(&[[10, 20, 30, 7], [200, 100, 50, 250], [0, 0, 0, 128]]);
    let n = base.pixel_count();
    let alpha: Vec<u8> = base.data[3 * n..4 * n].to_vec();
    let adjustments = vec![
        Adjustment::Levels(LevelsParams {
            input_black: 5,
            input_white: 250,
            gamma: 1.2,
            output_black: 0,
            output_white: 255,
            red: None,
            green: None,
            blue: None,
        }),
        Adjustment::Curves(CurvesParams {
            points: vec![(0, 0), (128, 180), (255, 255)],
            red: None,
            green: None,
            blue: None,
        }),
        Adjustment::BrightnessContrast(BrightnessContrastParams {
            brightness: 10,
            contrast: 10,
            use_legacy: false,
        }),
        Adjustment::Exposure(ExposureParams {
            exposure: 0.5,
            offset: 0.0,
            gamma: 1.0,
        }),
        Adjustment::HueSaturation(HueSaturationParams {
            hue: 10,
            saturation: 10,
            lightness: 10,
            ranges: Vec::new(),
        }),
        Adjustment::BlackWhite(BW_DEFAULT),
        Adjustment::PhotoFilter(PhotoFilterParams {
            color: [255, 180, 80],
            density: 25.0,
            preserve_luminosity: false,
        }),
        Adjustment::ChannelMixer(ChannelMixerParams {
            monochrome: false,
            red: [100.0, 0.0, 0.0],
            green: [0.0, 100.0, 0.0],
            blue: [0.0, 0.0, 100.0],
            constant: [0.0, 0.0, 0.0],
        }),
        Adjustment::Vibrance(VibranceParams {
            vibrance: 20,
            saturation: 10,
        }),
        Adjustment::ColorBalance(ColorBalanceParams {
            shadows: [0.0, 0.0, 0.0],
            midtones: [20.0, 0.0, 0.0],
            highlights: [0.0, 0.0, 0.0],
            preserve_luminosity: true,
        }),
        selective(SelectiveColorMethod::Relative, 0, 0, 50, 0, 0),
        Adjustment::Auto(AutoKind::Tone),
        Adjustment::Invert,
        Adjustment::Posterize(4),
        Adjustment::Threshold(128),
        Adjustment::Desaturate,
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
            reverse: false,
            transparency: Vec::new(),
        }),
        Adjustment::GradientFill(GradientFillParams {
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
            reverse: false,
            kind: GradientKind::Linear,
            angle_deg: 0.0,
            scale: 100.0,
        }),
        Adjustment::PatternFill(PatternFillParams {
            pattern_id: "pictura-pattern".into(),
            scale: 100.0,
            link_with_layer: true,
            origin: (0, 0),
        }),
        Adjustment::SolidFill([10, 20, 30, 40]),
    ];
    // One entry per `Adjustment` variant: the 17 destructive ones plus the three
    // refused fills (`SolidFill`, `GradientFill`, `PatternFill`).
    assert_eq!(
        adjustments.len(),
        20,
        "every Adjustment variant is exercised"
    );
    for a in adjustments {
        let mut b = base.clone();
        match apply(&a, &mut b) {
            Ok(()) => {}
            Err(AdjustError::Unsupported(_)) => {
                assert_eq!(b, base, "{a:?} refused but mutated the buffer");
            }
            Err(e) => panic!("{a:?} failed: {e}"),
        }
        assert_eq!(&b.data[3 * n..4 * n], &alpha[..], "{a:?} touched alpha");
    }
}

#[test]
fn apply_refuses_gradient_fill_without_mutating() {
    let mut buf = buf3(
        2,
        2,
        &[[10, 20, 30], [40, 50, 60], [70, 80, 90], [100, 110, 120]],
    );
    let before = buf.clone();
    let params = GradientFillParams {
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
        reverse: false,
        kind: GradientKind::Radial,
        angle_deg: 45.0,
        scale: 100.0,
    };
    assert!(matches!(
        apply(&Adjustment::GradientFill(params), &mut buf),
        Err(AdjustError::Unsupported(_))
    ));
    assert_eq!(
        buf, before,
        "a refused gradient fill leaves the buffer unchanged"
    );
}

#[test]
fn apply_refuses_pattern_fill_without_mutating() {
    let mut buf = buf3(
        2,
        2,
        &[[10, 20, 30], [40, 50, 60], [70, 80, 90], [100, 110, 120]],
    );
    let before = buf.clone();
    let params = PatternFillParams {
        pattern_id: "pictura-pattern".into(),
        scale: 50.0,
        link_with_layer: false,
        origin: (3, 4),
    };
    assert!(matches!(
        apply(&Adjustment::PatternFill(params), &mut buf),
        Err(AdjustError::Unsupported(_))
    ));
    assert_eq!(
        buf, before,
        "a refused pattern fill leaves the buffer unchanged"
    );
}

#[test]
fn apply_refuses_solid_fill_without_mutating() {
    let mut buf = buf3(
        2,
        2,
        &[[10, 20, 30], [40, 50, 60], [70, 80, 90], [100, 110, 120]],
    );
    let before = buf.clone();
    assert!(matches!(
        apply(&Adjustment::SolidFill([1, 2, 3, 200]), &mut buf),
        Err(AdjustError::Unsupported(_))
    ));
    assert_eq!(
        buf, before,
        "a refused solid fill leaves the buffer unchanged"
    );
}

#[test]
fn bad_buffers_error_not_panic() {
    let mut empty = PixelBuffer {
        width: 0,
        height: 0,
        channels: 3,
        data: vec![].into(),
    };
    assert!(apply(&Adjustment::Invert, &mut empty).is_err());

    let mut odd = PixelBuffer {
        width: 1,
        height: 1,
        channels: 2,
        data: vec![0, 0].into(),
    };
    assert!(apply(&Adjustment::Invert, &mut odd).is_err());

    let mut short = PixelBuffer {
        width: 2,
        height: 2,
        channels: 3,
        data: vec![0; 5].into(),
    };
    assert!(apply(&Adjustment::Invert, &mut short).is_err());
}

#[test]
fn deterministic_across_runs() {
    let src = [[13, 200, 45], [250, 3, 128], [66, 66, 66]];
    let mut a = buf3(3, 1, &src);
    let mut c = buf3(3, 1, &src);
    let adj = Adjustment::HueSaturation(HueSaturationParams {
        hue: 37,
        saturation: 21,
        lightness: -9,
        ranges: Vec::new(),
    });
    apply(&adj, &mut a).unwrap();
    apply(&adj, &mut c).unwrap();
    assert_eq!(a, c);

    let mut a = buf3(3, 1, &src);
    let mut c = buf3(3, 1, &src);
    apply(&Adjustment::Auto(AutoKind::Color), &mut a).unwrap();
    apply(&Adjustment::Auto(AutoKind::Color), &mut c).unwrap();
    assert_eq!(a, c);
}
