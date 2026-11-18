//! Hue/Saturation's colour ranges and lightness.

use super::{buf3, px3};
use crate::{apply, hue_range_weight, Adjustment, HueRange, HueSaturationParams};

fn reds(saturation: i16, lightness: i16) -> HueRange {
    HueRange {
        begin_ramp: 315,
        begin_sustain: 345,
        end_sustain: 15,
        end_ramp: 45,
        hue: 0,
        saturation,
        lightness,
    }
}

#[test]
fn a_band_wraps_through_zero_and_ramps_linearly() {
    let red = reds(0, 0);
    assert_eq!(hue_range_weight(&red, 0.0), 1.0);
    assert_eq!(hue_range_weight(&red, 350.0), 1.0);
    assert_eq!(hue_range_weight(&red, 330.0), 0.5);
    assert_eq!(hue_range_weight(&red, 30.0), 0.5);
    assert_eq!(hue_range_weight(&red, 120.0), 0.0);
}

#[test]
fn a_reds_edit_leaves_blue_and_gray_alone() {
    let mut b = buf3(3, 1, &[[200, 40, 40], [40, 40, 200], [128, 128, 128]]);
    apply(
        &Adjustment::HueSaturation(HueSaturationParams {
            hue: 0,
            saturation: 0,
            lightness: 0,
            ranges: vec![reds(0, -100)],
        }),
        &mut b,
    )
    .unwrap();
    assert_eq!(px3(&b, 0), [0, 0, 0]);
    assert_eq!(px3(&b, 1), [40, 40, 200]);
    assert_eq!(px3(&b, 2), [128, 128, 128]);

    let bad = HueSaturationParams {
        hue: 0,
        saturation: 0,
        lightness: 0,
        ranges: vec![reds(101, 0)],
    };
    assert!(apply(&Adjustment::HueSaturation(bad), &mut b).is_err());
}

#[test]
fn hue_saturation_lightness_blends_toward_white_and_black() {
    // A dark, nearly neutral pixel (HSL S = 0.33) must not gain colour when
    // lightened; each channel moves toward white by the same fraction.
    let run = |lightness: i16| {
        let mut b = buf3(1, 1, &[[10, 5, 8]]);
        apply(
            &Adjustment::HueSaturation(HueSaturationParams {
                hue: 0,
                saturation: 0,
                lightness,
                ranges: Vec::new(),
            }),
            &mut b,
        )
        .unwrap();
        px3(&b, 0)
    };
    assert_eq!(run(66), [172, 170, 171]);
    assert_eq!(run(-50), [5, 3, 4]);
    assert_eq!(run(100), [255, 255, 255]);
    assert_eq!(run(-100), [0, 0, 0]);
}
