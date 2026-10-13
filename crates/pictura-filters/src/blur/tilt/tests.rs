use super::*;

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

fn effects_off() -> (BokehParams, BlurNoise) {
    (
        BokehParams {
            enabled: false,
            ..Default::default()
        },
        BlurNoise {
            enabled: false,
            ..Default::default()
        },
    )
}

fn blur(buf: &mut PixelBuffer, pins: &[TiltPin]) {
    let (bokeh, noise) = effects_off();
    tilt_shift(buf, pins, &bokeh, &noise).unwrap();
}

/// A symmetric band through `(0.5, y)`: `focus` and `fade` in buffer heights.
fn band(y: f64, focus: f64, fade: f64, amount: f64) -> TiltPin {
    TiltPin {
        y,
        focus_above: focus,
        focus_below: focus,
        fade_above: fade,
        fade_below: fade,
        blur: amount,
        ..TiltPin::default()
    }
}

/// A noisy picture, so any blur changes it.
fn noise_picture(w: u32, h: u32) -> PixelBuffer {
    let px: Vec<[u8; 3]> = (0..(w * h) as usize)
        .map(|i| {
            let v = ((i * 2_654_435_761) >> 7) as u8;
            [v, v.wrapping_mul(3), v ^ 0x5A]
        })
        .collect();
    buf3(w, h, &px)
}

fn changed(a: &PixelBuffer, b: &PixelBuffer, x: u32, y: u32) -> bool {
    let n = a.pixel_count();
    let i = (y * a.width + x) as usize;
    (0..3).any(|c| a.data[c * n + i] != b.data[c * n + i])
}

fn row_changes(a: &PixelBuffer, b: &PixelBuffer, y: u32) -> usize {
    (0..a.width).filter(|&x| changed(a, b, x, y)).count()
}

