use super::*;
use crate::blur::field::field_blur;
use crate::blur::iris::iris_blur;
use crate::blur::path::path_blur;
use crate::blur::tilt::tilt_shift;

fn noise_picture(w: u32, h: u32) -> PixelBuffer {
    let n = (w * h) as usize;
    let mut data = vec![0u8; n * 4];
    for i in 0..n {
        let v = ((i * 2_654_435_761) >> 7) as u8;
        data[i] = v;
        data[n + i] = v.wrapping_mul(3);
        data[2 * n + i] = v ^ 0x5A;
        data[3 * n + i] = if i % 5 == 0 { 90 } else { 255 };
    }
    PixelBuffer {
        width: w,
        height: h,
        channels: 4,
        data: data.into(),
    }
}

fn effects() -> (BokehParams, BlurNoise) {
    (
        BokehParams {
            enabled: true,
            light_bokeh: 40.0,
            bokeh_color: 30.0,
            light_range: (20, 230),
        },
        BlurNoise {
            enabled: true,
            amount: 15.0,
            ..BlurNoise::default()
        },
    )
}

fn iris_pin() -> IrisPin {
    IrisPin {
        x: 0.3,
        y: 0.5,
        radius_x: 0.15,
        radius_y: 0.15,
        feather: 0.5,
        blur: 9.0,
        ..IrisPin::default()
    }
}

fn tilt_pin() -> TiltPin {
    TiltPin {
        y: 0.25,
        focus_above: 0.06,
        focus_below: 0.06,
        fade_above: 0.1,
        fade_below: 0.1,
        distortion: 40.0,
        blur: 7.0,
        ..TiltPin::default()
    }
}

#[test]
fn one_effect_alone_matches_its_own_kernel() {
    let (bokeh, noise) = effects();
    let base = noise_picture(48, 40);
    let field = [BlurPin {
        x: 0.2,
        y: 0.3,
        blur: 6.0,
    }];
    let cases: [(&[BlurPin], &[IrisPin], &[TiltPin]); 3] = [
        (&field, &[], &[]),
        (&[], &[iris_pin()], &[]),
        (&[], &[], &[tilt_pin()]),
    ];
    for (f, i, t) in cases {
        let mut combined = base.clone();
        blur_gallery(
            &mut combined,
            f,
            i,
            t,
            &[],
            &PathMotion::default(),
            &bokeh,
            &noise,
        )
        .unwrap();
        let mut alone = base.clone();
        if !f.is_empty() {
            field_blur(&mut alone, f, &bokeh, &noise).unwrap();
        } else if !i.is_empty() {
            iris_blur(&mut alone, i, &bokeh, &noise).unwrap();
        } else {
            tilt_shift(&mut alone, t, &bokeh, &noise).unwrap();
        }
        assert_eq!(combined.data, alone.data);
    }
}

#[test]
fn combined_effects_blur_where_either_one_does() {
    let (bokeh, noise) = (
        BokehParams {
            enabled: false,
            ..BokehParams::default()
        },
        BlurNoise::default(),
    );
    let base = noise_picture(64, 64);
    let changed = |buf: &PixelBuffer, x: u32, y: u32| {
        let n = buf.pixel_count();
        let i = (y * 64 + x) as usize;
        (0..3).any(|c| buf.data[c * n + i] != base.data[c * n + i])
    };
    // The band is sharp along row 16; the iris core is sharp round (19, 32).
    let mut band = base.clone();
    tilt_shift(&mut band, &[tilt_pin()], &bokeh, &noise).unwrap();
    let mut iris = base.clone();
    iris_blur(&mut iris, &[iris_pin()], &bokeh, &noise).unwrap();
    let mut both = base.clone();
    blur_gallery(
        &mut both,
        &[],
        &[iris_pin()],
        &[tilt_pin()],
        &[],
        &PathMotion::default(),
        &bokeh,
        &noise,
    )
    .unwrap();
    assert!(!changed(&band, 56, 16) && changed(&iris, 56, 16));
    assert!(!changed(&iris, 19, 32) && changed(&band, 19, 32));
    // Each sharp area blurs where the other effect blurs it...
    assert!(
        changed(&both, 56, 16),
        "the iris exterior left the band sharp"
    );
    assert!(
        changed(&both, 19, 32),
        "the band's blur left the iris core sharp"
    );
    // ...and a pixel sharp in both stays bit-identical.
    let mut sharp = base.clone();
    let near = IrisPin {
        x: 0.5,
        y: 0.25,
        ..iris_pin()
    };
    blur_gallery(
        &mut sharp,
        &[],
        &[near],
        &[tilt_pin()],
        &[],
        &PathMotion::default(),
        &bokeh,
        &noise,
    )
    .unwrap();
    assert!(
        !changed(&sharp, 32, 16),
        "a pixel sharp in both effects changed"
    );
}

