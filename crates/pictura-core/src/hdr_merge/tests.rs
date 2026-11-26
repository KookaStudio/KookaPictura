use super::*;

fn gray(value: u8) -> [u8; 4] {
    [value, value, value, 255]
}

fn bracket<'a>(frames: [&'a [[u8; 4]]; 3], ev100: [f64; 3]) -> [Exposure<'a>; 3] {
    std::array::from_fn(|i| Exposure { rgba: frames[i], ev100: ev100[i] })
}

#[test]
fn recovers_scene_radiance() {
    let scene = [0.05_f64, 0.2, 1.5];
    let encode = |linear: f64| {
        let linear = linear.clamp(0.0, 1.0);
        let encoded = if linear <= 0.0031308 { 12.92 * linear } else { 1.055 * linear.powf(1.0 / 2.4) - 0.055 };
        (encoded * 255.0).round() as u8
    };
    let dark = scene.map(|value| gray(encode(value * 0.25)));
    let normal = scene.map(|value| gray(encode(value)));
    let bright = scene.map(|value| gray(encode(value * 4.0)));
    let result = merge(
        &bracket([&bright, &dark, &normal], [-2.0, 2.0, 0.0]),
        3, 1, &MergeOptions { response: ResponseCurve::new([1.0; 3]), ..Default::default() },
    ).expect("valid bracket");
    for plane in &result.planes[..3] {
        for (&actual, expected) in plane.iter().zip(scene) {
            assert!((f64::from(actual) - expected).abs() < 0.015);
        }
        assert!(plane[0] < plane[1] && plane[1] < plane[2] && plane[2] > 1.0);
    }
}

#[test]
fn replays_saved_curve() {
    let frames = [vec![gray(70); 32], vec![gray(110); 32], vec![gray(163); 32]];
    let inputs = bracket([&frames[0], &frames[1], &frames[2]], [1.0, 0.0, -1.0]);
    let result = merge(&inputs, 8, 4, &MergeOptions::default()).unwrap();
    assert_ne!(result.response.exponents(), [1.0; 3]);
    let response = ResponseCurve::new(result.response.exponents()).unwrap();
    let replay = merge(
        &inputs, 8, 4,
        &MergeOptions { response: Some(response), ..Default::default() },
    ).unwrap();
    assert_eq!(result.planes, replay.planes);
    assert!(ResponseCurve::new([1.0, f64::NAN, 1.0]).is_none());
}

fn translated_pattern(delta: usize) -> Vec<[u8; 4]> {
    let mut pixels = vec![[0; 4]; 96 * 64];
    for y in 0..64 {
        for x in delta..96 {
            let x0 = x - delta;
            let index = x0 as u64 + 96 * y as u64;
            let seed = index.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let seed = (seed ^ (seed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            let seed = (seed ^ (seed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            pixels[y * 96 + x] = gray(35 + ((seed ^ (seed >> 31)) % 140) as u8);
        }
    }
    pixels
}

#[test]
fn recovers_translation() {
    let base = translated_pattern(0);
    let moved = translated_pattern(9);
    assert_eq!(registration::offset(&base, &moved, 96, 64), Some((9, 0)));
    let result = merge(
        &bracket([&moved, &base, &base], [1.0, 0.0, 0.0]),
        96,
        64,
        &MergeOptions { align: true, response: ResponseCurve::new([1.0; 3]), ..Default::default() },
    ).unwrap();
    assert!((f64::from(result.planes[0][95]) - linearize(base[95][0])).abs() < 1e-6);
}

#[test]
fn rejects_failed_alignment() {
    let base = translated_pattern(0);
    let moved = translated_pattern(37);
    assert_eq!(registration::offset(&base, &moved, 96, 64), None);
    let flat = [gray(80); 16];
    let result = merge(
        &bracket([&flat, &flat, &flat], [1.0, 0.0, -1.0]),
        4,
        4,
        &MergeOptions { align: true, ..Default::default() },
    );
    assert!(matches!(result, Err(MergeError::Alignment(0))));
}

#[test]
fn selects_base_on_disagreement() {
    let a = [gray(65)];
    let b = [gray(200)];
    let c = [gray(101)];
    let inputs = bracket([&a, &b, &c], [1.0, 0.0, -1.0]);
    for (index, expected) in [linearize(65) * 2.0, linearize(200)].into_iter().enumerate() {
        let result = merge(
            &inputs,
            1,
            1,
            &MergeOptions { ghost_base: Some(index), ..Default::default() },
        ).unwrap();
        assert!((f64::from(result.planes[0][0]) - expected).abs() < 1e-6);
    }
}

#[test]
fn saturation_and_alpha_fallback() {
    let dark = [[255, 255, 255, 101], gray(0), gray(255), [0; 4]];
    let base = [gray(255), gray(0), [0, 255, 0, 104], [0; 4]];
    let bright = [gray(255), [0, 0, 0, 102], gray(0), [0; 4]];
    let result = merge(
        &bracket([&dark, &base, &bright], [1.0, 0.0, -1.0]),
        4, 1, &MergeOptions::default(),
    ).unwrap();
    assert_eq!(result.planes[0], [2.0, 0.0, 0.0, 0.0]);
    assert_eq!(result.planes[1][2], 1.0);
    assert_eq!(result.planes[3], [1.0, 1.0, 104.0 / 255.0, 0.0]);
}

#[test]
fn rejects_memory_overflow() {
    assert_eq!(merge_working_bytes(u32::MAX, u32::MAX, 3), None);
    assert_eq!(merge_working_bytes(10, 20, 3), Some(21_600));
}

#[test]
fn fuses_unclipped_channels_in_saturated_colors() {
    let dark = [[100, 40, 0, 255]];
    let middle = [[137, 58, 0, 255]];
    let bright = [[188, 80, 0, 255]];
    let result = merge(&bracket([&dark, &middle, &bright], [1.0, 0.0, -1.0]), 1, 1,
        &MergeOptions { response: ResponseCurve::new([1.0; 3]), ..Default::default() }).unwrap();
    assert!((result.planes[0][0] - 0.25).abs() < 0.015);
    assert!((result.planes[1][0] - 0.04).abs() < 0.01);
    assert_eq!(result.planes[2][0], 0.0);
}

#[test]
fn rejects_ambiguous_periodic_and_smooth_registration() {
    let periodic: Vec<_> = (0..96 * 64).map(|i| gray(if (i % 96 / 4) % 2 == 0 { 40 } else { 180 })).collect();
    assert_eq!(registration::offset(&periodic, &periodic, 96, 64), None);
    let smooth: Vec<_> = (0..96 * 64).map(|i| gray((i % 96 * 2) as u8)).collect();
    assert_eq!(registration::offset(&smooth, &smooth, 96, 64), None);
}
