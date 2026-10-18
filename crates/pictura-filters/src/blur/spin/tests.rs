use super::*;

fn planar(w: u32, h: u32, channels: u32, value: impl Fn(u32, u32, u32) -> u8) -> PixelBuffer {
    let n = (w * h) as usize;
    let mut data = vec![0u8; n * channels as usize];
    for c in 0..channels {
        for y in 0..h {
            for x in 0..w {
                data[c as usize * n + (y * w + x) as usize] = value(x, y, c);
            }
        }
    }
    PixelBuffer {
        width: w,
        height: h,
        channels: channels as u8,
        data: data.into(),
    }
}

/// The offset of pixel `(x, y)`'s centre from the 64×64 picture's centre.
fn polar(x: u32, y: u32) -> (f64, f64) {
    (f64::from(x) + 0.5 - 32.0, f64::from(y) + 0.5 - 32.0)
}

fn stripes() -> PixelBuffer {
    planar(64, 64, 3, |x, _, _| if x % 4 < 2 { 0 } else { 255 })
}

/// A smooth function of the distance from the centre: constant on circles.
fn rings() -> PixelBuffer {
    planar(64, 64, 3, |x, y, _| {
        let (dx, dy) = polar(x, y);
        (128.0 + 100.0 * (dx.hypot(dy) / 5.0).sin()).round() as u8
    })
}

/// A function of the angle round the centre: changes along every circle.
fn spokes() -> PixelBuffer {
    planar(64, 64, 3, |x, y, _| {
        let (dx, dy) = polar(x, y);
        (128.0 + 100.0 * (6.0 * dy.atan2(dx)).sin()).round() as u8
    })
}

fn noise_picture() -> PixelBuffer {
    planar(64, 64, 3, |x, y, c| {
        ((x * 73 + y * 151 + c * 29) % 256) as u8
    })
}

/// A centred spin reaching 0.4 of the width, fully swept to its outline.
fn full(blur_angle: f64) -> SpinPin {
    SpinPin {
        radius_x: 0.4,
        radius_y: 0.4,
        feather: 1.0,
        blur_angle,
        ..SpinPin::default()
    }
}

fn spin(buf: &mut PixelBuffer, spins: &[SpinPin], strobe: &SpinStrobe) {
    spin_blur(buf, spins, strobe, &BlurNoise::default()).unwrap();
}

fn changed(a: &PixelBuffer, b: &PixelBuffer, x: u32, y: u32) -> bool {
    let n = a.pixel_count();
    let i = (y * a.width + x) as usize;
    (0..3).any(|c| a.data[c * n + i] != b.data[c * n + i])
}

#[test]
fn spin_blurs_inside_its_ellipse_only() {
    let base = stripes();
    let mut out = base.clone();
    // A circle 16 px across its radius; the feather ring at 8 px.
    let pin = SpinPin {
        radius_x: 0.25,
        radius_y: 0.25,
        feather: 0.5,
        blur_angle: 90.0,
        ..SpinPin::default()
    };
    spin(&mut out, &[pin], &SpinStrobe::default());
    let mut inside = 0;
    for y in 0..64 {
        for x in 0..64 {
            let (dx, dy) = polar(x, y);
            let r = dx.hypot(dy);
            if r >= 16.0 {
                assert!(!changed(&base, &out, x, y), "({x}, {y}) outside changed");
            } else if (2.0..=8.0).contains(&r) && changed(&base, &out, x, y) {
                inside += 1;
            }
        }
    }
    assert!(inside > 50, "only {inside} pixels inside the ring changed");
}

#[test]
fn spin_follows_circles_round_the_pivot() {
    let base = rings();
    let mut out = base.clone();
    spin(&mut out, &[full(90.0)], &SpinStrobe::default());
    let n = base.pixel_count();
    let worst = (0..n)
        .map(|i| (i32::from(base.data[i]) - i32::from(out.data[i])).abs())
        .max()
        .unwrap();
    assert!(worst <= 3, "rings moved by {worst}");

    let base = spokes();
    let mut out = base.clone();
    spin(&mut out, &[full(90.0)], &SpinStrobe::default());
    let moved = (0..n)
        .filter(|&i| (i32::from(base.data[i]) - i32::from(out.data[i])).abs() > 20)
        .count();
    assert!(moved > 500, "only {moved} spoke pixels moved");
}

#[test]
fn moved_pivot_moves_the_blur() {
    let mut centred = noise_picture();
    spin(&mut centred, &[full(60.0)], &SpinStrobe::default());
    let mut moved = noise_picture();
    let pin = SpinPin {
        pivot_x: 0.15,
        pivot_y: -0.1,
        ..full(60.0)
    };
    spin(&mut moved, &[pin], &SpinStrobe::default());
    assert_ne!(centred.data, moved.data);
}

