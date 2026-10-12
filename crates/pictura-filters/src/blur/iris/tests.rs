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

fn blur(buf: &mut PixelBuffer, pins: &[IrisPin]) {
    let (bokeh, noise) = effects_off();
    iris_blur(buf, pins, &bokeh, &noise).unwrap();
}

/// A circular iris of `radius` buffer widths with a `feather` core.
fn circle(x: f64, y: f64, radius: f64, feather: f64, amount: f64) -> IrisPin {
    IrisPin {
        x,
        y,
        radius_x: radius,
        radius_y: radius,
        angle: 0.0,
        squareness: 0.0,
        feather,
        blur: amount,
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

#[test]
fn the_sharp_core_is_untouched_and_the_exterior_blurs() {
    let base = noise_picture(64, 64);
    let mut buf = base.clone();
    // Outline radius 16 px, sharp core 8 px.
    blur(&mut buf, &[circle(0.5, 0.5, 0.25, 0.5, 6.0)]);
    for y in 26..38 {
        for x in 26..38 {
            let (dx, dy) = (f64::from(x) + 0.5 - 32.0, f64::from(y) + 0.5 - 32.0);
            if dx.hypot(dy) < 7.5 {
                assert!(!changed(&buf, &base, x, y), "core pixel ({x}, {y}) changed");
            }
        }
    }
    for (x, y) in [(2, 2), (61, 2), (2, 61), (61, 61), (32, 2), (2, 32)] {
        assert!(
            changed(&buf, &base, x, y),
            "exterior pixel ({x}, {y}) stayed sharp"
        );
    }
}

#[test]
fn the_fade_band_ramps_from_sharp_to_the_full_amount() {
    let w = 200usize;
    // Outline radius 60 px, sharp core 30 px.
    let pins = [circle(0.5, 0.5, 0.3, 0.5, 30.0)];
    let spread_at = |x: usize| {
        let mut px = vec![[0u8; 3]; w];
        px[x] = [255, 255, 255];
        let mut buf = buf3(w as u32, 1, &px);
        blur(&mut buf, &pins);
        buf.data[..w].iter().filter(|&&v| v > 0).count()
    };
    let core = spread_at(100 + 20);
    let fade = spread_at(100 + 45);
    let exterior = spread_at(100 + 85);
    assert_eq!(core, 1, "the core impulse spread");
    assert!(fade > core, "the fade band stayed sharp: {fade}");
    assert!(
        exterior > fade,
        "the exterior ({exterior}) is not blurrier than the fade ({fade})"
    );
}

#[test]
fn falloff_follows_rotation_squareness_and_feather() {
    let (w, h) = (100.0, 100.0);
    let wide = IrisPin {
        radius_x: 0.4,
        radius_y: 0.1,
        feather: 0.5,
        squareness: 0.0,
        ..IrisPin::default()
    };
    // 30 px right of the centre: inside the wide axis's core, outside the
    // narrow axis's outline.
    let shape = Shape::new(&wide, w, h);
    assert_eq!(shape.falloff(50.0 + 15.0, 50.0), 0.0);
    assert_eq!(
        shape.falloff(50.0 + 30.0, 50.0),
        shape.falloff(50.0 - 30.0, 50.0)
    );
    let turned = Shape::new(
        &IrisPin {
            angle: 90.0,
            ..wide
        },
        w,
        h,
    );
    assert_eq!(
        turned.falloff(50.0 + 30.0, 50.0),
        1.0,
        "rotation did not turn the iris"
    );
    assert_eq!(turned.falloff(50.0, 50.0 + 15.0), 0.0);

    // The diagonal at 0.8 of each radius is outside an ellipse but inside a
    // near-square.
    let round = IrisPin {
        radius_x: 0.4,
        radius_y: 0.4,
        feather: 0.9,
        squareness: 0.0,
        ..IrisPin::default()
    };
    let corner = (50.0 + 32.0, 50.0 + 32.0);
    assert_eq!(Shape::new(&round, w, h).falloff(corner.0, corner.1), 1.0);
    let square = IrisPin {
        squareness: 100.0,
        ..round
    };
    assert_eq!(Shape::new(&square, w, h).falloff(corner.0, corner.1), 0.0);

    // A wider feather core keeps more of the radius sharp.
    let narrow_core = Shape::new(
        &IrisPin {
            feather: 0.2,
            ..round
        },
        w,
        h,
    );
    let wide_core = Shape::new(
        &IrisPin {
            feather: 0.8,
            ..round
        },
        w,
        h,
    );
    let probe = 50.0 + 0.5 * 40.0;
    assert!(narrow_core.falloff(probe, 50.0) > 0.0);
    assert_eq!(wide_core.falloff(probe, 50.0), 0.0);
}

#[test]
fn exponent_runs_from_an_ellipse_to_a_near_square() {
    assert_eq!(iris_exponent(0.0), 2.0);
    assert!((iris_exponent(100.0) - 20.0).abs() < 1e-9);
    assert!(iris_exponent(50.0) > iris_exponent(10.0));
}

#[test]
fn two_pins_keep_two_sharp_islands() {
    let base = noise_picture(96, 48);
    let mut buf = base.clone();
    blur(
        &mut buf,
        &[
            circle(0.25, 0.5, 0.1, 0.6, 8.0),
            circle(0.75, 0.5, 0.1, 0.6, 8.0),
        ],
    );
    for (x, y) in [(24, 24), (72, 24)] {
        assert!(
            !changed(&buf, &base, x, y),
            "island centre ({x}, {y}) blurred"
        );
    }
    for (x, y) in [(48, 24), (2, 2), (93, 45)] {
        assert!(
            changed(&buf, &base, x, y),
            "({x}, {y}) between the islands stayed sharp"
        );
    }
}

#[test]
fn bokeh_stays_out_of_the_sharp_core() {
    let base = buf3(48, 48, &[[180, 120, 90]; 48 * 48]);
    let mut buf = base.clone();
    let bokeh = BokehParams {
        enabled: true,
        light_bokeh: 100.0,
        bokeh_color: 100.0,
        light_range: (0, 255),
    };
    let noise = BlurNoise {
        enabled: true,
        amount: 100.0,
        ..Default::default()
    };
    iris_blur(
        &mut buf,
        &[circle(0.5, 0.5, 0.25, 0.5, 20.0)],
        &bokeh,
        &noise,
    )
    .unwrap();
    assert!(
        !changed(&buf, &base, 24, 24),
        "the effects reached the core"
    );
    assert!(
        changed(&buf, &base, 1, 1),
        "the effects missed the exterior"
    );
}

#[test]
fn no_pins_and_zero_amounts_are_noops() {
    let base = noise_picture(8, 8);
    let (bokeh, noise) = effects_off();
    let mut none = base.clone();
    iris_blur(&mut none, &[], &bokeh, &noise).unwrap();
    assert_eq!(none.data, base.data);
    let mut zero = base.clone();
    iris_blur(
        &mut zero,
        &[circle(0.5, 0.5, 0.2, 0.5, 0.0)],
        &bokeh,
        &noise,
    )
    .unwrap();
    assert_eq!(zero.data, base.data);
}

#[test]
fn invalid_parameters_are_refused_without_mutation() {
    let base = buf3(8, 8, &[[40, 90, 160]; 64]);
    let (bokeh, noise) = effects_off();
    let good = IrisPin::default();
    for pins in [
        vec![IrisPin {
            x: f64::NAN,
            ..good
        }],
        vec![IrisPin {
            angle: f64::INFINITY,
            ..good
        }],
        vec![IrisPin {
            radius_x: 0.0,
            ..good
        }],
        vec![IrisPin {
            radius_y: MAX_IRIS_RADIUS + 1.0,
            ..good
        }],
        vec![IrisPin {
            squareness: 101.0,
            ..good
        }],
        vec![IrisPin {
            feather: -0.1,
            ..good
        }],
        vec![IrisPin {
            feather: 1.5,
            ..good
        }],
        vec![IrisPin { blur: -1.0, ..good }],
        vec![IrisPin {
            blur: 501.0,
            ..good
        }],
        vec![good; MAX_FIELD_PINS + 1],
    ] {
        let mut buf = base.clone();
        let error = iris_blur(&mut buf, &pins, &bokeh, &noise).unwrap_err();
        assert!(matches!(error, FilterError::InvalidParams(_)), "{error}");
        assert_eq!(buf.data, base.data, "mutated for {pins:?}");
    }
    let mut buf = base.clone();
    let bad = BokehParams {
        light_bokeh: 101.0,
        ..Default::default()
    };
    assert!(iris_blur(&mut buf, &[good], &bad, &noise).is_err());
    assert_eq!(buf.data, base.data);
}

#[test]
fn alpha_is_untouched() {
    let n = 64usize;
    let mut buf = noise_picture(8, 8);
    let alpha: Vec<u8> = (0..n).map(|i| ((i * 7) % 251) as u8).collect();
    let mut data = buf.data.to_vec();
    data.extend_from_slice(&alpha);
    buf.channels = 4;
    buf.data = data.into();
    blur(&mut buf, &[circle(0.5, 0.5, 0.2, 0.5, 20.0)]);
    assert_eq!(&buf.data[3 * n..], &alpha[..]);
}

#[test]
fn tiny_images_and_hard_edges_do_not_panic() {
    for (w, h) in [(1u32, 1u32), (1, 7), (7, 1)] {
        let mut buf = noise_picture(w, h);
        blur(
            &mut buf,
            &[
                circle(0.0, 0.0, 0.5, 1.0, 5.0),
                circle(1.0, 1.0, 0.5, 0.0, 500.0),
            ],
        );
    }
}

#[test]
fn equal_inputs_are_bit_identical() {
    let base = noise_picture(32, 24);
    let pins = [
        IrisPin {
            x: 0.3,
            y: 0.4,
            radius_x: 0.2,
            radius_y: 0.1,
            angle: 30.0,
            squareness: 40.0,
            feather: 0.5,
            blur: 12.0,
        },
        circle(0.8, 0.7, 0.1, 0.3, 45.0),
    ];
    let (bokeh, noise) = effects_off();
    let mut first = base.clone();
    let mut second = base.clone();
    iris_blur(&mut first, &pins, &bokeh, &noise).unwrap();
    iris_blur(&mut second, &pins, &bokeh, &noise).unwrap();
    assert_eq!(first.data, second.data);
    assert_ne!(first.data, base.data, "the blur did nothing");
}