#[test]
fn no_pins_is_a_no_op_and_bad_pins_are_refused() {
    let (bokeh, noise) = effects();
    let base = noise_picture(16, 16);
    let mut buf = base.clone();
    blur_gallery(
        &mut buf,
        &[],
        &[],
        &[],
        &[],
        &PathMotion::default(),
        &bokeh,
        &noise,
    )
    .unwrap();
    assert_eq!(buf.data, base.data);
    let bad = TiltPin {
        distortion: 200.0,
        ..TiltPin::default()
    };
    assert!(matches!(
        blur_gallery(
            &mut buf,
            &[],
            &[iris_pin()],
            &[bad],
            &[],
            &PathMotion::default(),
            &bokeh,
            &noise
        ),
        Err(FilterError::InvalidParams(_))
    ));
    assert_eq!(buf.data, base.data);
}

#[test]
fn equal_inputs_are_bit_identical() {
    let (bokeh, noise) = effects();
    let field = [BlurPin {
        x: 0.8,
        y: 0.8,
        blur: 12.0,
    }];
    let mut a = noise_picture(40, 30);
    let mut b = a.clone();
    for buf in [&mut a, &mut b] {
        blur_gallery(
            buf,
            &field,
            &[iris_pin()],
            &[tilt_pin()],
            &[],
            &PathMotion::default(),
            &bokeh,
            &noise,
        )
        .unwrap();
    }
    assert_eq!(a.data, b.data);
}

fn path() -> BlurPath {
    BlurPath {
        points: vec![(0.2, 0.6), (0.5, 0.4), (0.8, 0.6)],
        start_speed: 6.0,
        end_speed: 14.0,
    }
}

fn path_motion() -> PathMotion {
    PathMotion {
        speed: 100.0,
        taper: 20.0,
        ..PathMotion::default()
    }
}

#[test]
fn paths_alone_match_their_own_kernel() {
    let (bokeh, noise) = effects();
    let mut combined = noise_picture(48, 40);
    let mut alone = combined.clone();
    let paths = [path()];
    blur_gallery(
        &mut combined,
        &[],
        &[],
        &[],
        &paths,
        &path_motion(),
        &bokeh,
        &noise,
    )
    .unwrap();
    path_blur(&mut alone, &paths, &path_motion(), &noise).unwrap();
    assert_eq!(combined.data, alone.data);
}

#[test]
fn paths_add_to_a_radius_effect() {
    let (bokeh, noise) = effects();
    let base = noise_picture(48, 40);
    let field = [BlurPin {
        x: 0.2,
        y: 0.3,
        blur: 6.0,
    }];
    let paths = [path()];
    let mut both = base.clone();
    blur_gallery(
        &mut both,
        &field,
        &[],
        &[],
        &paths,
        &path_motion(),
        &bokeh,
        &noise,
    )
    .unwrap();
    let mut field_only = base.clone();
    field_blur(&mut field_only, &field, &bokeh, &noise).unwrap();
    let mut path_only = base.clone();
    path_blur(&mut path_only, &paths, &path_motion(), &noise).unwrap();
    assert_ne!(both.data, field_only.data);
    assert_ne!(both.data, path_only.data);
}