#[test]
fn strobe_separates_the_arc() {
    // One bright pixel 16 px right of the centre; each output pixel whose arc
    // crosses it picks it up.
    let impulse = || {
        planar(
            64,
            64,
            3,
            |x, y, _| if (x, y) == (48, 32) { 255 } else { 0 },
        )
    };
    let lit = |buf: &PixelBuffer| {
        buf.data[..buf.pixel_count()]
            .iter()
            .filter(|&&v| v > 0)
            .count()
    };
    let mut smooth = impulse();
    spin(&mut smooth, &[full(180.0)], &SpinStrobe::default());
    let mut strobed = impulse();
    let strobe = SpinStrobe {
        strength: 100.0,
        flashes: 4,
        duration: 1.0,
    };
    spin(&mut strobed, &[full(180.0)], &strobe);
    let (smooth_lit, strobed_lit) = (lit(&smooth), lit(&strobed));
    assert!(
        strobed_lit * 2 < smooth_lit,
        "strobe lit {strobed_lit}, continuous {smooth_lit}"
    );
    let peak = |buf: &PixelBuffer| *buf.data[..buf.pixel_count()].iter().max().unwrap();
    assert!(peak(&strobed) > peak(&smooth));
}

#[test]
fn invalid_spins_are_rejected() {
    let bad_spins = [
        SpinPin {
            x: f64::NAN,
            ..SpinPin::default()
        },
        SpinPin {
            blur_angle: 361.0,
            ..SpinPin::default()
        },
        SpinPin {
            radius_y: 0.0,
            ..SpinPin::default()
        },
        SpinPin {
            feather: 1.5,
            ..SpinPin::default()
        },
    ];
    for pin in bad_spins {
        let mut buf = noise_picture();
        let before = buf.data.clone();
        let result = spin_blur(
            &mut buf,
            &[pin],
            &SpinStrobe::default(),
            &BlurNoise::default(),
        );
        assert!(
            matches!(result, Err(FilterError::InvalidParams(_))),
            "{pin:?}"
        );
        assert_eq!(buf.data, before);
    }
    let bad_strobes = [
        SpinStrobe {
            flashes: 0,
            ..SpinStrobe::default()
        },
        SpinStrobe {
            duration: 400.0,
            ..SpinStrobe::default()
        },
        SpinStrobe {
            strength: f64::INFINITY,
            ..SpinStrobe::default()
        },
    ];
    for strobe in bad_strobes {
        let mut buf = noise_picture();
        let result = spin_blur(
            &mut buf,
            &[SpinPin::default()],
            &strobe,
            &BlurNoise::default(),
        );
        assert!(
            matches!(result, Err(FilterError::InvalidParams(_))),
            "{strobe:?}"
        );
    }
    let crowded = vec![SpinPin::default(); MAX_FIELD_PINS + 1];
    let mut buf = noise_picture();
    let result = spin_blur(
        &mut buf,
        &crowded,
        &SpinStrobe::default(),
        &BlurNoise::default(),
    );
    assert!(matches!(result, Err(FilterError::InvalidParams(_))));
}

#[test]
fn spin_no_op_alpha_and_determinism() {
    let base = noise_picture();
    let mut empty = base.clone();
    spin(&mut empty, &[], &SpinStrobe::default());
    assert_eq!(empty.data, base.data);
    let mut still = base.clone();
    spin(&mut still, &[full(0.0)], &SpinStrobe::default());
    assert_eq!(still.data, base.data);

    // The alpha plane spins with the colour.
    let layer = planar(
        64,
        64,
        4,
        |x, _, c| if c == 3 && x % 4 < 2 { 0 } else { 200 },
    );
    let mut out = layer.clone();
    spin(&mut out, &[full(90.0)], &SpinStrobe::default());
    let n = layer.pixel_count();
    assert_ne!(out.data[3 * n..], layer.data[3 * n..]);

    let mut again = noise_picture();
    let mut once = noise_picture();
    spin(&mut once, &[full(120.0)], &SpinStrobe::default());
    spin(&mut again, &[full(120.0)], &SpinStrobe::default());
    assert_eq!(once.data, again.data);
}

#[test]
fn the_smear_shortens_past_the_feather_ring() {
    // Dots pulled in to 0.2: along a ray from the centre the arc length
    // (radius × sweep) peaks at the ring and only falls toward the outline.
    let pin = SpinPin {
        radius_x: 0.4,
        radius_y: 0.4,
        feather: 0.2,
        blur_angle: 90.0,
        ..SpinPin::default()
    };
    let spin = Spin::new(&pin, 100.0, 100.0);
    let arc = |rho: f64| {
        let (mask, at) = spin.mask(50.0 + rho * 40.0, 50.0);
        rho * 40.0 * spin.sweep_at(mask, at)
    };
    let ring = arc(0.2);
    let mut last = ring;
    for step in 1..=16 {
        let rho = 0.2 + 0.05 * f64::from(step);
        let length = arc(rho.min(0.999));
        assert!(
            length <= last + 1e-9,
            "the arc grew at rho {rho}: {length} > {last}"
        );
        last = length;
    }
    // Halfway to the outline the smear is well under the ring's.
    assert!(arc(0.6) < ring * 0.6, "{} vs ring {ring}", arc(0.6));
    assert!(arc(0.9) < ring * 0.1);
}
