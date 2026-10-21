use super::super::state::TransformSession;
use super::convex::keep_convex;
use super::geometry::{projective_coefficients, store_quad};

/// Map `(x, y)` through `projective_coefficients`' `QTransform`-order matrix:
/// `x' = (m11·x + m21·y + m31) / w`, `y' = (m12·x + m22·y + m32) / w`.
pub(super) fn map_qtransform(c: &[f64; 9], x: f64, y: f64) -> Option<(f64, f64)> {
    let w = c[2] * x + c[5] * y + c[8];
    if !w.is_finite() || w.abs() < 1e-12 {
        return None;
    }
    let out = (
        (c[0] * x + c[3] * y + c[6]) / w,
        (c[1] * x + c[4] * y + c[7]) / w,
    );
    (out.0.is_finite() && out.1.is_finite()).then_some(out)
}

/// Where `pivot` lands when the box goes from `from` to `to`, keeping its
/// position within the box. A singular map leaves it where it was.
fn carry_pivot(from: [(f64, f64); 4], to: [(f64, f64); 4], pivot: (f64, f64)) -> (f64, f64) {
    projective_coefficients(from, to)
        .and_then(|c| map_qtransform(&c, pivot.0, pivot.1))
        .unwrap_or(pivot)
}

/// Distort, as Photoshop's Distort mode: the grabbed corner moves by the
/// press-to-pointer delta, keeping the grab offset, and stops where the quad
/// would become concave. The reference point rides along with the box.
pub(super) fn gesture_distort(session: &mut TransformSession, corner: i32, x: f64, y: f64) -> bool {
    if !(0..=3).contains(&corner) {
        return false;
    }
    let start = session.start_quad;
    let delta = (x - session.press_x, y - session.press_y);
    let mut quad = start;
    quad[corner as usize] = (
        start[corner as usize].0 + delta.0,
        start[corner as usize].1 + delta.1,
    );
    let quad = keep_convex(start, quad);
    if !store_quad(session, quad) {
        return false;
    }
    session.pivot = carry_pivot(start, quad, session.start_pivot);
    true
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

    fn distort_session() -> TransformSession {
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
            mode: TransformMode::Distort,
            type_layer: false,
            quad: Some(corners),
            base_quad: None,
            free_perspective: false,
            perspective_side: None,
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
            history_label: "Free Transform".to_string(),
        }
    }

    #[test]
    fn distort_corner_keeps_the_grab_offset() {
        let mut s = distort_session();
        s.press_x = 98.0;
        s.press_y = 2.0;
        assert!(gesture_distort(&mut s, 1, 108.0, 5.0));
        assert_eq!(
            s.quad.unwrap(),
            [(0.0, 0.0), (110.0, 3.0), (100.0, 50.0), (0.0, 50.0)]
        );
    }

    #[test]
    fn distort_corner_stops_before_the_box_folds() {
        let mut s = distort_session();
        s.press_x = 100.0;
        s.press_y = 0.0;
        assert!(gesture_distort(&mut s, 1, 100.0, 100.0));
        let q = s.quad.unwrap();
        assert!(q[1].1 > 0.0 && q[1].1 < 50.0, "{q:?}");
        assert!(quad_is_convex(&q, 1.0, 0.0), "{q:?}");
        assert_eq!([q[0], q[2], q[3]], [(0.0, 0.0), (100.0, 50.0), (0.0, 50.0)]);
    }

    #[test]
    fn distort_carries_the_reference_point() {
        let mut s = distort_session();
        s.press_x = 100.0;
        s.press_y = 0.0;
        assert!(gesture_distort(&mut s, 1, 150.0, 0.0));
        assert_eq!(s.quad.unwrap()[1], (150.0, 0.0));
        assert!((s.pivot.0 - 60.0).abs() < 1e-9 && (s.pivot.1 - 30.0).abs() < 1e-9);
    }
}
