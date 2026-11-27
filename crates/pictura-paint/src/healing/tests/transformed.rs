use super::*;

#[test]
fn transformed_healing_matches_explicitly_mirrored_source() {
    let mut original = flat(80, 40, BASE);
    for y in 0..40 {
        for x in 0..40 {
            let v = ((x * x + y * 7) % 100) as u8;
            original.set(x, y, [v, v, v, 255]);
        }
    }
    let mut mirrored = original.clone();
    for y in 0..40 {
        for x in 1..40 {
            mirrored.set(x, y, original.get(40 - x, y));
        }
    }
    let region = PsdRect { left: 46, top: 16, right: 54, bottom: 24 };
    let coverage = all_ones(region);
    let mut transformed = original.clone();
    let transform = crate::stamp::SourceTransform {
        scale_x: -1.0, scale_y: 1.0, angle_deg: 0.0,
    };
    assert!(clone_region_transformed(&mut transformed, region, &coverage, (-30, 0),
        (50.5, 20.5), transform, Transfer::Full).is_some());
    assert!(clone_region(&mut mirrored, region, &coverage, (-30, 0), Transfer::Full).is_some());
    for y in region.top..region.bottom {
        for x in region.left..region.right {
            assert_eq!(transformed.get(x, y), mirrored.get(x, y), "at ({x}, {y})");
        }
    }
    assert_ne!(transformed.get(50, 20), original.get(50, 20));
}

#[test]
fn transformed_healing_uses_destination_when_source_is_outside_layer() {
    let original = flat(32, 32, BASE);
    let region = PsdRect { left: 12, top: 12, right: 20, bottom: 20 };
    for scale_x in [1.0, -1.0] {
        let mut img = original.clone();
        let transform = crate::stamp::SourceTransform { scale_x, scale_y: 1.0, angle_deg: 0.0 };
        assert!(clone_region_transformed(&mut img, region, &all_ones(region),
            (-100, 0), (16.5, 16.5), transform, Transfer::Full).is_some());
        assert_eq!(img, original);
    }
}
