use super::*;
use pictura_core::{CrsSettings, PicturaRawSettings};

fn gray(w: u32, h: u32, value: u8) -> PixelBuffer {
    PixelBuffer {
        width: w,
        height: h,
        channels: 3,
        data: vec![value; (w * h) as usize * 3].into(),
    }
}

fn rgb_grid(colors: &[[u8; 3]]) -> PixelBuffer {
    let n = colors.len() as u32;
    let mut data = Vec::new();
    for c in colors {
        data.push(c[0]);
    }
    for c in colors {
        data.push(c[1]);
    }
    for c in colors {
        data.push(c[2]);
    }
    PixelBuffer {
        width: n,
        height: 1,
        channels: 3,
        data: data.into(),
    }
}

fn px(buf: &PixelBuffer, x: usize) -> [u8; 3] {
    let n = buf.pixel_count();
    [buf.data[x], buf.data[n + x], buf.data[2 * n + x]]
}

fn px2(buf: &PixelBuffer, x: usize, y: usize) -> [u8; 3] {
    let w = buf.width as usize;
    px(buf, y * w + x)
}

fn settings_with(apply: impl FnOnce(&mut PicturaRawSettings)) -> PicturaRawSettings {
    let mut s = PicturaRawSettings::default();
    apply(&mut s);
    s
}

