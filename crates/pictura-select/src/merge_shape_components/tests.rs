use super::*;
use pictura_core::{path::VectorPath, shape::path_coverage};

fn rect(x: f64, y: f64, w: f64, h: f64, reverse: bool) -> Subpath {
    let mut points = vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    if reverse {
        points.reverse();
    }
    Subpath {
        points: points
            .into_iter()
            .map(|anchor| PathPoint {
                anchor,
                in_handle: None,
                out_handle: None,
                smooth: false,
            })
            .collect(),
        closed: true,
    }
}

fn winding_coverage(paths: &[Subpath]) -> Vec<u8> {
    let mut path = VectorPath::default();
    path.subpaths = paths.to_vec();
    path_coverage(&path, 40, 32)
}

#[test]
fn merges_nonzero_overlap_and_keeps_hole_and_curve() {
    let mut independent = rect(28.0, 5.0, 6.0, 6.0, false);
    independent.points[0].out_handle = Some((31.0, 2.0));
    let input = [
        rect(1.0, 1.0, 12.0, 12.0, false),
        rect(8.0, 8.0, 12.0, 12.0, false),
        rect(3.0, 3.0, 3.0, 3.0, true),
        independent.clone(),
    ];
    let merged = merge_shape_components(&input, VectorFillRule::NonZero, None).unwrap();
    assert_eq!(merged.len(), 3);
    assert_eq!(merged.last(), Some(&independent));
    assert_eq!(winding_coverage(&input), winding_coverage(&merged));
    assert!(merge_shape_components(&merged, VectorFillRule::NonZero, None).is_none());
}

#[test]
fn preserves_nested_ambient_winding_and_parity() {
    let input = [
        rect(1.0, 1.0, 25.0, 20.0, false),
        rect(4.0, 4.0, 10.0, 8.0, true),
        rect(8.0, 7.0, 9.0, 8.0, true),
    ];
    let merged = merge_shape_components(&input, VectorFillRule::NonZero, None).unwrap();
    let coverage = winding_coverage(&merged);
    assert_eq!(coverage, winding_coverage(&input));
    assert_eq!(coverage[5 * 40 + 6], 0);
    assert_eq!(coverage[8 * 40 + 10], 255);

    let even_odd = merge_shape_components(&input, VectorFillRule::EvenOdd, None).unwrap();
    assert_eq!(
        pictura_core::shape::paths_coverage(&input, 40, 32),
        pictura_core::shape::paths_coverage(&even_odd, 40, 32),
    );
    let polygons = |paths: &[Subpath]| -> Vec<Vec<Vertex>> {
        paths
            .iter()
            .map(|path| {
                path.flatten(CURVE_TOLERANCE)
                    .into_iter()
                    .map(|(x, y)| [x, y])
                    .collect()
            })
            .collect()
    };
    assert!(compare::fills_match(
        &polygons(&input),
        &polygons(&even_odd),
        FillRule::EvenOdd
    ));
}

#[test]
fn preserves_even_odd_exclusion() {
    let input = [
        rect(1.0, 1.0, 12.0, 12.0, false),
        rect(8.0, 8.0, 12.0, 12.0, false),
    ];
    let merged = merge_shape_components(&input, VectorFillRule::EvenOdd, None).unwrap();
    let before = pictura_core::shape::paths_coverage(&input, 24, 24);
    assert_eq!(before, pictura_core::shape::paths_coverage(&merged, 24, 24));
    assert_eq!(before[9 * 24 + 9], 0);
    assert_eq!(before[9 * 24 + 2], 255);
}

#[test]
fn leaves_disconnected_and_over_budget_inputs_alone() {
    let a = rect(0.0, 0.0, 10.0, 10.0, false);
    let b = rect(20.0, 0.0, 10.0, 10.0, false);
    assert!(merge_shape_components(&[a.clone(), b], VectorFillRule::NonZero, None).is_none());
    let many = vec![a; MAX_CONTOURS + 1];
    assert!(merge_shape_components(&many, VectorFillRule::NonZero, None).is_none());
}

#[test]
fn preserves_curved_work_path_fill_at_the_coverage_tolerance() {
    let mut curved = rect(2.0, 4.0, 12.0, 12.0, false);
    curved.points[0].out_handle = Some((6.0, 0.0));
    curved.points[1].in_handle = Some((10.0, 0.0));
    let input = [curved, rect(8.0, 4.0, 12.0, 15.0, false)];
    let merged = merge_shape_components(&input, VectorFillRule::NonZero, None).unwrap();
    assert_eq!(merged.len(), 1);
    assert_eq!(winding_coverage(&input), winding_coverage(&merged));
}

#[test]
fn compares_effective_boundaries_and_topology() {
    let a = vec![
        vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
        vec![[5.0, 0.0], [15.0, 0.0], [15.0, 10.0], [5.0, 10.0]],
    ];
    let union = vec![vec![[0.0, 0.0], [15.0, 0.0], [15.0, 10.0], [0.0, 10.0]]];
    assert!(compare::fills_match(&a, &union, FillRule::NonZero));
    let shifted = vec![vec![[0.0, 0.0], [15.0, 0.0], [15.0, 10.02], [0.0, 10.02]]];
    assert!(!compare::fills_match(&a, &shifted, FillRule::NonZero));
    let outer = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let hole = vec![[9.0, 9.0], [9.0, 11.0], [11.0, 11.0], [11.0, 9.0]];
    assert!(!compare::fills_match(
        &[outer.clone(), hole],
        &[outer],
        FillRule::EvenOdd
    ));
}

#[test]
fn accepts_cancelled_work_path_fill() {
    let shape = rect(1.0, 1.0, 12.0, 12.0, false);
    let reversed = rect(1.0, 1.0, 12.0, 12.0, true);
    let cancelled =
        merge_shape_components(&[shape.clone(), reversed], VectorFillRule::NonZero, None);
    assert_eq!(cancelled, Some(Vec::new()));
    let cancelled = merge_shape_components(&[shape.clone(), shape], VectorFillRule::EvenOdd, None);
    assert_eq!(cancelled, Some(Vec::new()));
}
