use super::*;

fn stop(location: u16, v: u8) -> Stop {
    Stop {
        location,
        rgba: [v, v, v, 255],
    }
}

fn decode(block: &[u8]) -> GradientMapParams {
    match from_block(block).and_then(|d| pictura_render::decode_adjustment(&d)) {
        Some(Adjustment::GradientMap(p)) => p,
        other => panic!("not a gradient map: {other:?}"),
    }
}

#[test]
fn stops_round_trip_through_their_text() {
    let stops = vec![
        Stop {
            location: 0,
            rgba: [255, 0, 16, 255],
        },
        Stop {
            location: 4096,
            rgba: [255, 255, 255, 128],
        },
    ];
    let text = format_stops(&stops);
    assert_eq!(text, "0:ff0010 4096:ffffff80");
    assert_eq!(parse_stops(&text), Some(stops));
    assert_eq!(parse_stops("0:fff 4096:ffffff"), None);
    assert_eq!(parse_stops("4097:000000"), None);
}

#[test]
fn spread_sorts_and_separates_coincident_stops() {
    let mut stops = vec![stop(4096, 3), stop(2048, 1), stop(2048, 2), stop(4096, 4)];
    assert!(spread(&mut stops));
    let locations: Vec<u16> = stops.iter().map(|s| s.location).collect();
    assert_eq!(locations, [2048, 2049, 4095, 4096]);
    assert_eq!(
        stops[0].rgba[0], 1,
        "a stable sort keeps a hard edge's order"
    );
    assert!(!spread(&mut [stop(0, 0)]));
}

#[test]
fn every_preset_makes_a_valid_gradient_map() {
    for index in 0..gradient::PRESET_NAMES.len() as i32 {
        let stops = gradient_preset_map_stops(index, 0xff00_0000, 0xffff_ffff);
        let block = gradient_map_block(&stops, 0, false, false);
        assert!(!block.is_empty(), "preset {index}");
    }
    assert!(gradient_preset_map_stops(99, 0, 0).is_empty());
    // Foreground to Transparent keeps its fade as opacity stops.
    let fade = gradient_preset_map_stops(1, 0xff00_0000, 0xffff_ffff);
    let p = decode(&gradient_map_block(&fade, 0, false, false));
    assert_eq!(p.transparency.first().map(|s| s.opacity), Some(100));
    assert_eq!(p.transparency.last().map(|s| s.opacity), Some(0));
    let opaque = gradient_preset_map_stops(2, 0, 0);
    assert!(decode(&gradient_map_block(&opaque, 0, true, true))
        .transparency
        .is_empty());
    assert_eq!(
        gradient_map_stops(&gradient_map_block(&opaque, 0, false, false)),
        opaque
    );
}

#[test]
fn smoothness_curves_through_the_stops_and_keeps_two_stops_straight() {
    let two = [stop(0, 0), stop(4096, 255)];
    assert_eq!(smoothed(&two, 100), two.to_vec());
    let three = [stop(0, 0), stop(1024, 200), stop(4096, 255)];
    assert_eq!(smoothed(&three, 0), three.to_vec());
    let curve = smoothed(&three, 100);
    assert_eq!(curve.len(), 129);
    assert_eq!(curve[32].rgba[0], 200, "the curve passes through each stop");
    let linear_at_512 = 100;
    assert!(
        curve[16].rgba[0] > linear_at_512,
        "it bows above the straight line"
    );
    assert_eq!(curve[128].rgba[0], 255);
}

#[test]
fn noise_is_reproducible_and_stays_inside_its_ranges() {
    let full = [(0, 100); 3];
    let a = noise_stops(7, 50, 0, full, false, false);
    assert_eq!(a, noise_stops(7, 50, 0, full, false, false));
    assert_ne!(a, noise_stops(8, 50, 0, full, false, false));
    assert_eq!(a.len(), 3 + 50 * 61 / 100);
    assert!(
        noise_stops(7, 100, 0, full, false, false).len()
            > noise_stops(7, 0, 0, full, false, false).len()
    );
    assert!(a.windows(2).all(|w| w[0].location < w[1].location));
    assert!(a.iter().all(|s| s.rgba[3] == 255));
    // Red pinned to its top quarter, blue to none.
    let red = noise_stops(3, 80, 0, [(75, 100), (0, 100), (0, 0)], false, false);
    assert!(red.iter().all(|s| s.rgba[0] >= 191 && s.rgba[2] == 0));
    let restricted = noise_stops(3, 80, 1, [(0, 100), (100, 100), (100, 100)], true, false);
    assert!(restricted.iter().all(|s| {
        let max = s.rgba[..3].iter().max().copied().unwrap_or(0) as f64;
        let min = s.rgba[..3].iter().min().copied().unwrap_or(0) as f64;
        (max - min) / max <= 0.81
    }));
    assert!(noise_stops(3, 80, 0, full, false, true)
        .iter()
        .any(|s| s.rgba[3] < 255));
    let lab = noise_stops(5, 30, 2, full, false, false);
    let block = gradient_map_block(&QString::from(format_stops(&lab).as_str()), 0, false, false);
    assert!(!block.is_empty());
}

#[test]
fn a_block_reads_back_its_opacity_reverse_and_dither() {
    let text = QString::from("0:ff000080 2048:00ff00 4096:0000ff00");
    let block = gradient_map_block(&text, 0, true, true);
    assert_eq!(
        gradient_map_stops(&block).to_string(),
        "0:ff000080 2048:00ff00 4096:0000ff00"
    );
    assert!(gradient_map_reverse(&block));
    assert!(gradient_map_dither(&block));
    let plain = gradient_map_block(&QString::from("0:000000 4096:ffffff"), 0, false, false);
    assert!(!gradient_map_reverse(&plain) && !gradient_map_dither(&plain));
    assert!(!gradient_map_dither(b"levl\0\0\0\0"));
}

#[test]
fn an_opacity_stop_between_colour_stops_gets_its_own_stop() {
    let p = GradientMapParams {
        stops: vec![
            GradientStop {
                location: 0,
                color: [0, 0, 0],
            },
            GradientStop {
                location: 4096,
                color: [200, 100, 0],
            },
        ],
        reverse: false,
        transparency: vec![
            OpacityStop {
                location: 0,
                opacity: 100,
            },
            OpacityStop {
                location: 2048,
                opacity: 0,
            },
            OpacityStop {
                location: 4096,
                opacity: 100,
            },
        ],
    };
    assert_eq!(
        format_stops(&readback_stops(&p)),
        "0:000000 2048:64320000 4096:c86400"
    );
}
