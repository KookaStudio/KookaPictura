use super::*;
use crate::{assert_no_materialization, LayerMask};

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        left,
        top,
        right,
        bottom,
    }
}

#[test]
fn coverage_samples_document_coordinates() {
    for (left, top) in [(-3, -2), (4, 7)] {
        for outside in [0, 128, 255] {
            let coverage = Coverage::new(
                rect(left, top, left + 3, top + 2),
                vec![0, 1, 64, 128, 254, 255].into(),
                outside,
            )
            .unwrap();
            assert_eq!(coverage.sample(left + 2, top + 1), 255);
            assert_eq!(coverage.sample(left, top + 1), 128);
            for (x, y) in [
                (left - 1, top),
                (left + 3, top),
                (left, top - 1),
                (left, top + 2),
                (i32::MIN, i32::MAX),
            ] {
                assert_eq!(coverage.sample(x, y), outside);
            }
        }
    }
}

#[test]
fn coverage_rows_match_samples() {
    let data: Vec<u8> = (0..600 * 3).map(|i| (i % 251) as u8).collect();
    let mut tiled: Plane<u8> = data.clone().into();
    assert!(tiled.tile(600, 3));
    for plane in [data.into(), tiled] {
        let coverage = Coverage::new(rect(-11, 7, 589, 10), plane, 128).unwrap();
        assert_no_materialization(|| {
            for y in [6, 7, 8, 10] {
                let mut row = vec![17; 620];
                coverage.read_row(-20, y, &mut row);
                for (i, value) in row.iter().enumerate() {
                    assert_eq!(*value, coverage.sample(-20 + i as i32, y));
                }
            }
        });
    }
}

#[test]
fn coverage_rebase_shares_storage() {
    let plane = Plane::filled(600, 530, 1, 128);
    let coverage = Coverage::new(rect(0, 0, 600, 530), plane, 0).unwrap();
    assert_no_materialization(|| {
        for (x, y) in [(40, -12), (-16, 20)] {
            let local = coverage.rebased(x, y).unwrap();
            assert_eq!(local.sample(0, 0), coverage.sample(x, y));
            assert_eq!(local.rect(), rect(-x, -y, 600 - x, 530 - y));
            assert!(local
                .data
                .as_ref()
                .unwrap()
                .shares_tile(0, coverage.data.as_ref().unwrap()));
        }
    });
    assert_eq!(coverage.rect(), rect(0, 0, 600, 530));
}

#[test]
fn coverage_invalid_geometry_is_refused() {
    assert!(Coverage::new(rect(0, 0, 2, 2), vec![0; 3].into(), 0).is_none());
    assert!(Coverage::new(rect(0, 0, 2, 2), Plane::filled(4, 1, 1, 0), 0).is_none());
    assert!(Coverage::new(rect(2, 0, 0, 2), Plane::default(), 0).is_none());
    let coverage = Coverage::new(rect(i32::MIN, 0, i32::MIN + 1, 1), vec![64].into(), 0).unwrap();
    assert!(coverage.rebased(1, 0).is_none());
    assert_eq!(coverage.sample(i32::MIN, 0), 64);
    assert!(Coverage::new(rect(0, 0, 0, 0), Plane::default(), 0).is_some());
    assert_eq!(Coverage::constant(255).sample(i32::MAX, i32::MIN), 255);
}

#[test]
fn coverage_uniform_regions_preserve_tiles() {
    let tiled = Plane::filled(600, 530, 1, 64);
    let flat: Plane<u8> = vec![64; 600 * 530].into();
    for plane in [flat, tiled] {
        let coverage = Coverage::new(rect(10, 20, 610, 550), plane, 0).unwrap();
        assert_no_materialization(|| {
            assert_eq!(coverage.uniform(rect(10, 20, 300, 300)), Some(64));
            assert_eq!(coverage.uniform(rect(-20, -20, 0, 0)), Some(0));
            assert_eq!(coverage.uniform(rect(0, 0, 20, 30)), None);
            assert_eq!(
                coverage.covered_bounds(rect(0, 0, 800, 800)),
                Some(coverage.rect())
            );
        });
    }
    assert_eq!(
        Coverage::constant(255).covered_bounds(rect(-3, 2, 7, 9)),
        Some(rect(-3, 2, 7, 9))
    );
    assert_eq!(
        Coverage::constant(0).covered_bounds(rect(-3, 2, 7, 9)),
        None
    );
}

#[test]
fn raw_mask_projection_preserves_samples() {
    let mask = LayerMask {
        rect: rect(-1, 2, 2, 4),
        default_color: 128,
        data: Some(vec![0, 1, 64, 128, 254, 255].into()),
        density: 17,
        disabled: true,
        flags: 7,
        ..Default::default()
    };
    let before = mask.clone();
    let coverage = mask.raw_coverage().unwrap();
    for (i, value) in [0, 1, 64, 128, 254, 255].into_iter().enumerate() {
        assert_eq!(
            coverage.sample(-1 + (i % 3) as i32, 2 + (i / 3) as i32),
            value
        );
    }
    assert_eq!(coverage.sample(-1, 2), 0);
    assert_eq!(coverage.sample(1, 3), 255);
    assert_eq!(coverage.sample(2, 3), 128);
    assert_eq!(mask, before);
    let missing = LayerMask { data: None, ..mask };
    assert_eq!(missing.raw_coverage().unwrap().rect(), missing.rect);
    assert_eq!(missing.raw_coverage().unwrap().sample(-1, 2), 128);
}
