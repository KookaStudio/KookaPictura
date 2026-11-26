//! Shadows/Highlights (`shdH`) kernel behaviour.

use pictura_adjust::{apply, AdjustError, Adjustment, ShadowsHighlightsParams};
use pictura_core::PixelBuffer;

fn buf3(px: &[[u8; 3]]) -> PixelBuffer {
    let n = px.len();
    let mut data = vec![0u8; n * 3];
    for (i, p) in px.iter().enumerate() {
        data[i] = p[0];
        data[n + i] = p[1];
        data[2 * n + i] = p[2];
    }
    PixelBuffer {
        width: n as u32,
        height: 1,
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

fn params(shadows_amount: f64, highlights_amount: f64) -> Adjustment {
    Adjustment::ShadowsHighlights(ShadowsHighlightsParams {
        shadows_amount,
        highlights_amount,
        local: None,
    })
}

#[test]
fn lifts_shadows_and_pulls_highlights() {
    let dark = [20u8, 30, 10];
    let mut b = buf3(&[dark]);
    apply(&params(100.0, 0.0), &mut b).unwrap();
    assert!(px3(&b, 0)[0] > dark[0], "a dark pixel should lighten");
    assert!(px3(&b, 0)[1] > dark[1]);

    let bright = [230u8, 240, 245];
    let mut b = buf3(&[bright]);
    apply(&params(0.0, 100.0), &mut b).unwrap();
    assert!(px3(&b, 0)[0] < bright[0], "a bright pixel should darken");
}

#[test]
fn amount_100_lifts_black_by_the_full_gain() {
    let mut b = buf3(&[[0, 0, 0]]);
    apply(&params(100.0, 0.0), &mut b).unwrap();
    assert_eq!(
        px3(&b, 0),
        [89, 89, 89],
        "0.35 * 255 rounds to 89, not the old ~35"
    );
}

#[test]
fn zero_is_a_noop_and_alpha_is_kept() {
    let mut b = buf4(&[[20, 30, 10, 77], [230, 240, 245, 128]]);
    let orig = b.clone();
    apply(&params(0.0, 0.0), &mut b).unwrap();
    assert_eq!(b, orig, "0/0 is byte-identical");

    apply(&params(50.0, 50.0), &mut b).unwrap();
    let n = b.pixel_count();
    assert_eq!(&b.data[3 * n..], &[77, 128], "alpha is untouched");
    assert_ne!(&b.data[..3 * n], &orig.data[..3 * n]);
}

#[test]
fn rejects_invalid_amounts() {
    let mut b = buf3(&[[10, 10, 10]]);
    for (s, h) in [(101.0, 0.0), (0.0, 101.0), (-1.0, 0.0), (f64::NAN, 0.0)] {
        let before = b.clone();
        assert!(matches!(
            apply(&params(s, h), &mut b),
            Err(AdjustError::InvalidParams(_))
        ));
        assert_eq!(b, before, "a rejected amount must not mutate");
    }
}

fn local_options() -> pictura_adjust::ShadowsHighlightsOptions {
    pictura_adjust::ShadowsHighlightsOptions {
        color_correction: 0.0,
        black_clip: 0.0,
        white_clip: 0.0,
        ..pictura_adjust::ShadowsHighlightsOptions::FACTORY
    }
}

fn local(shadows: f64, highlights: f64, options: pictura_adjust::ShadowsHighlightsOptions) -> Adjustment {
    Adjustment::ShadowsHighlights(ShadowsHighlightsParams {
        shadows_amount: shadows,
        highlights_amount: highlights,
        local: Some(options),
    })
}

#[test]
fn local_surround_changes_equal_pixels() {
    let mut pixels = [[10; 3]; 41];
    pixels[21..].fill([115; 3]);
    pixels[8] = [40; 3];
    pixels[32] = [40; 3];
    let mut options = local_options();
    options.shadows_radius = 2.0;
    let mut blurred = buf3(&pixels);
    apply(&local(60.0, 0.0, options), &mut blurred).unwrap();
    assert!(px3(&blurred, 8)[0] > px3(&blurred, 32)[0]);
    options.shadows_radius = 0.0;
    let mut point = buf3(&pixels);
    apply(&local(60.0, 0.0, options), &mut point).unwrap();
    assert_eq!(px3(&point, 8), px3(&point, 32));
}

#[test]
fn advanced_tones_and_clips() {
    let ramp = buf3(&[[0; 3], [40; 3], [150; 3], [200; 3], [255; 3]]);
    let mut options = local_options();
    options.shadows_radius = 0.0;
    let by_width = [0.0, 50.0, 100.0].map(|width| {
        let mut output = ramp.clone();
        options.shadows_width = width;
        apply(&local(50.0, 0.0, options), &mut output).unwrap();
        output
    });
    assert!(px3(&by_width[0], 0)[0] > 0);
    assert_eq!(px3(&by_width[0], 1), [40; 3]);
    assert!(px3(&by_width[1], 1)[0] > 40);
    assert_eq!(px3(&by_width[1], 2), [150; 3]);
    assert!(px3(&by_width[2], 2)[0] > 150);
    assert_eq!(px3(&by_width[2], 4), [255; 3]);

    options.highlights_radius = 0.0;
    let mut highlights = ramp.clone();
    apply(&local(0.0, 50.0, options), &mut highlights).unwrap();
    assert_eq!(px3(&highlights, 1), [40; 3]);
    assert!(px3(&highlights, 3)[0] < 200);

    let mut contrast = ramp;
    options.midtone_contrast = 80.0;
    apply(&local(0.0, 0.0, options), &mut contrast).unwrap();
    assert!(px3(&contrast, 1)[0] < 40 && px3(&contrast, 3)[0] > 200);

    let ramp = (20..120).map(|v| [v; 3]).collect::<Vec<_>>();
    let mut clipped = buf3(&ramp);
    let mut flat = buf3(&[[110; 3]; 100]);
    options.midtone_contrast = 0.0;
    options.black_clip = 10.0;
    options.white_clip = 10.0;
    let clip = local(0.0, 0.0, options);
    apply(&clip, &mut clipped).unwrap();
    assert_eq!(px3(&clipped, 9), [0; 3]);
    assert_eq!(px3(&clipped, 90), [255; 3]);
    apply(&clip, &mut flat).unwrap();
    assert_eq!(flat, buf3(&[[110; 3]; 100]));
}

#[test]
fn signed_color_and_brightness() {
    use pictura_adjust::{apply_in_mode, ColorBalanceMode};

    let spreads = [-80.0, 0.0, 80.0].map(|correction| {
        let mut image = buf3(&[[62, 40, 30]]);
        let mut options = local_options();
        options.color_correction = correction;
        apply(&local(60.0, 0.0, options), &mut image).unwrap();
        let [r, _, b] = px3(&image, 0);
        r - b
    });
    assert!(spreads[0] < spreads[1] && spreads[1] < spreads[2]);

    let levels = [-30.0, 0.0, 30.0].map(|brightness| {
        let mut image = buf3(&[[128; 3]]);
        let mut options = local_options();
        options.brightness = brightness;
        apply_in_mode(
            &local(0.0, 0.0, options),
            &mut image,
            ColorBalanceMode::Grayscale,
        )
        .unwrap();
        px3(&image, 0)[0]
    });
    assert!(levels[0] < levels[1] && levels[1] < levels[2]);
}

#[test]
fn expanded_precision_and_validation() {
    use pictura_adjust::apply_native;
    use pictura_core::Samples;

    let options = local_options();
    let neutral = local(0.0, 0.0, options);
    let mut bytes = buf4(&[[20, 30, 10, 77]]);
    let before = bytes.clone();
    apply(&neutral, &mut bytes).unwrap();
    assert_eq!(bytes, before);

    let mut native = Samples::U16(vec![8001, 8009, 8001, 8009, 8001, 8009, 21, 43210].into());
    let original = native.clone();
    apply_native(&neutral, &mut native, 2, 1, 4).unwrap();
    assert_eq!(native, original);
    apply_native(&local(10.0, 0.0, options), &mut native, 2, 1, 4).unwrap();
    let Samples::U16(values) = native else { unreachable!() };
    assert!(values[0] > 8001 && values[1] > values[0]);
    assert_eq!(&values[6..], &[21, 43210]);

    let mut invalid = options;
    invalid.shadows_radius = f64::NAN;
    let mut native = original.clone();
    assert!(matches!(
        apply_native(&local(35.0, 0.0, invalid), &mut native, 2, 1, 4),
        Err(AdjustError::InvalidParams(_))
    ));
    assert_eq!(native, original);
    assert!(apply(&local(35.0, 0.0, invalid), &mut bytes).is_err());
    assert_eq!(bytes, before);
    let mut float = Samples::F32(vec![0.2; 8].into());
    let original = float.clone();
    assert!(matches!(
        apply_native(&neutral, &mut float, 2, 1, 4),
        Err(AdjustError::Unsupported(_))
    ));
    assert_eq!(float, original);
}

#[test]
fn legacy_pointwise_outputs() {
    let adjustment = params(100.0, 0.0);
    let mut byte = buf4(&[[0, 0, 0, 77]]);
    apply(&adjustment, &mut byte).unwrap();
    assert_eq!(&byte.data[..], &[89, 89, 89, 77]);
    let mut native = pictura_core::Samples::U16(vec![0, 0, 0, 987].into());
    pictura_adjust::apply_native(&adjustment, &mut native, 1, 1, 4).unwrap();
    assert_eq!(
        native,
        pictura_core::Samples::U16(vec![22937, 22937, 22937, 987].into())
    );
}
