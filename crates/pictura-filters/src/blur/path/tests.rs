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

/// Each row one colour, so a blur along rows leaves it alone.
fn rows(w: u32, h: u32) -> PixelBuffer {
    planar(w, h, 3, |_, y, c| (y * 37 + c * 50) as u8)
}

/// Vertical stripes, so a blur along rows changes them.
fn stripes(w: u32, h: u32) -> PixelBuffer {
    planar(w, h, 3, |x, _, _| if x % 4 < 2 { 0 } else { 255 })
}

/// One bright pixel at `(x, y)` on black.
fn impulse(w: u32, h: u32, x: u32, y: u32) -> PixelBuffer {
    planar(
        w,
        h,
        3,
        |px, py, _| if (px, py) == (x, y) { 255 } else { 0 },
    )
}

fn at(buf: &PixelBuffer, x: u32, y: u32) -> u8 {
    buf.data[(y * buf.width + x) as usize]
}

/// A straight two-point path; `speed` is both end-point speeds.
fn line(from: (f64, f64), to: (f64, f64), speed: f64) -> BlurPath {
    BlurPath {
        points: vec![from, to],
        start_speed: speed,
        end_speed: speed,
    }
}

fn rightward(speed: f64) -> BlurPath {
    line((0.3, 0.5), (0.7, 0.5), speed)
}

/// Full Speed, so `speed` is the trail length in pixels.
fn motion() -> PathMotion {
    PathMotion {
        speed: 100.0,
        ..PathMotion::default()
    }
}

fn blur(buf: &mut PixelBuffer, paths: &[BlurPath], motion: &PathMotion) {
    path_blur(buf, paths, motion, &BlurNoise::default()).unwrap();
}

#[test]
fn a_horizontal_path_blurs_along_rows_only() {
    let mut flat = rows(40, 24);
    let before = flat.clone();
    blur(&mut flat, &[rightward(12.0)], &motion());
    assert_eq!(flat.data, before.data);

    let mut striped = stripes(40, 24);
    let before = striped.clone();
    blur(&mut striped, &[rightward(12.0)], &motion());
    assert_ne!(striped.data, before.data);
}

#[test]
fn a_vertical_path_turns_the_blur() {
    let mut flat = rows(40, 24);
    let before = flat.clone();
    blur(&mut flat, &[line((0.5, 0.2), (0.5, 0.8), 12.0)], &motion());
    assert_ne!(flat.data, before.data);
}

#[test]
fn centred_blur_spreads_both_ways_and_trailing_only_forward() {
    let mut centred = impulse(41, 9, 20, 4);
    blur(&mut centred, &[rightward(10.0)], &motion());
    assert!(at(&centred, 16, 4) > 0 && at(&centred, 24, 4) > 0);
    assert_eq!(
        at(&centred, 20, 3),
        0,
        "a horizontal trail stays on its row"
    );

    let mut trailing = impulse(41, 9, 20, 4);
    let off = PathMotion {
        centered: false,
        ..motion()
    };
    blur(&mut trailing, &[rightward(10.0)], &off);
    // A pixel gathers from behind it, so the streak runs ahead of the subject.
    assert!(at(&trailing, 25, 4) > 0);
    assert_eq!(at(&trailing, 15, 4), 0);
}

#[test]
fn a_sub_pixel_trail_is_a_no_op() {
    let mut picture = stripes(32, 16);
    let before = picture.clone();
    blur(&mut picture, &[rightward(0.5)], &motion());
    assert_eq!(picture.data, before.data);
    let stopped = PathMotion {
        speed: 0.0,
        ..motion()
    };
    blur(&mut picture, &[rightward(40.0)], &stopped);
    assert_eq!(picture.data, before.data);
    blur(&mut picture, &[], &motion());
    assert_eq!(picture.data, before.data);
}

#[test]
fn strobe_flashes_leave_separated_ghosts() {
    let strobe = PathMotion {
        centered: false,
        strobe_strength: 100.0,
        strobe_flashes: 4,
        ..motion()
    };
    let mut picture = impulse(64, 5, 10, 2);
    blur(&mut picture, &[rightward(30.0)], &strobe);
    let row: Vec<u8> = (10..41).map(|x| at(&picture, x, 2)).collect();
    let lit = row.iter().filter(|&&v| v > 0).count();
    assert_eq!(lit, 4, "four flashes, four ghosts: {row:?}");
}