#[test]
fn default_settings_are_a_byte_identical_no_op() {
    let input = gray(4, 4, 128);
    let out = render_pictura_raw(&input, &PicturaRawSettings::default()).unwrap();
    assert_eq!(out, input);
    // NaN fields are ignored, not propagated.
    let weird = PicturaRawSettings {
        basic: CrsSettings {
            exposure: Some(f64::NAN),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(render_pictura_raw(&input, &weird).unwrap(), input);
}

#[test]
fn a_zero_set_control_in_each_tab_is_still_a_no_op() {
    let input = gray(4, 4, 77);
    let cases: Vec<PicturaRawSettings> = vec![
        settings_with(|s| s.basic.exposure = Some(0.0)),
        settings_with(|s| s.tone_curve.parametric_highlights = Some(0)),
        settings_with(|s| s.detail.sharpen_amount = Some(0)),
        settings_with(|s| s.hsl.hue[0] = Some(0)),
        settings_with(|s| s.split_toning.shadow_saturation = Some(0)),
        settings_with(|s| s.lens.distortion = Some(0)),
        settings_with(|s| s.effects.dehaze = Some(0)),
        settings_with(|s| s.calibration.red_hue = Some(0)),
        settings_with(|s| s.mode.grayscale = Some(false)),
    ];
    for s in cases {
        assert_eq!(render_pictura_raw(&input, &s).unwrap(), input);
    }
}

#[test]
fn exposure_is_monotonic_and_preserves_alpha() {
    let input = gray(2, 2, 100);
    let brighter =
        render_pictura_raw(&input, &settings_with(|s| s.basic.exposure = Some(1.0))).unwrap();
    let darker =
        render_pictura_raw(&input, &settings_with(|s| s.basic.exposure = Some(-1.0))).unwrap();
    assert!(px(&brighter, 0)[0] > 100);
    assert!(px(&darker, 0)[0] < 100);

    let mut rgba = input.clone();
    rgba.channels = 4;
    let n = rgba.pixel_count();
    rgba.data.extend(std::iter::repeat_n(7u8, n));
    let out = render_pictura_raw(&rgba, &settings_with(|s| s.basic.exposure = Some(1.0))).unwrap();
    assert_eq!(out.channels, 4);
    assert!(out.data[3 * n..].iter().all(|&a| a == 7), "alpha preserved");
}

#[test]
fn temperature_is_monotonic_on_a_neutral() {
    let input = gray(2, 2, 128);
    let warm =
        render_pictura_raw(&input, &settings_with(|s| s.basic.temperature = Some(60.0))).unwrap();
    let cool = render_pictura_raw(
        &input,
        &settings_with(|s| s.basic.temperature = Some(-60.0)),
    )
    .unwrap();
    let warm_px = px(&warm, 0);
    let cool_px = px(&cool, 0);
    assert!(warm_px[0] as i32 - warm_px[2] as i32 > 0, "warm is redder");
    assert!(
        (cool_px[0] as i32) - (cool_px[2] as i32) < 0,
        "cool is bluer"
    );
}

#[test]
fn clarity_boosts_local_contrast() {
    let input = rgb_grid(&[[64, 64, 64], [64, 64, 64], [192, 192, 192], [192, 192, 192]]);
    let more =
        render_pictura_raw(&input, &settings_with(|s| s.basic.clarity = Some(80.0))).unwrap();
    let less =
        render_pictura_raw(&input, &settings_with(|s| s.basic.clarity = Some(-80.0))).unwrap();
    let edge = |b: &PixelBuffer| px(b, 2)[0] as i32 - px(b, 1)[0] as i32;
    assert!(edge(&more) > edge(&input), "positive clarity steepens");
    assert!(edge(&less) < edge(&input), "negative clarity softens");
}

#[test]
fn vibrance_increases_saturation_and_is_deterministic() {
    let input = rgb_grid(&[[130, 150, 130], [130, 150, 130]]);
    let settings = settings_with(|s| s.basic.vibrance = Some(70.0));
    let a = render_pictura_raw(&input, &settings).unwrap();
    let b = render_pictura_raw(&input, &settings).unwrap();
    assert_eq!(a, b, "deterministic");
    let sat = |v: [u8; 3]| v[1] as i32 - v[0] as i32;
    assert!(sat(px(&a, 0)) > sat(px(&input, 0)), "more saturated");
}

#[test]
fn tone_curve_point_curve_raises_the_midtones() {
    let input = gray(4, 1, 128);
    let lifted = settings_with(|s| {
        s.tone_curve.curve_rgb = vec![0, 0, 128, 180, 255, 255];
    });
    let out = render_pictura_raw(&input, &lifted).unwrap();
    assert!(px(&out, 0)[0] > 128, "the midpoint is raised");
    // A positive highlight region slider brightens a bright pixel.
    let bright = gray(1, 1, 220);
    let hi = settings_with(|s| s.tone_curve.parametric_highlights = Some(100));
    assert!(px(&render_pictura_raw(&bright, &hi).unwrap(), 0)[0] > 220);
}

#[test]
fn detail_sharpening_raises_edge_contrast_and_masking_gates_flat_detail() {
    let input = rgb_grid(&[
        [100, 100, 100],
        [100, 100, 100],
        [110, 110, 110],
        [110, 110, 110],
    ]);
    let open = render_pictura_raw(
        &input,
        &settings_with(|s| {
            s.detail.sharpen_amount = Some(150);
            s.detail.sharpen_masking = Some(0);
        }),
    )
    .unwrap();
    let masked = render_pictura_raw(
        &input,
        &settings_with(|s| {
            s.detail.sharpen_amount = Some(150);
            s.detail.sharpen_masking = Some(100);
        }),
    )
    .unwrap();
    let step = |b: &PixelBuffer| (px(b, 2)[0] as i32 - px(b, 1)[0] as i32).abs();
    assert!(step(&open) > step(&input), "sharpening steepens the edge");
    assert!(
        step(&masked) < step(&open),
        "masking gates low-contrast detail"
    );
}

#[test]
fn hsl_range_saturation_moves_only_its_own_hue() {
    let input = rgb_grid(&[[200, 40, 40], [40, 200, 40]]);
    let out =
        render_pictura_raw(&input, &settings_with(|s| s.hsl.saturation[0] = Some(80))).unwrap();
    let sat = |v: [u8; 3]| v[0] as i32 - v[1] as i32;
    assert!(
        sat(px(&out, 0)) > sat(px(&input, 0)),
        "reds get more saturated"
    );
    assert_eq!(px(&out, 1), px(&input, 1), "greens are untouched");
}

#[test]
fn grayscale_mix_weights_the_conversion_by_hue() {
    let input = rgb_grid(&[[200, 40, 40], [40, 40, 200]]);
    let plain =
        render_pictura_raw(&input, &settings_with(|s| s.mode.grayscale = Some(true))).unwrap();
    let mixed = render_pictura_raw(
        &input,
        &settings_with(|s| {
            s.mode.grayscale = Some(true);
            s.hsl.grayscale_mix[0] = Some(80);
        }),
    )
    .unwrap();
    assert!(px(&mixed, 0)[0] > px(&plain, 0)[0], "reds brighten");
    assert_eq!(px(&mixed, 1), px(&plain, 1), "blues are untouched");
}

#[test]
fn hsl_luminance_does_not_touch_a_neutral() {
    // A grey has no hue, so a hue-targeted range (Reds) must not move it.
    let input = gray(2, 2, 128);
    let out =
        render_pictura_raw(&input, &settings_with(|s| s.hsl.luminance[0] = Some(100))).unwrap();
    assert_eq!(out, input);
}

#[test]
fn split_toning_tints_the_two_ends_oppositely() {
    let input = rgb_grid(&[[40, 40, 40], [220, 220, 220]]);
    let out = render_pictura_raw(
        &input,
        &settings_with(|s| {
            s.split_toning.shadow_hue = Some(0);
            s.split_toning.shadow_saturation = Some(100);
            s.split_toning.highlight_hue = Some(240);
            s.split_toning.highlight_saturation = Some(100);
        }),
    )
    .unwrap();
    let shadow = px(&out, 0);
    let highlight = px(&out, 1);
    assert!(shadow[0] > shadow[1], "shadows warm toward red");
    assert!(highlight[2] > highlight[0], "highlights cool toward blue");
}

#[test]
fn lens_vignette_darkens_the_corners() {
    let input = gray(9, 9, 200);
    let out = render_pictura_raw(
        &input,
        &settings_with(|s| {
            s.lens.vignette_amount = Some(-100);
            s.lens.vignette_midpoint = Some(50);
        }),
    )
    .unwrap();
    assert!(px2(&out, 0, 0)[0] < px2(&out, 4, 4)[0], "corner darkens");
}

#[test]
fn lens_distortion_moves_off_centre_detail() {
    let mut input = gray(21, 21, 0);
    let n = input.pixel_count();
    let idx = 10 * 21 + 2;
    input.data[idx] = 255;
    input.data[n + idx] = 255;
    input.data[2 * n + idx] = 255;
    let out =
        render_pictura_raw(&input, &settings_with(|s| s.lens.distortion = Some(100))).unwrap();
    assert_ne!(out, input, "distortion resamples the image");
    let white = |b: &PixelBuffer, i: usize| {
        b.data[i] as u32 + b.data[n + i] as u32 + b.data[2 * n + i] as u32
    };
    assert_eq!(white(&input, idx), 765);
    assert!(white(&out, idx) < 765, "the feature moved off its pixel");
}

#[test]
fn lens_defringe_desaturates_only_its_default_hue_window() {
    // Default purple window must target magenta, not green/cyan.
    let input = rgb_grid(&[[180, 60, 200], [200, 40, 40], [40, 200, 40]]);
    let purple = render_pictura_raw(
        &input,
        &settings_with(|s| s.lens.defringe_purple_amount = Some(100)),
    )
    .unwrap();
    let magenta = px(&purple, 0);
    assert!(
        (magenta[2] as i32 - magenta[0] as i32) < 10,
        "magenta is desaturated"
    );
    assert_eq!(px(&purple, 1), px(&input, 1), "red stays saturated");

    let green = render_pictura_raw(
        &input,
        &settings_with(|s| s.lens.defringe_green_amount = Some(100)),
    )
    .unwrap();
    let g = px(&green, 2);
    assert!(g[0] == g[1] && g[1] == g[2], "green is desaturated");
    assert_eq!(
        px(&green, 0),
        px(&input, 0),
        "magenta survives green defringe"
    );
}

#[test]
fn effects_post_crop_vignette_darkens_the_corners() {
    let input = gray(9, 9, 200);
    let out = render_pictura_raw(
        &input,
        &settings_with(|s| {
            s.effects.vignette_amount = Some(-100);
            s.effects.vignette_midpoint = Some(50);
        }),
    )
    .unwrap();
    assert!(px2(&out, 0, 0)[0] < px2(&out, 4, 4)[0], "corner darkens");
}

#[test]
fn every_tab_changes_the_render() {
    let input = rgb_grid(&[
        [180, 120, 60],
        [60, 120, 180],
        [120, 120, 120],
        [200, 80, 80],
    ]);
    let cases: Vec<PicturaRawSettings> = vec![
        settings_with(|s| s.basic.exposure = Some(0.5)),
        settings_with(|s| s.tone_curve.parametric_highlights = Some(50)),
        settings_with(|s| s.detail.sharpen_amount = Some(100)),
        settings_with(|s| s.hsl.saturation[0] = Some(50)),
        settings_with(|s| s.split_toning.shadow_saturation = Some(50)),
        settings_with(|s| s.lens.vignette_amount = Some(-100)),
        settings_with(|s| s.effects.dehaze = Some(50)),
        settings_with(|s| s.calibration.red_hue = Some(50)),
    ];
    for s in cases {
        assert_ne!(render_pictura_raw(&input, &s).unwrap(), input);
    }
}

#[test]
fn effects_dehaze_raises_contrast_and_grain_is_deterministic() {
    let input = rgb_grid(&[[100, 100, 100], [150, 150, 150]]);
    let dehazed =
        render_pictura_raw(&input, &settings_with(|s| s.effects.dehaze = Some(80))).unwrap();
    let spread = |b: &PixelBuffer| px(b, 1)[0] as i32 - px(b, 0)[0] as i32;
    assert!(spread(&dehazed) > spread(&input), "dehaze raises contrast");

    let uniform = gray(4, 4, 128);
    let grain_settings = settings_with(|s| {
        s.effects.grain_amount = Some(100);
        s.effects.grain_size = Some(25);
        s.effects.grain_frequency = Some(50);
    });
    let a = render_pictura_raw(&uniform, &grain_settings).unwrap();
    let b = render_pictura_raw(&uniform, &grain_settings).unwrap();
    assert_eq!(a, b, "grain is deterministic");
    assert_ne!(a, uniform, "grain changes the pixels");
}

#[test]
fn calibration_primaries_rotate_their_hue_and_shadow_tint_spares_highlights() {
    let red = rgb_grid(&[[200, 40, 40]]);
    let rotated =
        render_pictura_raw(&red, &settings_with(|s| s.calibration.red_hue = Some(100))).unwrap();
    assert_ne!(px(&rotated, 0), px(&red, 0), "the red primary rotates");

    let green = rgb_grid(&[[40, 200, 40]]);
    let g = render_pictura_raw(
        &green,
        &settings_with(|s| s.calibration.green_hue = Some(100)),
    )
    .unwrap();
    assert_ne!(px(&g, 0), px(&green, 0), "the green primary rotates");

    let blue = rgb_grid(&[[40, 40, 200]]);
    let b = render_pictura_raw(
        &blue,
        &settings_with(|s| s.calibration.blue_hue = Some(100)),
    )
    .unwrap();
    assert_ne!(px(&b, 0), px(&blue, 0), "the blue primary rotates");

    let input = rgb_grid(&[[40, 40, 40], [220, 220, 220]]);
    let tinted = render_pictura_raw(
        &input,
        &settings_with(|s| s.calibration.shadow_tint = Some(100)),
    )
    .unwrap();
    let dark_delta = (px(&tinted, 0)[0] as i32 - 40).abs();
    let light_delta = (px(&tinted, 1)[0] as i32 - 220).abs();
    assert!(
        dark_delta > light_delta,
        "the shadow tint spares highlights"
    );
}

#[test]
fn calibration_camera_profile() {
    // The profile name/digest and process version are metadata: they do not
    // transform the pixels on their own.
    let input = gray(2, 2, 128);
    let s = settings_with(|s| {
        s.calibration.camera_profile = Some("Embedded\0".into());
        s.mode.process_version = Some(20120000);
    });
    assert_eq!(render_pictura_raw(&input, &s).unwrap(), input);
}
