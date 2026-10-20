use super::super::state::TransformSession;
use super::convex::keep_convex;
use super::geometry::store_quad;

/// Skew, as Photoshop's Skew mode: a corner moves alone along whichever of its
/// two adjacent edges the drag follows most, while an edge handle slides its two
/// endpoints only along the edge's own axis. Alt moves the opposite corner or
/// edge by the same displacement in the opposite direction. An editable type
/// layer keeps a parallelogram, so a corner drag slides the whole followed edge.
/// Shift has no effect in this mode.
pub(super) fn gesture_skew(
    session: &mut TransformSession,
    handle: i32,
    x: f64,
    y: f64,
    alt: bool,
) -> bool {
    let start = session.start_quad;
    let delta = (x - session.press_x, y - session.press_y);
    if (0..=3).contains(&handle) {
        let corner = handle as usize;
        let (h_neighbor, v_neighbor) = match corner {
            0 => (1, 3),
            1 => (0, 2),
            2 => (3, 1),
            _ => (2, 0),
        };
        let along = |neighbor: usize| {
            let (ex, ey) = (
                start[corner].0 - start[neighbor].0,
                start[corner].1 - start[neighbor].1,
            );
            let len = ex.hypot(ey);
            if len <= f64::EPSILON {
                return (0.0, 0.0);
            }
            let (ux, uy) = (ex / len, ey / len);
            let t = delta.0 * ux + delta.1 * uy;
            (t * ux, t * uy)
        };
        let (horizontal, vertical) = (along(h_neighbor), along(v_neighbor));
        let (shift, partner) = if horizontal.0.hypot(horizontal.1) >= vertical.0.hypot(vertical.1) {
            (horizontal, h_neighbor)
        } else {
            (vertical, v_neighbor)
        };
        let moved: &[usize] = if session.type_layer {
            &[corner, partner]
        } else {
            &[corner]
        };
        let mut quad = start;
        for k in 0..4 {
            let sign = if moved.contains(&k) {
                1.0
            } else if alt
                && (if session.type_layer {
                    true
                } else {
                    k == (corner + 2) % 4
                })
            {
                -1.0
            } else {
                continue;
            };
            quad[k] = (start[k].0 + sign * shift.0, start[k].1 + sign * shift.1);
        }
        return store_quad(session, keep_convex(start, quad));
    }
    let (e0, e1) = match handle {
        4 => (0, 1),
        5 => (1, 2),
        6 => (2, 3),
        7 => (3, 0),
        _ => return false,
    };
    let (ex, ey) = (start[e1].0 - start[e0].0, start[e1].1 - start[e0].1);
    let len2 = ex * ex + ey * ey;
    let t = if len2 > f64::EPSILON {
        (delta.0 * ex + delta.1 * ey) / len2
    } else {
        0.0
    };
    let shift = (t * ex, t * ey);
    let mut quad = start;
    quad[e0] = (start[e0].0 + shift.0, start[e0].1 + shift.1);
    quad[e1] = (start[e1].0 + shift.0, start[e1].1 + shift.1);
    if alt {
        let (o0, o1) = ((handle as usize + 2) % 4, (handle as usize + 3) % 4);
        quad[o0] = (start[o0].0 - shift.0, start[o0].1 - shift.1);
        quad[o1] = (start[o1].0 - shift.0, start[o1].1 - shift.1);
    }
    store_quad(session, quad)
}

#[cfg(test)]
mod tests {
    use super::super::super::state::{TransformMode, TransformSession};
    use super::super::convex::quad_is_convex;
    use super::super::geometry::source_corners;
    use super::*;
    use pictura_core::PsdRect;