#[test]
fn taper_fades_the_far_end() {
    let tapered = PathMotion {
        centered: false,
        taper: 100.0,
        ..motion()
    };
    let mut picture = impulse(64, 5, 10, 2);
    blur(&mut picture, &[rightward(20.0)], &tapered);
    // The near end of the streak is brighter than its far end.
    assert!(at(&picture, 11, 2) > at(&picture, 27, 2));

    let mut flat = impulse(64, 5, 10, 2);
    blur(
        &mut flat,
        &[rightward(20.0)],
        &PathMotion {
            centered: false,
            ..motion()
        },
    );
    assert_eq!(at(&flat, 11, 2), at(&flat, 27, 2));
}

#[test]
fn rear_sync_keeps_the_subject_brightest() {
    let flash = PathMotion {
        rear_sync: true,
        ..motion()
    };
    let mut picture = impulse(41, 5, 20, 2);
    blur(&mut picture, &[rightward(10.0)], &flash);
    let subject = at(&picture, 20, 2);
    assert!(
        subject >= 127,
        "half the weight stays on the subject: {subject}"
    );
    assert!(subject > at(&picture, 22, 2) && at(&picture, 22, 2) > 0);
}

#[test]
fn the_nearer_path_wins() {
    // A horizontal path along the top and a vertical one along the bottom:
    // the rows picture survives near the top and changes near the bottom.
    let paths = [
        line((0.2, 0.05), (0.8, 0.05), 10.0),
        line((0.5, 0.9), (0.5, 0.98), 10.0),
    ];
    let mut flat = rows(48, 64);
    let before = flat.clone();
    blur(&mut flat, &paths, &motion());
    let row = |buf: &PixelBuffer, y: u32| -> Vec<u8> { (0..48).map(|x| at(buf, x, y)).collect() };
    assert_ne!(row(&flat, 58), row(&before, 58));
    let top_change: i32 = (0..48)
        .map(|x| (i32::from(at(&flat, x, 4)) - i32::from(at(&before, x, 4))).abs())
        .sum();
    let bottom_change: i32 = (0..48)
        .map(|x| (i32::from(at(&flat, x, 58)) - i32::from(at(&before, x, 58))).abs())
        .sum();
    assert!(top_change < bottom_change);
}

#[test]
fn a_curved_path_bends_the_field() {
    let field = MotionField::new(
        &[BlurPath {
            points: vec![(0.1, 0.5), (0.5, 0.1), (0.9, 0.5)],
            start_speed: 10.0,
            end_speed: 10.0,
        }],
        1.0,
        100,
        100,
    );
    let (lx, ly) = field.at(20.0, 40.0);
    let (rx, ry) = field.at(80.0, 40.0);
    // Up-right on the rising side, down-right on the falling side.
    assert!(lx > 0.0 && ly < 0.0, "{lx} {ly}");
    assert!(rx > 0.0 && ry > 0.0, "{rx} {ry}");
}

#[test]
fn end_point_speeds_interpolate_along_the_path() {
    let field = MotionField::new(&[line((0.1, 0.5), (0.9, 0.5), 0.0)], 1.0, 100, 10);
    assert_eq!(field.at(50.0, 5.0), (0.0, 0.0));
    let ramp = BlurPath {
        start_speed: 0.0,
        end_speed: 40.0,
        ..line((0.1, 0.5), (0.9, 0.5), 0.0)
    };
    let field = MotionField::new(&[ramp], 1.0, 100, 10);
    let (slow, _) = field.at(20.0, 5.0);
    let (fast, _) = field.at(80.0, 5.0);
    assert!(slow < fast && fast <= 40.0, "{slow} {fast}");
}

