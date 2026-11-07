//! Color Balance's RGB, Lab, and CMYK mappings.

use super::{buf3, px3};
use crate::common::luma;
use crate::{apply, apply_in_mode, Adjustment, ColorBalanceMode, ColorBalanceParams};

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

fn color_balance_params(midtones: [f64; 3]) -> ColorBalanceParams {
    ColorBalanceParams {
        shadows: [0.0; 3],
        midtones,
        highlights: [0.0; 3],
        preserve_luminosity: false,
    }
}

/// The pre-change RGB kernel, kept verbatim in the test so the refactor's
/// byte-identity claim is checked rather than asserted.
fn old_color_balance_pixel(p: &ColorBalanceParams, rv: u8, gv: u8, bv: u8) -> [u8; 3] {
    let (ir, ig, ib) = (rv as f64, gv as f64, bv as f64);
    let y = luma(ir, ig, ib) / 255.0;
    let sh = (1.0 - y) * (1.0 - y);
    let hi = y * y;
    let mid = (1.0 - sh - hi).max(0.0);
    let w = [sh, mid, hi];
    let mut out = [ir, ig, ib];
    for (c, o) in out.iter_mut().enumerate() {
        let delta = w[0] * p.shadows[c] + w[1] * p.midtones[c] + w[2] * p.highlights[c];
        *o = (*o + delta).clamp(0.0, 255.0);
    }
    if p.preserve_luminosity {
        let y1 = luma(out[0], out[1], out[2]);
        if y1 > 0.0 {
            let scale = luma(ir, ig, ib) / y1;
            for o in &mut out {
                *o = (*o * scale).clamp(0.0, 255.0);
            }
        }
    }
    [
        out[0].round() as u8,
        out[1].round() as u8,
        out[2].round() as u8,
    ]
}

#[test]
fn color_balance_rgb_mode_is_the_default() {
    let params = ColorBalanceParams {
        shadows: [12.0, -6.0, 4.0],
        midtones: [-9.0, 8.0, -3.0],
        highlights: [6.0, 5.0, -11.0],
        preserve_luminosity: true,
    };
    let px: Vec<[u8; 3]> = (0..64)
        .map(|i| {
            [
                (i * 37 % 256) as u8,
                (i * 53 % 256) as u8,
                (i * 91 % 256) as u8,
            ]
        })
        .collect();
    let mut plain = buf3(64, 1, &px);
    let mut mode = buf3(64, 1, &px);
    apply(&Adjustment::ColorBalance(params.clone()), &mut plain).unwrap();
    apply_in_mode(
        &Adjustment::ColorBalance(params.clone()),
        &mut mode,
        ColorBalanceMode::Rgb,
    )
    .unwrap();
    assert_eq!(plain.data, mode.data);
    for (i, pixel) in px.iter().enumerate() {
        assert_eq!(
            px3(&plain, i),
            old_color_balance_pixel(&params, pixel[0], pixel[1], pixel[2]),
            "pixel {pixel:?}"
        );
    }
}

#[test]
fn color_balance_lab_shifts_a_and_b() {
    let base = pictura_color::model::srgb_to_lab([128, 128, 128]);
    let shifted = |midtones| {
        let mut b = buf3(1, 1, &[[128, 128, 128]]);
        apply_in_mode(
            &Adjustment::ColorBalance(color_balance_params(midtones)),
            &mut b,
            ColorBalanceMode::Lab,
        )
        .unwrap();
        pictura_color::model::srgb_to_lab(px3(&b, 0))
    };
    let red = shifted([40.0, 0.0, 0.0]);
    assert!(red[1] > base[1] + 10.0, "Cyan-Red raises a: {red:?}");
    let green = shifted([0.0, 40.0, 0.0]);
    assert!(
        green[1] < base[1] - 10.0,
        "Magenta-Green lowers a: {green:?}"
    );
    let blue = shifted([0.0, 0.0, 40.0]);
    assert!(blue[2] < base[2] - 10.0, "Yellow-Blue lowers b: {blue:?}");
    for out in [red, green, blue] {
        assert!(
            (out[0] - base[0]).abs() <= 1.0,
            "Lab L is unchanged: {out:?}"
        );
    }
}

#[test]
fn color_balance_cmyk_lowers_the_named_ink() {
    let shifted = |midtones| {
        let mut b = buf3(1, 1, &[[128, 128, 128]]);
        apply_in_mode(
            &Adjustment::ColorBalance(color_balance_params(midtones)),
            &mut b,
            ColorBalanceMode::Cmyk,
        )
        .unwrap();
        px3(&b, 0)
    };
    let red = shifted([100.0, 0.0, 0.0]);
    assert!(red[0] > red[1] && red[0] > red[2], "red shift: {red:?}");
    let green = shifted([0.0, 100.0, 0.0]);
    assert!(
        green[1] > green[0] && green[1] > green[2],
        "green shift: {green:?}"
    );
    let blue = shifted([0.0, 0.0, 100.0]);
    assert!(
        blue[2] > blue[0] && blue[2] > blue[1],
        "blue shift: {blue:?}"
    );

    let input = crate::color::rgb_unit_to_cmyk([128.0 / 255.0; 3]);
    let out = crate::color::rgb_unit_to_cmyk(red.map(|v| f64::from(v) / 255.0));
    assert!(out[0] < input[0] - 0.15, "Cyan ink falls: {out:?}");
    assert_eq!(out[3], 0.0, "K is held at the no-black value");
}

#[test]
fn color_balance_other_adjustments_ignore_the_mode() {
    let px = [[10, 20, 30], [200, 150, 100]];
    let mut plain = buf3(2, 1, &px);
    let mut mode = buf3(2, 1, &px);
    apply(&Adjustment::Invert, &mut plain).unwrap();
    apply_in_mode(&Adjustment::Invert, &mut mode, ColorBalanceMode::Lab).unwrap();
    assert_eq!(plain.data, mode.data);
}