    fn rect100x50() -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: 50,
            right: 100,
        }
    }

    fn skew_session() -> TransformSession {
        let corners = source_corners(rect100x50());
        TransformSession {
            path: "0".to_string(),
            orig_rect: rect100x50(),
            scale_x: 1.0,
            scale_y: 1.0,
            angle: 0.0,
            dx: 0.0,
            dy: 0.0,
            handle: -1,
            press_x: 0.0,
            press_y: 0.0,
            start: [1.0, 1.0, 0.0, 0.0, 0.0],
            dragging: true,
            mode: TransformMode::Skew,
            type_layer: false,
            quad: Some(corners),
            start_quad: corners,
            pivot: (50.0, 25.0),
            pivot_origin: (50.0, 25.0),
            start_pivot: (50.0, 25.0),
            show_pivot: false,
            skew_x: 0.0,
            skew_y: 0.0,
            interpolation: pictura_render::Interpolation::Bicubic,
            warp: None,
            warp_drag: None,
            lifted: None,
            untrimmed: None,
        }
    }

    #[test]
    fn skew_corner_follows_its_dominant_edge() {
        let mut s = skew_session();
        assert!(gesture_skew(&mut s, 1, 10.0, 3.0, false));
        assert_eq!(
            s.quad.unwrap(),
            [(0.0, 0.0), (110.0, 0.0), (100.0, 50.0), (0.0, 50.0)]
        );

        let mut s = skew_session();
        assert!(gesture_skew(&mut s, 1, 2.0, -9.0, false));
        assert_eq!(
            s.quad.unwrap(),
            [(0.0, 0.0), (100.0, -9.0), (100.0, 50.0), (0.0, 50.0)]
        );
    }

    #[test]
    fn skew_edge_slides_along_itself() {
        let mut s = skew_session();
        assert!(gesture_skew(&mut s, 4, 12.0, 7.0, false));
        assert_eq!(
            s.quad.unwrap(),
            [(12.0, 0.0), (112.0, 0.0), (100.0, 50.0), (0.0, 50.0)]
        );

        let mut s = skew_session();
        assert!(gesture_skew(&mut s, 7, 6.0, 8.0, false));
        assert_eq!(
            s.quad.unwrap(),
            [(0.0, 8.0), (100.0, 0.0), (100.0, 50.0), (0.0, 58.0)]
        );
    }

    #[test]
    fn skew_alt_mirrors_the_opposite_corner_or_edge() {
        let mut s = skew_session();
        assert!(gesture_skew(&mut s, 1, 10.0, 3.0, true));
        assert_eq!(
            s.quad.unwrap(),
            [(0.0, 0.0), (110.0, 0.0), (100.0, 50.0), (-10.0, 50.0)]
        );

        let mut s = skew_session();
        assert!(gesture_skew(&mut s, 4, 12.0, 7.0, true));
        assert_eq!(
            s.quad.unwrap(),
            [(12.0, 0.0), (112.0, 0.0), (88.0, 50.0), (-12.0, 50.0)]
        );
    }

    #[test]
    fn skew_keeps_editable_type_a_parallelogram() {
        let mut s = skew_session();
        s.type_layer = true;
        assert!(gesture_skew(&mut s, 1, 10.0, 3.0, false));
        assert_eq!(
            s.quad.unwrap(),
            [(10.0, 0.0), (110.0, 0.0), (100.0, 50.0), (0.0, 50.0)]
        );

        let mut s = skew_session();
        s.type_layer = true;
        assert!(gesture_skew(&mut s, 1, 10.0, 3.0, true));
        assert_eq!(
            s.quad.unwrap(),
            [(10.0, 0.0), (110.0, 0.0), (90.0, 50.0), (-10.0, 50.0)]
        );
    }

    #[test]
    fn skew_corner_stops_before_the_box_folds() {
        let mut s = skew_session();
        s.press_x = 100.0;
        s.press_y = 0.0;
        assert!(gesture_skew(&mut s, 1, 100.0, 100.0, false));
        let q = s.quad.unwrap();
        assert!(q[1].1 > 0.0 && q[1].1 < 50.0, "{:?}", q);
        assert!(quad_is_convex(&q, 1.0, 0.0), "{:?}", q);
    }
}
