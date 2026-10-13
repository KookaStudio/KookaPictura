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

fn buf4(w: u32, h: u32, px: &[[u8; 4]]) -> PixelBuffer {
    let n = (w * h) as usize;
    assert_eq!(px.len(), n);
    let mut data = vec![0u8; n * 4];
    for (i, p) in px.iter().enumerate() {
        data[i] = p[0];
        data[n + i] = p[1];
        data[2 * n + i] = p[2];
        data[3 * n + i] = p[3];
    }
    PixelBuffer {
        width: w,
        height: h,
        channels: 4,
        data: data.into(),
    }
}

fn pin(x: f64, y: f64, blur: f64) -> BlurPin {
    BlurPin { x, y, blur }
}

fn px3(buf: &PixelBuffer, x: u32, y: u32) -> [u8; 3] {
    let w = buf.width as usize;
    let n = buf.pixel_count();
    let i = y as usize * w + x as usize;
    [buf.data[i], buf.data[n + i], buf.data[2 * n + i]]
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

fn blur(buf: &mut PixelBuffer, pins: &[BlurPin]) {
    let (bokeh, noise) = effects_off();
    field_blur(buf, pins, &bokeh, &noise).unwrap();
}

/// How many non-zero pixels of plane 0 sit within `window` of `x`.
fn spread(buf: &PixelBuffer, x: usize, window: usize) -> usize {
    let w = buf.width as usize;
    let lo = x.saturating_sub(window);
    let hi = (x + window + 1).min(w);
    (lo..hi).filter(|&i| buf.data[i] > 0).count()
}

#[test]
fn a_pin_gradient_blurs_toward_the_larger_amount() {
    let w = 192usize;
    let pins = [pin(8.5 / w as f64, 0.5, 0.0), pin(0.75, 0.5, 60.0)];
    let spread_at = |x: usize| {
        let mut px = vec![[0u8; 3]; w];
        px[x] = [255, 255, 255];
        let mut buf = buf3(w as u32, 1, &px);
        blur(&mut buf, &pins);
        assert!(buf.data[x] > 0, "the impulse vanished at {x}");
        buf.data[..w].iter().filter(|&&v| v > 0).count()
    };
    let at_pin = spread_at(8);
    let middle = spread_at(64);
    let far = spread_at(128);
    assert_eq!(at_pin, 1, "the sharp pin's impulse must not spread");
    assert!(middle > at_pin, "the midpoint did not blur: {middle}");
    assert!(
        far > middle,
        "the gradient is not monotonic: far {far} middle {middle}"
    );
}

#[test]
fn a_pin_outside_the_image_anchors_the_border() {
    let w = 64usize;
    let mut px = vec![[0u8; 3]; w];
    px[4] = [255, 255, 255];
    px[32] = [255, 255, 255];
    let mut buf = buf3(w as u32, 1, &px);
    blur(&mut buf, &[pin(-0.2, 0.5, 0.0), pin(0.5, 0.5, 60.0)]);
    let border = spread(&buf, 4, 24);
    let interior = spread(&buf, 32, 24);
    assert!(
        border < interior,
        "the outside pin did not anchor the border: border {border} interior {interior}"
    );
}

#[test]
fn no_pins_and_zero_amounts_are_noops() {
    let px: Vec<[u8; 3]> = (0..16)
        .map(|i| [(i * 17) as u8, (i * 5) as u8, (i * 3) as u8])
        .collect();
    let base = buf3(4, 4, &px);
    let (bokeh, noise) = effects_off();
    let mut none = base.clone();
    field_blur(&mut none, &[], &bokeh, &noise).unwrap();
    assert_eq!(none.data, base.data);
    let mut zero = base.clone();
    field_blur(
        &mut zero,
        &[pin(0.5, 0.5, 0.0), pin(0.1, 0.9, 0.0)],
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
    for pins in [
        vec![pin(f64::NAN, 0.5, 10.0)],
        vec![pin(0.5, f64::INFINITY, 10.0)],
        vec![pin(0.5, 0.5, -1.0)],
        vec![pin(0.5, 0.5, 501.0)],
        vec![pin(0.5, 0.5, f64::NAN)],
        vec![pin(0.5, 0.5, 1.0); MAX_FIELD_PINS + 1],
    ] {
        let mut buf = base.clone();
        let error = field_blur(&mut buf, &pins, &bokeh, &noise).unwrap_err();
        assert!(matches!(error, FilterError::InvalidParams(_)), "{error}");
        assert_eq!(buf.data, base.data, "mutated for {pins:?}");
    }
    let pins = [pin(0.5, 0.5, 10.0)];
    for (bad_bokeh, bad_noise) in [
        (
            BokehParams {
                light_bokeh: 101.0,
                ..Default::default()
            },
            BlurNoise::default(),
        ),
        (
            BokehParams {
                bokeh_color: -1.0,
                ..Default::default()
            },
            BlurNoise::default(),
        ),
        (
            BokehParams::default(),
            BlurNoise {
                amount: -1.0,
                ..Default::default()
            },
        ),
        (
            BokehParams::default(),
            BlurNoise {
                size: 101.0,
                ..Default::default()
            },
        ),
        (
            BokehParams::default(),
            BlurNoise {
                roughness: f64::NAN,
                ..Default::default()
            },
        ),
        (
            BokehParams::default(),
            BlurNoise {
                color: 200.0,
                ..Default::default()
            },
        ),
        (
            BokehParams::default(),
            BlurNoise {
                highlights: 101.0,
                ..Default::default()
            },
        ),
    ] {
        let mut buf = base.clone();
        let error = field_blur(&mut buf, &pins, &bad_bokeh, &bad_noise).unwrap_err();
        assert!(matches!(error, FilterError::InvalidParams(_)), "{error}");
        assert_eq!(
            buf.data, base.data,
            "mutated for {bad_bokeh:?} {bad_noise:?}"
        );
    }
}

#[test]
fn alpha_blurs_with_the_colour() {
    // An opaque left half on a transparent right half: the edge feathers
    // under a blurring pin and stays hard under a sharp one.
    let px: Vec<[u8; 4]> = (0..64u32)
        .map(|i| [30, 60, 90, if i % 16 < 8 { 255 } else { 0 }])
        .collect();
    let mut buf = buf4(16, 4, &px);
    blur(&mut buf, &[pin(0.5, 0.5, 6.0)]);
    let n = buf.pixel_count();
    assert!(buf.data[3 * n + 8] > 0, "the transparent side stayed clear");
    assert!(buf.data[3 * n + 7] < 255, "the opaque side stayed solid");

    let mut sharp = buf4(16, 4, &px);
    blur(&mut sharp, &[pin(0.5, 0.5, 0.0)]);
    assert_eq!(&sharp.data[3 * n..], &buf4(16, 4, &px).data[3 * n..]);
}

#[test]
fn tiny_images_do_not_panic() {
    for (w, h) in [(1u32, 1u32), (1, 7), (7, 1)] {
        let n = (w * h) as usize;
        let px: Vec<[u8; 3]> = (0..n)
            .map(|i| [(i * 31) as u8, (i * 17) as u8, (i * 7) as u8])
            .collect();
        let mut buf = buf3(w, h, &px);
        blur(&mut buf, &[pin(0.0, 0.0, 5.0), pin(1.0, 1.0, 500.0)]);
    }
}

#[test]
fn equal_inputs_are_bit_identical() {
    let px: Vec<[u8; 3]> = (0..32 * 24)
        .map(|i| {
            [
                ((i * 37) % 256) as u8,
                ((i * 91) % 256) as u8,
                ((i * 13) % 256) as u8,
            ]
        })
        .collect();
    let base = buf3(32, 24, &px);
    let pins = [pin(0.2, 0.3, 12.0), pin(0.8, 0.7, 45.0)];
    let (bokeh, noise) = effects_off();
    let mut first = base.clone();
    let mut second = base.clone();
    field_blur(&mut first, &pins, &bokeh, &noise).unwrap();
    field_blur(&mut second, &pins, &bokeh, &noise).unwrap();
    assert_eq!(first.data, second.data);
    assert_ne!(first.data, base.data, "the blur did nothing");
}

#[test]
fn level_ladder_is_strictly_increasing_and_reaches_the_amount() {
    for max in [1.0, 5.0, 40.0, 500.0] {
        let boxes = level_boxes(max);
        assert!(boxes.windows(2).all(|pair| pair[0] < pair[1]), "{boxes:?}");
        let nominal = level_nominals(&boxes);
        assert!(nominal.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(nominal.last().copied().unwrap_or(0.0) >= max);
    }
}

#[test]
fn light_bokeh_brightens_only_the_blurred_band() {
    let base = buf3(64, 1, &[[100, 100, 100]; 64]);
    let pins = [pin(0.02, 0.5, 0.0), pin(0.98, 0.5, 40.0)];
    let bokeh = BokehParams {
        enabled: true,
        light_bokeh: 100.0,
        bokeh_color: 0.0,
        light_range: (0, 255),
    };
    let mut lit = base.clone();
    field_blur(&mut lit, &pins, &bokeh, &BlurNoise::default()).unwrap();
    assert_eq!(px3(&lit, 0, 0), [100, 100, 100], "a sharp pixel changed");
    assert!(
        px3(&lit, 63, 0)[0] > 100,
        "the blurred band did not brighten: {}",
        px3(&lit, 63, 0)[0]
    );
    // Light Bokeh 0 with Bokeh Color 0 is a no-op.
    let mut plain = base.clone();
    let quiet = BokehParams {
        enabled: true,
        light_bokeh: 0.0,
        bokeh_color: 0.0,
        light_range: (0, 255),
    };
    field_blur(&mut plain, &pins, &quiet, &BlurNoise::default()).unwrap();
    assert_eq!(plain.data, base.data);
}

#[test]
fn bokeh_color_adds_chroma_to_the_lightened_areas() {
    let base = buf3(64, 1, &[[120, 100, 100]; 64]);
    let pins = [pin(0.02, 0.5, 0.0), pin(0.98, 0.5, 40.0)];
    let bokeh = BokehParams {
        enabled: true,
        light_bokeh: 0.0,
        bokeh_color: 100.0,
        light_range: (0, 255),
    };
    let mut buf = base.clone();
    field_blur(&mut buf, &pins, &bokeh, &BlurNoise::default()).unwrap();
    assert_eq!(px3(&buf, 0, 0), [120, 100, 100], "a sharp pixel changed");
    let far = px3(&buf, 63, 0);
    assert!(
        i32::from(far[0]) - i32::from(far[1]) > 20,
        "the blurred band did not gain chroma: {far:?}"
    );
}

#[test]
fn light_range_gates_the_bokeh_effect() {
    let pins = [pin(0.5, 0.5, 40.0)];
    let bokeh = BokehParams {
        enabled: true,
        light_bokeh: 100.0,
        bokeh_color: 0.0,
        light_range: (100, 255),
    };
    let mut bright = buf3(32, 1, &[[200, 200, 200]; 32]);
    field_blur(&mut bright, &pins, &bokeh, &BlurNoise::default()).unwrap();
    // At full strength an in-range bright pixel blooms to white.
    assert_eq!(px3(&bright, 16, 0), [255, 255, 255]);
    let mut dark = buf3(32, 1, &[[50, 50, 50]; 32]);
    field_blur(&mut dark, &pins, &bokeh, &BlurNoise::default()).unwrap();
    assert_eq!(
        px3(&dark, 16, 0),
        [50, 50, 50],
        "an out-of-range tone lifted"
    );
}

#[test]
fn noise_is_seeded_and_follows_its_controls() {
    let base = buf3(64, 1, &[[100, 100, 100]; 64]);
    let pins = [pin(0.5, 0.5, 40.0)];
    let (bokeh, _) = effects_off();
    let noise = BlurNoise {
        enabled: true,
        amount: 60.0,
        size: 50.0,
        roughness: 50.0,
        color: 0.0,
        highlights: 0.0,
        kind: BlurNoiseKind::Grain,
        seed: 7,
    };
    let mut first = base.clone();
    let mut second = base.clone();
    let mut other_seed = base.clone();
    field_blur(&mut first, &pins, &bokeh, &noise).unwrap();
    field_blur(&mut second, &pins, &bokeh, &noise).unwrap();
    field_blur(
        &mut other_seed,
        &pins,
        &bokeh,
        &BlurNoise { seed: 8, ..noise },
    )
    .unwrap();
    assert_eq!(first.data, second.data, "the same seed diverged");
    assert_ne!(first.data, other_seed.data, "a different seed matched");
    assert_ne!(first.data, base.data, "the grain did nothing");
    // Monochromatic grain moves the three channels by the same amount.
    let n = first.pixel_count();
    let i = (0..n).find(|&i| first.data[i] != 100).expect("some grain");
    let delta = |c: usize| i32::from(first.data[c * n + i]) - 100;
    assert_eq!(delta(0), delta(1));
    assert_eq!(delta(1), delta(2));
    // Amount 0 is a no-op.
    let mut quiet = base.clone();
    field_blur(
        &mut quiet,
        &pins,
        &bokeh,
        &BlurNoise {
            amount: 0.0,
            ..noise
        },
    )
    .unwrap();
    assert_eq!(quiet.data, base.data);
    // The Uniform model draws per pixel and differs from Grain.
    let mut per_pixel = base.clone();
    field_blur(
        &mut per_pixel,
        &pins,
        &bokeh,
        &BlurNoise {
            kind: BlurNoiseKind::Uniform,
            ..noise
        },
    )
    .unwrap();
    assert_ne!(per_pixel.data, first.data);
    // Gaussian differs from both as well.
    let mut bell = base.clone();
    field_blur(
        &mut bell,
        &pins,
        &bokeh,
        &BlurNoise {
            kind: BlurNoiseKind::Gaussian,
            ..noise
        },
    )
    .unwrap();
    assert_ne!(bell.data, first.data);
    assert_ne!(bell.data, per_pixel.data);
    // Colour 100 separates the channels.
    let mut colored = base.clone();
    field_blur(
        &mut colored,
        &pins,
        &bokeh,
        &BlurNoise {
            color: 100.0,
            ..noise
        },
    )
    .unwrap();
    let j = (0..n)
        .find(|&j| colored.data[j] != 100)
        .expect("some grain");
    assert!(
        colored.data[j] != colored.data[n + j] || colored.data[n + j] != colored.data[2 * n + j],
        "colour grain kept the channels equal"
    );
}

#[test]
fn noise_highlights_damp_the_grain_in_bright_areas() {
    let base = buf3(64, 1, &[[220, 220, 220]; 64]);
    let pins = [pin(0.5, 0.5, 40.0)];
    let (bokeh, _) = effects_off();
    let noise = BlurNoise {
        enabled: true,
        amount: 100.0,
        size: 50.0,
        roughness: 50.0,
        color: 0.0,
        highlights: 0.0,
        kind: BlurNoiseKind::Grain,
        seed: 7,
    };
    let deviation = |buf: &PixelBuffer| -> u64 {
        buf.data[..buf.pixel_count()]
            .iter()
            .map(|&v| u64::from(v.abs_diff(220)))
            .sum()
    };
    let mut strong = base.clone();
    field_blur(&mut strong, &pins, &bokeh, &noise).unwrap();
    let mut damped = base.clone();
    field_blur(
        &mut damped,
        &pins,
        &bokeh,
        &BlurNoise {
            highlights: 100.0,
            ..noise
        },
    )
    .unwrap();
    assert!(
        deviation(&damped) < deviation(&strong),
        "highlights did not damp the grain: {} vs {}",
        deviation(&damped),
        deviation(&strong)
    );
}
