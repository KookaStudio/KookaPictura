//! Canvas clipping for live selection-preview loops.

use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::clip::FloatClip;
use i_overlay::float::single::SingleFloatOverlay;
use i_overlay::string::clip::ClipRule;

const EDGE_SNAP: f64 = 1e-6;

type Point = (f64, f64);

/// Intersect closed selection-preview loops with a canvas rectangle using the
/// even-odd fill rule. Short, non-finite, and empty-intersection traces are
/// preserved; wholly outside loops are dropped.
pub fn clip_selection_shape_to_canvas(loops: &[Vec<Point>], canvas: (f64, f64)) -> Vec<Vec<Point>> {
    let (width, height) = canvas;
    if width <= 0.0 || height <= 0.0 || !width.is_finite() || !height.is_finite() {
        return loops.to_vec();
    }

    let mut out = Vec::with_capacity(loops.len());
    for loop_points in loops {
        if loop_points.len() < 3
            || loop_points
                .iter()
                .any(|point| !point.0.is_finite() || !point.1.is_finite())
        {
            out.push(loop_points.clone());
            continue;
        }

        let mut left = f64::INFINITY;
        let mut top = f64::INFINITY;
        let mut right = f64::NEG_INFINITY;
        let mut bottom = f64::NEG_INFINITY;
        for &(x, y) in loop_points {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
        if left >= 0.0 && top >= 0.0 && right <= width && bottom <= height {
            out.push(loop_points.clone());
            continue;
        }
        if right < 0.0 || left > width || bottom < 0.0 || top > height {
            continue;
        }

        let subject: Vec<[f64; 2]> = loop_points.iter().map(|&(x, y)| [x, y]).collect();
        let area = [[0.0, 0.0], [width, 0.0], [width, height], [0.0, height]];
        let shapes = subject.overlay_as::<i64>(&area, OverlayRule::Intersect, FillRule::EvenOdd);
        if !shapes.iter().any(|shape| !shape.is_empty()) {
            if loop_points.first() == loop_points.last() || signed_area(loop_points) != 0.0 {
                out.push(loop_points.clone());
            } else {
                let paths = subject.clip_by_as::<i64>(
                    &area,
                    FillRule::EvenOdd,
                    ClipRule {
                        invert: false,
                        boundary_included: true,
                    },
                );
                let mut clipped = Vec::new();
                for path in paths {
                    if path.len() >= 2 {
                        clipped.push(
                            path.into_iter()
                                .map(|point| {
                                    (
                                        snap_to_edge(point[0], width),
                                        snap_to_edge(point[1], height),
                                    )
                                })
                                .collect(),
                        );
                    }
                }
                if clipped.is_empty() {
                    out.push(loop_points.clone());
                } else {
                    out.extend(clipped);
                }
            }
            continue;
        }

        let mut parts = Vec::new();
        for shape in shapes {
            for contour in shape {
                if let Some(part) = filter_contour_arity(contour) {
                    parts.push(part);
                }
            }
        }
        for part in join_touching_contours(parts) {
            out.push(
                part.into_iter()
                    .map(|(x, y)| (snap_to_edge(x, width), snap_to_edge(y, height)))
                    .collect(),
            );
        }
    }
    out
}

fn signed_area(points: &[Point]) -> f64 {
    let mut area = 0.0;
    for (index, &(x1, y1)) in points.iter().enumerate() {
        let (x2, y2) = points[(index + 1) % points.len()];
        area += x1 * y2 - x2 * y1;
    }
    area * 0.5
}

fn snap_to_edge(value: f64, maximum: f64) -> f64 {
    if value.abs() <= EDGE_SNAP {
        0.0
    } else if (value - maximum).abs() <= EDGE_SNAP {
        maximum
    } else {
        value.clamp(0.0, maximum)
    }
}

fn filter_contour_arity(contour: Vec<[f64; 2]>) -> Option<Vec<Point>> {
    if contour.len() < 2 {
        return None;
    }
    Some(
        contour
            .into_iter()
            .map(|point| (point[0], point[1]))
            .collect(),
    )
}

fn join_touching_contours(mut contours: Vec<Vec<Point>>) -> Vec<Vec<Point>> {
    let mut joined: Vec<Vec<Point>> = Vec::with_capacity(contours.len());
    for contour in contours.drain(..) {
        let mut next = contour;
        while let Some((loop_index, loop_vertex, next_vertex)) = shared_endpoint(&joined, &next) {
            let mut existing = joined.remove(loop_index);
            if loop_vertex != 0 {
                existing.rotate_left(loop_vertex);
            }
            if next_vertex != 0 {
                let next_len = next.len();
                next.rotate_right(next_len - next_vertex);
            }
            existing.extend(next);
            next = existing;
        }
        joined.push(next);
    }
    joined
}

fn shared_endpoint(loops: &[Vec<Point>], next: &[Point]) -> Option<(usize, usize, usize)> {
    for (index, loop_points) in loops.iter().enumerate() {
        for (loop_vertex, loop_point) in loop_points.iter().enumerate() {
            for (next_vertex, next_point) in next.iter().enumerate() {
                if loop_point == next_point {
                    return Some((index, loop_vertex, next_vertex));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_loop(points: &[Point]) -> Vec<(i64, i64)> {
        let mut clean = Vec::new();
        for &point in points {
            let quantized = (
                (point.0 * 1_000_000.0).round() as i64,
                (point.1 * 1_000_000.0).round() as i64,
            );
            if clean.last() != Some(&quantized) {
                clean.push(quantized);
            }
        }
        if clean.len() > 1 && clean.first() == clean.last() {
            clean.pop();
        }
        while clean.len() > 3 {
            let count = clean.len();
            let remove = (0..count).find(|&index| {
                let a = clean[(index + count - 1) % count];
                let b = clean[index];
                let c = clean[(index + 1) % count];
                let ab = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
                let bc = (i128::from(c.0 - b.0), i128::from(c.1 - b.1));
                ab.0 * bc.1 - ab.1 * bc.0 == 0 && ab.0 * bc.0 + ab.1 * bc.1 >= 0
            });
            let Some(index) = remove else {
                break;
            };
            clean.remove(index);
        }
        if clean.len() < 3 {
            return clean;
        }
        let reversed: Vec<_> = clean.iter().rev().copied().collect();
        min_rotation(clean).min(min_rotation(reversed))
    }

    fn min_rotation(mut points: Vec<(i64, i64)>) -> Vec<(i64, i64)> {
        let mut best = points.clone();
        for _ in 1..points.len() {
            points.rotate_left(1);
            best = best.min(points.clone());
        }
        best
    }

    fn canonical_shape(loops: &[Vec<Point>]) -> Vec<Vec<(i64, i64)>> {
        let mut loops: Vec<_> = loops
            .iter()
            .map(|loop_points| canonical_loop(loop_points))
            .collect();
        loops.sort();
        loops
    }

    #[test]
    fn inner_loop_is_unchanged() {
        let loop_points = vec![
            (10.0, 10.0),
            (30.0, 10.0),
            (30.0, 30.0),
            (10.0, 30.0),
            (10.0, 10.0),
        ];
        assert_eq!(
            clip_selection_shape_to_canvas(std::slice::from_ref(&loop_points), (64.0, 64.0)),
            vec![loop_points]
        );
    }

    #[test]
    fn outside_loop_is_dropped() {
        let loop_points = vec![(80.0, 80.0), (100.0, 80.0), (100.0, 100.0), (80.0, 100.0)];
        assert!(clip_selection_shape_to_canvas(&[loop_points], (64.0, 64.0)).is_empty());
    }

    #[test]
    fn short_traces_are_unchanged() {
        let loops = vec![vec![], vec![(5.0, 5.0)], vec![(5.0, 5.0), (90.0, 5.0)]];
        assert_eq!(clip_selection_shape_to_canvas(&loops, (64.0, 64.0)), loops);
    }

    #[test]
    fn zero_area_trace_is_unchanged() {
        let loop_points = vec![
            (20.0, 20.0),
            (90.0, 20.0),
            (90.0, 20.0),
            (60.0, 20.0),
            (20.0, 20.0),
        ];
        assert_eq!(
            clip_selection_shape_to_canvas(std::slice::from_ref(&loop_points), (64.0, 64.0)),
            vec![loop_points]
        );
    }

    #[test]
    fn border_crossing_closes_along_the_border() {
        let loop_points = vec![(50.0, 50.0), (100.0, 50.0), (100.0, 100.0), (50.0, 100.0)];
        let clipped = clip_selection_shape_to_canvas(&[loop_points], (64.0, 64.0));
        let expected = vec![vec![(50.0, 50.0), (64.0, 50.0), (64.0, 64.0), (50.0, 64.0)]];
        assert_eq!(canonical_shape(&clipped), canonical_shape(&expected));
        assert!(clipped[0].contains(&(64.0, 50.0)));
        assert!(clipped[0].contains(&(64.0, 64.0)));
        assert!(clipped[0].contains(&(50.0, 64.0)));
    }

    #[test]
    fn self_overlap_uses_the_even_odd_region() {
        let loop_points = vec![
            (20.0, 20.0),
            (90.0, 20.0),
            (20.0, 50.0),
            (90.0, 50.0),
            (20.0, 20.0),
        ];
        let clipped = clip_selection_shape_to_canvas(&[loop_points], (64.0, 64.0));
        let expected = vec![vec![
            (20.0, 20.0),
            (64.0, 20.0),
            (64.0, 218.0 / 7.0),
            (55.0, 35.0),
            (64.0, 272.0 / 7.0),
            (64.0, 50.0),
            (20.0, 50.0),
            (55.0, 35.0),
        ]];
        assert_eq!(clipped.len(), 1);
        assert_eq!(canonical_shape(&clipped), canonical_shape(&expected));
    }

    #[test]
    fn invalid_canvas_returns_the_input() {
        let loops = vec![vec![(5.0, 5.0), (90.0, 5.0), (90.0, 90.0)]];
        for canvas in [
            (0.0, 64.0),
            (64.0, 0.0),
            (-1.0, 64.0),
            (64.0, -1.0),
            (f64::NAN, 64.0),
            (64.0, f64::INFINITY),
        ] {
            assert_eq!(clip_selection_shape_to_canvas(&loops, canvas), loops);
        }
    }

    #[test]
    fn non_finite_loop_is_unchanged() {
        let loops = vec![
            vec![(f64::NAN, 5.0), (90.0, 5.0), (90.0, 90.0)],
            vec![(5.0, f64::INFINITY), (90.0, 5.0), (90.0, 90.0)],
        ];
        let result = clip_selection_shape_to_canvas(&loops, (64.0, 64.0));
        assert_eq!(result.len(), loops.len());
        for (actual, expected) in result.iter().zip(&loops) {
            assert_eq!(actual.len(), expected.len());
            for (&(ax, ay), &(ex, ey)) in actual.iter().zip(expected) {
                assert!(
                    (ax == ex || (ax.is_nan() && ex.is_nan()))
                        && (ay == ey || (ay.is_nan() && ey.is_nan()))
                );
            }
        }
    }

    #[test]
    fn clipping_preserves_holes() {
        let outer = vec![
            (-10.0, -10.0),
            (74.0, -10.0),
            (74.0, 74.0),
            (-10.0, 74.0),
            (-10.0, -10.0),
        ];
        let hole = vec![
            (-10.0, 20.0),
            (30.0, 20.0),
            (30.0, 40.0),
            (-10.0, 40.0),
            (-10.0, 20.0),
        ];
        let expected = vec![
            vec![(0.0, 0.0), (64.0, 0.0), (64.0, 64.0), (0.0, 64.0)],
            vec![(0.0, 20.0), (30.0, 20.0), (30.0, 40.0), (0.0, 40.0)],
        ];
        let forward = clip_selection_shape_to_canvas(&[outer.clone(), hole.clone()], (64.0, 64.0));
        let reverse = clip_selection_shape_to_canvas(&[hole, outer], (64.0, 64.0));
        assert_eq!(canonical_shape(&forward), canonical_shape(&expected));
        assert_eq!(canonical_shape(&forward), canonical_shape(&reverse));
    }

    #[test]
    fn near_touching_contours_stay_separate() {
        let first = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let second = vec![
            (10.0 + 0.5e-6, 10.0),
            (20.0, 10.0),
            (20.0, 20.0),
            (10.0 + 0.5e-6, 20.0),
        ];
        assert_eq!(
            join_touching_contours(vec![first.clone(), second.clone()]),
            vec![first, second]
        );
    }

    #[test]
    fn exact_endpoint_join_preserves_both_contours() {
        let first = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let second = vec![(10.0, 10.0), (20.0, 10.0), (20.0, 20.0), (10.0, 20.0)];
        let joined = join_touching_contours(vec![first, second]);
        let expected = vec![vec![
            (10.0, 10.0),
            (0.0, 10.0),
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (20.0, 10.0),
            (20.0, 20.0),
            (10.0, 20.0),
        ]];
        assert_eq!(joined, expected);
    }

    #[test]
    fn contour_arity_filter_keeps_two_or_more_points() {
        assert_eq!(filter_contour_arity(vec![]), None);
        assert_eq!(filter_contour_arity(vec![[0.0, 0.0]]), None);
        assert_eq!(
            filter_contour_arity(vec![[0.0, 0.0], [1.0, 1.0]]),
            Some(vec![(0.0, 0.0), (1.0, 1.0)])
        );
        assert_eq!(
            filter_contour_arity(vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]),
            Some(vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)])
        );
    }

    #[test]
    fn degenerate_intersections_follow_arity_rules() {
        let point_tangent = vec![(-10.0, 0.0), (0.0, 0.0), (-5.0, -10.0), (-10.0, 0.0)];
        let line_tangent = vec![
            (-10.0, 0.0),
            (0.0, 0.0),
            (0.0, 10.0),
            (-10.0, 10.0),
            (-10.0, 0.0),
        ];
        for loop_points in [point_tangent, line_tangent] {
            assert_eq!(
                clip_selection_shape_to_canvas(std::slice::from_ref(&loop_points), (64.0, 64.0)),
                vec![loop_points]
            );
        }
        assert_eq!(
            join_touching_contours(vec![vec![(0.0, 0.0)], vec![(1.0, 1.0), (2.0, 2.0)]]),
            vec![vec![(0.0, 0.0)], vec![(1.0, 1.0), (2.0, 2.0)]]
        );
    }

    #[test]
    fn duplicate_closing_vertices_and_border_collinear_edges_are_simplified() {
        let loop_points = vec![
            (50.0, 50.0),
            (60.0, 50.0),
            (100.0, 50.0),
            (100.0, 100.0),
            (50.0, 100.0),
            (50.0, 50.0),
            (50.0, 50.0),
        ];
        let clipped = clip_selection_shape_to_canvas(&[loop_points], (64.0, 64.0));
        let expected = vec![vec![(50.0, 50.0), (64.0, 50.0), (64.0, 64.0), (50.0, 64.0)]];
        assert_eq!(canonical_shape(&clipped), canonical_shape(&expected));
    }

    #[test]
    fn open_zero_area_band_is_clipped() {
        let loop_points = vec![(50.0, 50.0), (75.0, 75.0), (100.0, 100.0)];
        let clipped = clip_selection_shape_to_canvas(&[loop_points], (64.0, 64.0));
        assert_eq!(clipped, vec![vec![(50.0, 50.0), (64.0, 64.0)]]);
    }

    #[test]
    fn large_coordinates_keep_sub_micro_precision() {
        let canvas = 300_000.0;
        let loop_points = vec![
            (canvas - 10.0, 10.0),
            (canvas + 10.0, 13.0),
            (canvas + 10.0, 27.0),
            (canvas - 10.0, 30.0),
        ];
        let clipped = clip_selection_shape_to_canvas(&[loop_points], (canvas, canvas));
        let expected = vec![
            (canvas - 10.0, 10.0),
            (canvas, 11.5),
            (canvas, 28.5),
            (canvas - 10.0, 30.0),
        ];
        assert_eq!(clipped.len(), 1);
        assert_eq!(clipped[0].len(), expected.len());
        assert_eq!(canonical_shape(&clipped), canonical_shape(&[expected]));
        assert!(clipped[0].contains(&(canvas, 11.5)));
        assert!(clipped[0].contains(&(canvas, 28.5)));
    }
}