#[test]
fn the_alpha_plane_smears_too() {
    let mut picture = planar(40, 8, 4, |x, _, c| {
        if c == 3 {
            if x < 20 {
                255
            } else {
                0
            }
        } else {
            128
        }
    });
    blur(&mut picture, &[rightward(12.0)], &motion());
    let n = 40 * 8;
    let alpha = |x: usize| picture.data[3 * n + 4 * 40 + x];
    assert!(alpha(21) > 0 && alpha(18) < 255);
}

#[test]
fn equal_inputs_give_identical_output() {
    let paths = [BlurPath {
        points: vec![(0.1, 0.8), (0.4, 0.3), (0.9, 0.6)],
        start_speed: 6.0,
        end_speed: 30.0,
    }];
    let motion = PathMotion {
        taper: 30.0,
        strobe_strength: 40.0,
        strobe_flashes: 3,
        ..motion()
    };
    let noise = BlurNoise {
        enabled: true,
        amount: 20.0,
        ..BlurNoise::default()
    };
    let mut a = stripes(48, 32);
    let mut b = stripes(48, 32);
    path_blur(&mut a, &paths, &motion, &noise).unwrap();
    path_blur(&mut b, &paths, &motion, &noise).unwrap();
    assert_eq!(a.data, b.data);
}

#[test]
fn invalid_values_are_rejected_before_any_write() {
    let picture = stripes(16, 8);
    let bad_paths = [
        BlurPath {
            points: vec![(0.5, 0.5)],
            start_speed: 10.0,
            end_speed: 10.0,
        },
        line((f64::NAN, 0.5), (0.7, 0.5), 10.0),
        line((0.3, 0.5), (0.7, 0.5), MAX_END_POINT_SPEED + 1.0),
        BlurPath {
            points: vec![(0.5, 0.5); MAX_PATH_POINTS + 1],
            start_speed: 1.0,
            end_speed: 1.0,
        },
    ];
    for path in bad_paths {
        let mut buf = picture.clone();
        let result = path_blur(&mut buf, &[path], &motion(), &BlurNoise::default());
        assert!(matches!(result, Err(FilterError::InvalidParams(_))));
        assert_eq!(buf.data, picture.data);
    }
    let bad_motion = [
        PathMotion {
            speed: MAX_PATH_SPEED + 1.0,
            ..motion()
        },
        PathMotion {
            taper: -1.0,
            ..motion()
        },
        PathMotion {
            strobe_strength: 101.0,
            ..motion()
        },
        PathMotion {
            strobe_flashes: 0,
            ..motion()
        },
    ];
    for motion in bad_motion {
        let mut buf = picture.clone();
        let result = path_blur(&mut buf, &[rightward(5.0)], &motion, &BlurNoise::default());
        assert!(matches!(result, Err(FilterError::InvalidParams(_))));
        assert_eq!(buf.data, picture.data);
    }
    let too_many = vec![rightward(5.0); MAX_FIELD_PINS + 1];
    let mut buf = picture.clone();
    assert!(path_blur(&mut buf, &too_many, &motion(), &BlurNoise::default()).is_err());
}

#[test]
fn a_tiny_buffer_and_coincident_points_are_safe() {
    let mut one = planar(1, 1, 3, |_, _, _| 90);
    blur(&mut one, &[rightward(50.0)], &motion());
    assert_eq!(one.data[0], 90);
    let mut picture = stripes(16, 8);
    blur(
        &mut picture,
        &[line((0.5, 0.5), (0.5, 0.5), 30.0)],
        &motion(),
    );
}

#[test]
fn noise_grains_the_whole_moving_picture() {
    let noise = BlurNoise {
        enabled: true,
        amount: 30.0,
        ..BlurNoise::default()
    };
    let mut clean = rows(48, 32);
    let mut grainy = rows(48, 32);
    // The rows picture survives a horizontal blur, so any change is grain.
    path_blur(
        &mut clean,
        &[rightward(12.0)],
        &motion(),
        &BlurNoise::default(),
    )
    .unwrap();
    path_blur(&mut grainy, &[rightward(12.0)], &motion(), &noise).unwrap();
    let changed = clean
        .data
        .iter()
        .zip(grainy.data.iter())
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        changed > clean.data.len() * 3 / 4,
        "{changed} of {}",
        clean.data.len()
    );
}
