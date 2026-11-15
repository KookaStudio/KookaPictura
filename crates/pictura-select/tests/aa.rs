//! Anti-aliased primitive rasterisation: fractional boundary coverage, the
//! binary mode, the 50 % contour, feather order, and canvas clipping.

use pictura_select::{contour, Selection};

fn partial(selection: &Selection) -> bool {
    selection.data.as_slice().iter().any(|&v| v > 0 && v < 255)
}

fn binary(selection: &Selection) -> bool {
    selection
        .data
        .as_slice()
        .iter()
        .all(|&v| v == 0 || v == 255)
}

fn distance_to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    if len2 <= f64::EPSILON {
        return ((p.0 - a.0).powi(2) + (p.1 - a.1).powi(2)).sqrt();
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0);
    let (x, y) = (a.0 + t * dx, a.1 + t * dy);
    ((p.0 - x).powi(2) + (p.1 - y).powi(2)).sqrt()
}

fn distance_to_polygon(p: (f64, f64), points: &[(i32, i32)]) -> f64 {
    let n = points.len();
    (0..n)
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % n];
            distance_to_segment(p, (a.0 as f64, a.1 as f64), (b.0 as f64, b.1 as f64))
        })
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn ellipse_aa_has_fractional_edge() {
    let selection = Selection::ellipse_aa(20, 20, 2, 2, 14, 12);
    assert!(partial(&selection), "ellipse AA boundary is fractional");
    assert_eq!(selection.data[10 * 20 + 9], 255, "interior stays selected");
    assert_eq!(selection.data[0], 0, "exterior stays empty");
}

#[test]
fn ellipse_aa_grades_the_flat_caps() {
    let flat = Selection::ellipse_aa(40, 40, 2, 2, 16, 4);
    for &(x, y) in &[(5usize, 2usize), (14, 2)] {
        let value = flat.data[y * 40 + x];
        assert!(
            (i32::from(value) - 165).abs() <= 20,
            "flat cap ({x},{y}) = {value}, expected ~165"
        );
    }

    let tall = Selection::ellipse_aa(40, 40, 2, 2, 30, 20);
    for &(x, y) in &[(22usize, 2usize), (22, 21)] {
        let value = tall.data[y * 40 + x];
        assert!(
            (i32::from(value) - 77).abs() <= 20,
            "tall cap ({x},{y}) = {value}, expected ~77"
        );
    }
}

#[test]
fn ellipse_aa_off_is_binary() {
    let selection = Selection::ellipse(20, 20, 2, 2, 14, 12);
    assert!(binary(&selection));
}

#[test]
fn polygon_aa_has_fractional_edge() {
    let points = [(2, 2), (17, 4), (8, 17)];
    let selection = Selection::polygon_aa(20, 20, &points);
    assert!(partial(&selection), "polygon AA boundary is fractional");
    assert_eq!(selection.data[10 * 20 + 10], 255, "interior stays selected");
    assert_eq!(selection.data[0], 0, "exterior stays empty");
}

#[test]
fn polygon_aa_off_is_binary() {
    let points = [(2, 2), (17, 4), (8, 17)];
    let selection = Selection::polygon(20, 20, &points);
    assert!(binary(&selection));
}

#[test]
fn aa_contour_follows_the_primitive() {
    let points = [(2, 2), (17, 4), (8, 17)];
    let selection = Selection::polygon_aa(20, 20, &points);
    let loops = contour(&selection, 128);
    assert!(!loops.is_empty());
    for loop_points in loops {
        for point in loop_points {
            let distance = distance_to_polygon((point[0] as f64, point[1] as f64), &points);
            assert!(
                distance <= 1.0,
                "contour vertex {point:?} sits {distance} px away"
            );
        }
    }
}

#[test]
fn ellipse_aa_contour_follows_the_primitive() {
    let (width, height, x, y, w, h) = (40u32, 40u32, 5, 5, 28, 20);
    let selection = Selection::ellipse_aa(width, height, x, y, w, h);
    let (cx, cy) = (
        f64::from(x) + f64::from(w) / 2.0,
        f64::from(y) + f64::from(h) / 2.0,
    );
    let (rx, ry) = (f64::from(w) / 2.0, f64::from(h) / 2.0);
    let boundary: Vec<(f64, f64)> = (0..720)
        .map(|i| {
            let turn = f64::from(i) * std::f64::consts::TAU / 720.0;
            (cx + rx * turn.cos(), cy + ry * turn.sin())
        })
        .collect();
    let loops = contour(&selection, 128);
    assert!(!loops.is_empty());
    for loop_points in loops {
        for point in loop_points {
            let distance = boundary
                .iter()
                .fold(f64::INFINITY, |best, &(bx, by)| {
                    best.min(
                        (f64::from(point[0]) - bx).powi(2) + (f64::from(point[1]) - by).powi(2),
                    )
                })
                .sqrt();
            assert!(
                distance <= 1.0,
                "contour vertex {point:?} sits {distance} px away"
            );
        }
    }
}

#[test]
fn aa_applied_before_feather() {
    let aa = Selection::ellipse_aa(30, 30, 3, 3, 20, 16);
    let softened = aa.clone().feather(2.0);
    assert!(partial(&softened));
    assert_ne!(
        softened,
        Selection::ellipse(30, 30, 3, 3, 20, 16).feather(2.0)
    );
}

#[test]
fn aa_leaves_existing_selection_unchanged() {
    let existing = Selection::rect(30, 30, 1, 1, 5, 5);
    let before = existing.clone();
    let _ = Selection::ellipse_aa(30, 30, 3, 3, 20, 16);
    assert_eq!(existing, before);
}

#[test]
fn aa_clips_at_the_canvas_edge() {
    let ellipse = Selection::ellipse_aa(10, 10, -4, -2, 10, 8);
    assert_eq!(ellipse.data.len(), 100);
    assert!(partial(&ellipse));
    assert_eq!(
        ellipse.data[0], 255,
        "the canvas edge keeps its clipped value"
    );

    let points = [(-5, -5), (15, 3), (4, 15)];
    let polygon = Selection::polygon_aa(10, 10, &points);
    assert_eq!(polygon.data.len(), 100);
    assert!(partial(&polygon));

    let diamond = Selection::polygon_aa(10, 10, &[(4, 9), (9, 4), (15, 9), (9, 15)]);
    assert_eq!(diamond.data.len(), 100);
    assert_eq!(
        diamond.data[9 * 10 + 9],
        255,
        "a canvas-clipped edge pixel stays covered"
    );
    assert!(partial(&diamond), "the in-canvas boundary is soft");
}
