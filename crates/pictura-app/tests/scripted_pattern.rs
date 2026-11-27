use pictura_paint::bucket::scripted_pattern_at_center;

fn scripted_pattern_at(tile: &RgbaImage, script: i32, x: i32, y: i32) -> pictura_paint::Rgba {
    scripted_pattern_at_center(tile, script, x, y, (32, 32))
}
use pictura_paint::healing::RgbaImage;

#[test]
fn scripted_pattern_geometry_and_determinism() {
    let tile = RgbaImage {
        width: 3,
        height: 2,
        data: vec![
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 0, 255],
            [0, 255, 255, 255],
            [255, 0, 255, 255],
        ],
    };
    let render = |script| {
        (-32..32)
            .flat_map(|y| (-32..32).map(move |x| (x, y)))
            .map(|(x, y)| scripted_pattern_at(&tile, script, x, y))
            .collect::<Vec<_>>()
    };
    let layouts: Vec<_> = (1..=5).map(render).collect();
    // Adjacent brick rows have the same tile shifted by half its width.
    assert_eq!(
        scripted_pattern_at(&tile, 1, 0, 2),
        scripted_pattern_at(&tile, 1, 1, 0)
    );
    assert_ne!(
        scripted_pattern_at(&tile, 1, 0, 2),
        scripted_pattern_at(&tile, 1, 0, 0)
    );
    // Every source pixel survives rotation in the neighboring square cell.
    for u in 0..3 {
        for v in 0..2 {
            assert_eq!(
                scripted_pattern_at(&tile, 2, 3 + 1 - v, u),
                scripted_pattern_at(&tile, 2, u, v)
            );
        }
    }
    assert_eq!(scripted_pattern_at(&tile, 2, 5, 0).a, 0);
    // Scatter and spiral include both stamps and gaps, including away from the origin.
    for script in [3, 4] {
        let samples: Vec<_> = (-32_i32..32)
            .flat_map(|y| (-32_i32..32).map(move |x| (x, y)))
            .filter(|(x, y)| x.abs() >= 12 || y.abs() >= 12)
            .map(|(x, y)| scripted_pattern_at(&tile, script, x, y).a)
            .collect();
        assert!(samples.contains(&0), "script {script} has gaps");
        assert!(
            samples.contains(&255),
            "script {script} places tiles away from the origin"
        );
    }
    for y in 16..32 {
        for x in 16..32 {
            assert_eq!(
                scripted_pattern_at(&tile, 5, x, y),
                scripted_pattern_at(&tile, 5, 63 - x, y)
            );
            assert_eq!(
                scripted_pattern_at(&tile, 5, x, y),
                scripted_pattern_at(&tile, 5, x, 63 - y)
            );
        }
    }
    assert_ne!(
        scripted_pattern_at(&tile, 5, 32, 17),
        scripted_pattern_at(&tile, 0, 32, 17)
    );
    for (index, layout) in layouts.iter().enumerate() {
        assert_eq!(
            *layout,
            render(index as i32 + 1),
            "script must be deterministic"
        );
        for previous in &layouts[..index] {
            assert_ne!(layout, previous, "scripts must produce distinct layouts");
        }
    }
}

#[test]
fn scripted_pattern_bounds() {
    use pictura_paint::bucket::scripted_pattern_work_admitted;
    assert!(scripted_pattern_work_admitted(1, 1));
    assert!(scripted_pattern_work_admitted(8192, 8192));
    assert!(!scripted_pattern_work_admitted(8193, 8192));
    assert!(!scripted_pattern_work_admitted(0, 1));
    assert!(!scripted_pattern_work_admitted(i32::MAX, i32::MAX));
}
