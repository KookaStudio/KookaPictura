use super::*;

fn polyline(points: &[(f64, f64)]) -> VectorPath {
    let mut p = VectorPath::default();
    for &(x, y) in points {
        p.append_corner(x, y);
    }
    p
}

#[test]
fn appending_corners_builds_a_polyline() {
    let p = polyline(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
    assert_eq!(p.subpaths.len(), 1);
    assert!(!p.subpaths[0].closed);
    let flat = p.flatten(0.1);
    assert_eq!(flat[0].0, vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
    assert_eq!(p.last_anchor(), Some((10.0, 10.0)));
}

#[test]
fn a_short_drag_stays_a_corner() {
    let mut p = polyline(&[(0.0, 0.0)]);
    p.update_last_handle(0.3, 0.2, false);
    let pt = p.subpaths[0].points[0];
    assert!(!pt.smooth && pt.out_handle.is_none() && pt.in_handle.is_none());
}

#[test]
fn dragging_places_symmetric_handles() {
    let mut p = polyline(&[(0.0, 0.0)]);
    p.update_last_handle(10.0, 0.0, false);
    let pt = p.subpaths[0].points[0];
    assert!(pt.smooth);
    assert_eq!(pt.out_handle, Some((10.0, 0.0)));
    assert_eq!(pt.in_handle, Some((-10.0, 0.0)));
}

#[test]
fn alt_drag_while_placing_leaves_the_in_handle_alone() {
    let mut p = polyline(&[(0.0, 0.0)]);
    p.update_last_handle(10.0, 0.0, true);
    let pt = p.subpaths[0].points[0];
    assert!(!pt.smooth);
    assert_eq!(pt.out_handle, Some((10.0, 0.0)));
    assert_eq!(pt.in_handle, None);
}

#[test]
fn closing_needs_two_points_and_ends_the_session() {
    let mut p = polyline(&[(0.0, 0.0)]);
    assert!(!p.close_active_subpath());
    p.append_corner(10.0, 0.0);
    assert!(p.close_active_subpath());
    assert!(p.subpaths[0].closed);
    assert_eq!(p.editing_subpath(), None);
}

#[test]
fn finishing_leaves_the_subpath_open_and_the_next_anchor_starts_another() {
    let mut p = polyline(&[(0.0, 0.0), (10.0, 0.0)]);
    p.finish_editing();
    assert!(!p.subpaths[0].closed);
    p.append_corner(50.0, 50.0);
    assert_eq!(p.subpaths.len(), 2);
    assert_eq!(p.editing_subpath(), Some(1));
}

#[test]
fn resuming_from_the_first_point_reverses_the_subpath() {
    let mut p = polyline(&[(0.0, 0.0), (10.0, 0.0)]);
    p.update_last_handle(10.0, 5.0, false);
    p.finish_editing();
    assert!(!p.resume_at(0, 5), "an out-of-range point resumed");
    assert!(p.resume_at(0, 0));
    let pts = &p.subpaths[0].points;
    assert_eq!(pts[0].anchor, (10.0, 0.0));
    assert_eq!(pts[0].out_handle, Some((10.0, -5.0)), "handles not swapped");
    p.append_corner(-10.0, 0.0);
    assert_eq!(p.subpaths[0].points.len(), 3);
    assert_eq!(p.subpaths.len(), 1);
}

#[test]
fn resuming_is_refused_on_a_closed_subpath_or_interior_point() {
    let mut p = polyline(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
    p.finish_editing();
    assert!(!p.resume_at(0, 1));
    assert!(p.resume_at(0, 2));
    assert!(p.close_active_subpath());
    assert!(!p.resume_at(0, 0));
}

#[test]
fn a_restored_session_extends_only_an_open_subpath() {
    let p = polyline(&[(0.0, 0.0), (10.0, 0.0)]);
    let mut rebuilt = VectorPath::default();
    rebuilt.add_subpath(p.subpaths[0].clone());
    rebuilt.set_editing_subpath(p.editing_subpath());
    rebuilt.append_corner(10.0, 10.0);
    assert_eq!(rebuilt.subpaths.len(), 1);
    assert_eq!(rebuilt.subpaths[0].points.len(), 3);
    rebuilt.set_editing_subpath(Some(4));
    assert_eq!(rebuilt.editing_subpath(), None, "an out-of-range subpath");
    rebuilt.subpaths[0].closed = true;
    rebuilt.set_editing_subpath(Some(0));
    assert_eq!(rebuilt.editing_subpath(), None, "a closed subpath");
}

#[test]
fn dragging_one_handle_of_a_smooth_point_mirrors_the_angle_not_the_length() {
    let mut p = polyline(&[(0.0, 0.0)]);
    p.update_last_handle(10.0, 0.0, false);
    p.move_handle(0, 0, HandleSide::In, -4.0, 0.0, false);
    p.move_handle(0, 0, HandleSide::Out, 0.0, 10.0, false);
    let pt = p.subpaths[0].points[0];
    let (ix, iy) = pt.in_handle.unwrap();
    assert!(ix.abs() < 1e-9 && iy < 0.0);
    assert!(
        (ix.hypot(iy) - 4.0).abs() < 1e-9,
        "in-handle length changed"
    );
}

#[test]
fn an_independent_handle_drag_breaks_smoothness_for_good() {
    let mut p = polyline(&[(0.0, 0.0)]);
    p.update_last_handle(10.0, 0.0, false);
    p.move_handle(0, 0, HandleSide::Out, 0.0, 10.0, true);
    assert!(!p.subpaths[0].points[0].smooth);
    p.move_handle(0, 0, HandleSide::Out, 0.0, 20.0, false);
    assert_eq!(p.subpaths[0].points[0].in_handle, Some((-10.0, 0.0)));
}

#[test]
fn convert_point_click_strips_handles_once() {
    let mut p = polyline(&[(0.0, 0.0)]);
    p.update_last_handle(10.0, 0.0, false);
    assert!(p.set_corner(0, 0));
    let pt = p.subpaths[0].points[0];
    assert!(!pt.smooth && pt.in_handle.is_none() && pt.out_handle.is_none());
    assert!(!p.set_corner(0, 0), "an unchanged corner reported a change");
}

#[test]
fn convert_point_drag_pulls_symmetric_handles_from_a_corner() {
    let mut p = polyline(&[(0.0, 0.0)]);
    assert!(p.drag_new_handles(0, 0, 8.0, 0.0));
    let pt = p.subpaths[0].points[0];
    assert!(pt.smooth);
    assert_eq!(pt.in_handle, Some((-8.0, 0.0)));
}

#[test]
fn inserting_on_a_straight_segment_stays_straight() {
    let mut p = polyline(&[(0.0, 0.0), (10.0, 0.0)]);
    assert!(p.insert_anchor(0, 0, 0.5));
    let mid = p.subpaths[0].points[1];
    assert_eq!(mid.anchor, (5.0, 0.0));
    assert!(mid.in_handle.is_none() && mid.out_handle.is_none());
}

#[test]
fn inserting_on_a_curve_preserves_its_shape() {
    let mut p = polyline(&[(0.0, 0.0)]);
    p.update_last_handle(0.0, 10.0, false);
    p.append_corner(20.0, 0.0);
    let before = p.subpaths[0].segment(0).unwrap();
    assert!(p.insert_anchor(0, 0, 0.5));
    assert!(p.subpaths[0].points[1].smooth);
    for i in 0..=20 {
        let t = i as f64 / 20.0;
        let c = cubic_at(before, t);
        let (_, d0) = nearest_on_cubic(p.subpaths[0].segment(0).unwrap(), c);
        let (_, d1) = nearest_on_cubic(p.subpaths[0].segment(1).unwrap(), c);
        assert!(d0.min(d1) < 1e-3, "the curve moved at t={t}");
    }
}

#[test]
fn inserting_on_the_closing_segment_appends() {
    let mut p = polyline(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
    p.close_active_subpath();
    assert!(p.insert_anchor(0, 2, 0.5));
    let pts = &p.subpaths[0].points;
    assert_eq!(pts.len(), 4);
    assert_eq!(pts[3].anchor, (5.0, 5.0));
}

#[test]
fn deleting_the_last_anchor_removes_the_subpath_and_renumbers_editing() {
    let mut p = polyline(&[(0.0, 0.0), (1.0, 1.0)]);
    p.finish_editing();
    p.append_corner(9.0, 9.0);
    p.delete_anchor(0, 1);
    p.delete_anchor(0, 0);
    assert_eq!(p.subpaths.len(), 1);
    assert_eq!(p.editing_subpath(), Some(0));
    p.delete_anchor(0, 0);
    assert!(p.is_empty());
    assert_eq!(p.editing_subpath(), None);
}

#[test]
fn deleting_down_to_one_point_reopens_a_closed_subpath() {
    let mut p = polyline(&[(0.0, 0.0), (10.0, 0.0)]);
    p.close_active_subpath();
    p.delete_anchor(0, 1);
    assert!(!p.subpaths[0].closed);
}

#[test]
fn hit_tests_find_the_nearest_within_radius() {
    let mut p = polyline(&[(0.0, 0.0), (100.0, 0.0)]);
    p.update_last_handle(110.0, 0.0, false);
    assert_eq!(p.hit_anchor(2.0, 2.0, 5.0), Some((0, 0)));
    assert_eq!(p.hit_anchor(2.0, 2.0, 1.0), None);
    assert_eq!(p.hit_handle(109.0, 1.0, 3.0), Some((0, 1, HandleSide::Out)));
    let (sp, seg, t) = p.hit_segment(25.0, 1.0, 2.0).expect("no segment hit");
    assert_eq!((sp, seg), (0, 0));
    assert!(p.insert_anchor(sp, seg, t));
    let inserted = p.subpaths[0].points[1].anchor;
    assert!((inserted.0 - 25.0).abs() < 0.5 && inserted.1.abs() < 1e-9);
}

#[test]
fn closed_flattening_includes_the_closing_segment() {
    let mut p = polyline(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
    p.close_active_subpath();
    let flat = p.flatten(0.1);
    assert!(flat[0].1);
    assert_eq!(flat[0].0.last(), Some(&(0.0, 0.0)));
}

#[test]
fn flattening_a_curve_stays_within_tolerance() {
    let q = [(0.0, 0.0), (0.0, 50.0), (100.0, 0.0), (100.0, 0.0)];
    let mut p = polyline(&[q[0]]);
    p.update_last_handle(q[1].0, q[1].1, false);
    p.append_corner(q[3].0, q[3].1);
    let flat = &p.flatten(0.25)[0].0;
    assert!(flat.len() > 4);
    for i in 0..=400 {
        let s = cubic_at(q, i as f64 / 400.0);
        let nearest = flat
            .windows(2)
            .map(|w| distance_to_segment(s, w[0], w[1]))
            .fold(f64::MAX, f64::min);
        assert!(nearest <= 1.0, "{s:?} sat {nearest} from the polyline");
    }
}

#[test]
fn simplify_freehand_drops_wobble_and_keeps_a_corner() {
    let wobble: Vec<_> = (0..=20)
        .map(|i| (i as f64 * 5.0, if i % 2 == 0 { 0.3 } else { -0.3 }))
        .collect();
    let s = simplify_freehand(&wobble, 1.0);
    assert_eq!(s, vec![wobble[0], wobble[20]]);

    let mut l: Vec<_> = (0..=10).map(|i| (i as f64 * 10.0, 0.0)).collect();
    l.extend((1..=10).map(|i| (100.0, i as f64 * 10.0)));
    assert!(simplify_freehand(&l, 2.0).contains(&(100.0, 0.0)));
}

#[test]
fn a_higher_curve_fit_never_keeps_more_anchors() {
    let circle: Vec<_> = (0..=200)
        .map(|i| {
            let a = i as f64 / 200.0 * std::f64::consts::TAU;
            (50.0 + 40.0 * a.cos(), 50.0 + 40.0 * a.sin())
        })
        .collect();
    let counts: Vec<_> = [0.5, 1.0, 2.0, 5.0, 10.0]
        .iter()
        .map(|&t| simplify_freehand(&circle, t).len())
        .collect();
    assert!(counts.windows(2).all(|w| w[0] >= w[1]), "{counts:?}");
    assert!(counts[0] > counts[4]);
}

#[test]
fn freeform_adds_a_new_subpath_and_closes_when_asked() {
    let mut p = polyline(&[(0.0, 0.0), (5.0, 5.0)]);
    let square = [
        (10.0, 10.0),
        (30.0, 10.0),
        (30.0, 30.0),
        (10.0, 30.0),
        (10.5, 10.5),
    ];
    assert!(p.add_freeform(&square, 1.0, true));
    assert_eq!(p.subpaths.len(), 2);
    assert!(p.subpaths[1].closed);
    assert_eq!(p.editing_subpath(), None);
    assert!(!p.add_freeform(&[(1.0, 1.0)], 1.0, false));
    assert!(p.add_freeform(&[(0.0, 50.0), (40.0, 50.0)], 1.0, true));
    assert!(!p.subpaths[2].closed, "a two-point freeform closed");
}

fn closed_square(x: f64, y: f64, size: f64) -> VectorPath {
    let mut p = polyline(&[(x, y), (x + size, y), (x + size, y + size), (x, y + size)]);
    p.close_active_subpath();
    p
}

#[test]
fn moving_an_anchor_carries_its_handles() {
    let mut p = polyline(&[(0.0, 0.0)]);
    p.update_last_handle(10.0, 0.0, false);
    assert!(p.move_anchor(0, 0, 5.0, 5.0));
    let pt = p.subpaths[0].points[0];
    assert_eq!(pt.anchor, (5.0, 5.0));
    assert_eq!(pt.out_handle, Some((15.0, 5.0)));
    assert_eq!(pt.in_handle, Some((-5.0, 5.0)));
    assert!(!p.move_anchor(0, 1, 0.0, 0.0));
}

#[test]
fn moving_a_subpath_shifts_only_that_component() {
    let mut p = polyline(&[(0.0, 0.0)]);
    p.update_last_handle(4.0, 0.0, false);
    p.append_corner(10.0, 0.0);
    p.finish_editing();
    p.append_corner(50.0, 50.0);
    assert!(p.move_subpath(0, 100.0, 0.0));
    assert_eq!(p.subpaths[0].points[0].anchor, (100.0, 0.0));
    assert_eq!(p.subpaths[0].points[0].out_handle, Some((104.0, 0.0)));
    assert_eq!(p.subpaths[0].points[1].anchor, (110.0, 0.0));
    assert_eq!(p.subpaths[1].points[0].anchor, (50.0, 50.0));
    assert!(!p.move_subpath(2, 1.0, 1.0));
}

#[test]
fn duplicating_and_removing_a_subpath() {
    let mut p = closed_square(0.0, 0.0, 10.0);
    assert_eq!(p.duplicate_subpath(0), Some(1));
    assert_eq!(p.subpaths[1], p.subpaths[0]);
    assert_eq!(p.duplicate_subpath(5), None);
    p.subpaths[1].closed = false;
    assert!(p.resume_at(1, 0));
    assert!(p.remove_subpath(0));
    assert_eq!(p.subpaths.len(), 1);
    assert_eq!(
        p.editing_subpath(),
        Some(0),
        "editing follows its subpath down"
    );
    assert!(!p.remove_subpath(1));
}

#[test]
fn hit_subpath_picks_the_inside_of_a_closed_one_and_the_line_of_an_open_one() {
    let mut p = closed_square(0.0, 0.0, 20.0);
    p.append_corner(5.0, 10.0);
    p.append_corner(15.0, 10.0);
    p.finish_editing();
    assert_eq!(p.hit_subpath(10.0, 4.0, 2.0), Some(0), "the interior hits");
    assert_eq!(
        p.hit_subpath(10.0, 10.5, 2.0),
        Some(1),
        "a line inside stays reachable"
    );
    assert_eq!(p.hit_subpath(-50.0, -50.0, 2.0), None);
    let mut lone = VectorPath::default();
    lone.append_corner(3.0, 3.0);
    assert_eq!(
        lone.hit_subpath(4.0, 3.0, 2.0),
        Some(0),
        "a lone anchor hits"
    );
}

#[test]
fn subpath_bounds_follow_the_curve_not_the_handles() {
    let p = closed_square(10.0, 20.0, 30.0);
    assert_eq!(p.subpath_bounds(0), Some((10.0, 20.0, 40.0, 50.0)));
    let mut curve = polyline(&[(0.0, 0.0)]);
    curve.update_last_handle(0.0, -40.0, false);
    curve.append_corner(20.0, 0.0);
    let (_, top, right, bottom) = curve.subpath_bounds(0).unwrap();
    assert!(
        top > -40.0 && top < -10.0,
        "the curve, not its handle: {top}"
    );
    assert_eq!((right, bottom), (20.0, 0.0));
    assert_eq!(VectorPath::default().subpath_bounds(0), None);
}