/// The red plane's spread: the count of non-zero pixels in a window round
/// `(x, y)` after a white impulse there is blurred.
fn impulse_spread(pins: &[TiltPin], x: u32, y: u32) -> usize {
    let (w, h) = (64u32, 64u32);
    let mut px = vec![[0u8; 3]; (w * h) as usize];
    px[(y * w + x) as usize] = [255, 255, 255];
    let mut buf = buf3(w, h, &px);
    blur(&mut buf, pins);
    let mut count = 0;
    for yy in y.saturating_sub(12)..(y + 13).min(h) {
        for xx in x.saturating_sub(12)..(x + 13).min(w) {
            if buf.data[(yy * w + xx) as usize] > 0 {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn the_focus_band_is_untouched_and_the_far_rows_blur() {
    let base = noise_picture(64, 64);
    let mut buf = base.clone();
    // Focus 8 px each way of row 32, then 16 px of fade.
    blur(&mut buf, &[band(0.5, 0.125, 0.25, 6.0)]);
    for y in 25..39 {
        assert_eq!(row_changes(&buf, &base, y), 0, "focus row {y} changed");
    }
    for y in [0, 2, 61, 63] {
        assert_eq!(row_changes(&buf, &base, y), 64, "blur row {y} kept pixels");
    }
}

#[test]
fn the_fade_ramps_from_the_focus_line_to_the_full_blur() {
    let pins = [band(0.5, 0.125, 0.25, 8.0)];
    let core = impulse_spread(&pins, 32, 32);
    let fade = impulse_spread(&pins, 32, 46);
    let outside = impulse_spread(&pins, 32, 60);
    assert_eq!(core, 1, "an impulse in the focus band spread");
    assert!(fade > core, "the fade did not blur ({fade})");
    assert!(
        outside > fade,
        "the blur zone ({outside}) is not wider than the fade ({fade})"
    );
}

#[test]
fn each_side_keeps_its_own_extents() {
    let base = noise_picture(64, 64);
    let mut buf = base.clone();
    // Sharp from 16 px above row 32 to 2 px below it.
    let pin = TiltPin {
        focus_above: 0.25,
        focus_below: 0.03125,
        fade_above: 0.0,
        fade_below: 0.0,
        blur: 6.0,
        ..TiltPin::default()
    };
    blur(&mut buf, &[pin]);
    assert_eq!(row_changes(&buf, &base, 18), 0, "the wide side blurred");
    assert_eq!(
        row_changes(&buf, &base, 40),
        64,
        "the narrow side stayed sharp"
    );
}

#[test]
fn rotation_turns_the_band() {
    let base = noise_picture(64, 64);
    let mut buf = base.clone();
    let pin = TiltPin {
        x: 0.5,
        angle: 90.0,
        ..band(0.5, 0.125, 0.0, 6.0)
    };
    blur(&mut buf, &[pin]);
    // A vertical band: the centre columns stay sharp, the far rows' ends blur.
    for y in 0..64 {
        for x in 26..38 {
            assert!(!changed(&buf, &base, x, y), "band pixel ({x}, {y}) changed");
        }
        assert!(changed(&buf, &base, 2, y) || changed(&buf, &base, 61, y));
    }
}

#[test]
fn two_bands_keep_both_sharp() {
    let base = noise_picture(64, 64);
    let mut buf = base.clone();
    blur(
        &mut buf,
        &[band(0.2, 0.06, 0.06, 6.0), band(0.8, 0.06, 0.06, 6.0)],
    );
    assert_eq!(row_changes(&buf, &base, 13), 0);
    assert_eq!(row_changes(&buf, &base, 51), 0);
    assert_eq!(
        row_changes(&buf, &base, 32),
        64,
        "the gap between the bands stayed sharp"
    );
}

/// A picture of vertical stripes, so a radial smear changes the rows below
/// the band more than the plain blur does.
fn stripes() -> PixelBuffer {
    let px: Vec<[u8; 3]> = (0..64 * 64)
        .map(|i| {
            let v = if (i % 64) / 3 % 2 == 0 { 230 } else { 20 };
            [v, v, v]
        })
        .collect();
    buf3(64, 64, &px)
}

#[test]
fn distortion_smears_below_the_band_only_unless_symmetric() {
    let plain_pin = band(0.5, 0.06, 0.06, 4.0);
    let mut plain = stripes();
    blur(&mut plain, &[plain_pin]);
    let distorted_pin = TiltPin {
        distortion: 100.0,
        ..plain_pin
    };
    let mut distorted = stripes();
    blur(&mut distorted, &[distorted_pin]);
    assert_eq!(
        row_changes(&distorted, &plain, 4),
        0,
        "the zone above was distorted"
    );
    assert!(
        row_changes(&distorted, &plain, 60) > 0,
        "the zone below was not distorted"
    );

    let mut both = stripes();
    blur(
        &mut both,
        &[TiltPin {
            symmetric: true,
            ..distorted_pin
        }],
    );
    assert!(
        row_changes(&both, &plain, 4) > 0,
        "symmetric left the zone above alone"
    );

    let mut spun = stripes();
    blur(
        &mut spun,
        &[TiltPin {
            distortion: -100.0,
            ..plain_pin
        }],
    );
    assert_ne!(
        spun.data, distorted.data,
        "negative distortion matched positive"
    );
    for y in 28..36 {
        assert_eq!(row_changes(&spun, &plain, y), 0, "the band was distorted");
    }
}

#[test]
fn the_effects_only_touch_the_blurred_zone() {
    let base = noise_picture(64, 64);
    let mut buf = base.clone();
    let bokeh = BokehParams {
        enabled: true,
        light_bokeh: 100.0,
        bokeh_color: 100.0,
        light_range: (0, 255),
    };
    let noise = BlurNoise {
        enabled: true,
        amount: 60.0,
        ..BlurNoise::default()
    };
    tilt_shift(&mut buf, &[band(0.5, 0.125, 0.125, 6.0)], &bokeh, &noise).unwrap();
    for y in 25..39 {
        assert_eq!(row_changes(&buf, &base, y), 0, "focus row {y} changed");
    }
}

#[test]
fn zero_amounts_and_no_pins_are_no_ops() {
    let base = noise_picture(16, 16);
    let mut buf = base.clone();
    blur(&mut buf, &[]);
    assert_eq!(buf.data, base.data);
    blur(&mut buf, &[band(0.5, 0.1, 0.1, 0.0)]);
    assert_eq!(buf.data, base.data);
}

#[test]
fn invalid_parameters_are_rejected_without_mutation() {
    let base = noise_picture(8, 8);
    let (bokeh, noise) = effects_off();
    let bad = [
        TiltPin {
            angle: f64::NAN,
            ..TiltPin::default()
        },
        TiltPin {
            focus_above: -0.1,
            ..TiltPin::default()
        },
        TiltPin {
            fade_below: 10.5,
            ..TiltPin::default()
        },
        TiltPin {
            distortion: 101.0,
            ..TiltPin::default()
        },
        TiltPin {
            blur: 501.0,
            ..TiltPin::default()
        },
    ];
    for pin in bad {
        let mut buf = base.clone();
        let result = tilt_shift(&mut buf, &[pin], &bokeh, &noise);
        assert!(
            matches!(result, Err(FilterError::InvalidParams(_))),
            "{pin:?}"
        );
        assert_eq!(buf.data, base.data);
    }
    let mut buf = base.clone();
    let too_many = vec![TiltPin::default(); MAX_FIELD_PINS + 1];
    assert!(tilt_shift(&mut buf, &too_many, &bokeh, &noise).is_err());
    assert_eq!(buf.data, base.data);
}

#[test]
fn alpha_feathers_beyond_the_band_only() {
    // Alpha stripes across the columns: the band rows keep them, the far
    // rows (blurred and distorted) soften them.
    let mut buf = noise_picture(32, 32);
    let n = buf.pixel_count();
    let alpha: Vec<u8> = (0..n).map(|i| if i % 4 < 2 { 255 } else { 0 }).collect();
    let mut data = buf.data.to_vec();
    data.extend_from_slice(&alpha);
    buf.data = data.into();
    buf.channels = 4;
    blur(
        &mut buf,
        &[TiltPin {
            distortion: 60.0,
            ..band(0.5, 0.125, 0.1, 6.0)
        }],
    );
    let row = |y: usize| &buf.data[3 * n + y * 32..3 * n + (y + 1) * 32];
    assert_eq!(
        row(16),
        &alpha[16 * 32..17 * 32],
        "the band's alpha changed"
    );
    for y in [1, 30] {
        assert!(
            row(y).iter().any(|&a| a > 0 && a < 255),
            "row {y} kept hard alpha"
        );
    }
}

#[test]
fn tiny_images_and_repeat_runs_are_deterministic() {
    let pins = [TiltPin {
        distortion: -40.0,
        symmetric: true,
        angle: 25.0,
        ..TiltPin::default()
    }];
    for (w, h) in [(1, 1), (1, 7), (5, 1)] {
        let mut buf = noise_picture(w, h);
        blur(&mut buf, &pins);
    }
    let mut a = noise_picture(48, 40);
    let mut b = a.clone();
    blur(&mut a, &pins);
    blur(&mut b, &pins);
    assert_eq!(a.data, b.data);
}
