//! Switching an open transform session's mode from `Edit > Transform` (#1297).
//! Scale and Rotate restrict a similarity session's gestures; Skew, Distort,
//! and Perspective take any session's box as their quad, so the pending move,
//! scale, rotation, and skew carry over as CS6 keeps them.

use super::super::document_session::{TransformMode, TransformSession};
use super::geometry::{pivot_point, quad_is_degenerate, session_quad};

/// Whether an open `session` may switch to `mode`. Leaving a projective mode
/// for a similarity one is refused, as are a session editing or holding a
/// warp (its box is the warp's frame), and Distort or Perspective on editable
/// type, which their begin refuses too.
pub(super) fn can_set_mode(session: &TransformSession, mode: TransformMode) -> bool {
    if session.warp_editing {
        return false;
    }
    if !mode.is_projective() {
        return !session.mode.is_projective();
    }
    session.warp.is_none()
        && !(session.type_layer
            && matches!(mode, TransformMode::Distort | TransformMode::Perspective))
}

/// Switch `session` to `mode`, keeping the box's corners and the reference
/// point's document position. A similarity session entering a projective mode
/// becomes a quad session: its live box is the quad, and its similarity resets
/// to identity with the reference point as a document point.
pub(super) fn switch_mode(session: &mut TransformSession, mode: TransformMode) -> bool {
    if !can_set_mode(session, mode) {
        return false;
    }
    if mode.is_projective() && !session.mode.is_projective() {
        let quad = session_quad(session);
        if quad_is_degenerate(&quad) {
            return false;
        }
        let pivot = pivot_point(session);
        // The relative X/Y readout's base, carried by the translation it had.
        session.pivot_origin = (
            session.pivot_origin.0 + session.dx,
            session.pivot_origin.1 + session.dy,
        );
        session.quad = Some(quad);
        session.start_quad = quad;
        session.base_quad = None;
        session.scale_x = 1.0;
        session.scale_y = 1.0;
        session.angle = 0.0;
        session.skew_x = 0.0;
        session.skew_y = 0.0;
        session.dx = 0.0;
        session.dy = 0.0;
        session.start = [1.0, 1.0, 0.0, 0.0, 0.0];
        session.free_perspective = false;
        session.free_distort = false;
        session.free_skew = false;
        session.perspective_side = None;
        session.pivot = pivot;
        session.start_pivot = pivot;
    }
    session.mode = mode;
    true
}

#[cfg(test)]
mod tests {
    use super::super::geometry::{source_corners, MapParams};
    use super::*;
    use pictura_core::PsdRect;

    const RECT: PsdRect = PsdRect {
        top: 10,
        left: 20,
        bottom: 60,
        right: 120,
    };

    fn free_session() -> TransformSession {
        let corners = source_corners(RECT);
        TransformSession {
            path: "0".to_string(),
            members: vec![("0".to_string(), RECT)],
            orig_rect: RECT,
            scale_x: 1.0,
            scale_y: 1.0,
            angle: 0.0,
            dx: 0.0,
            dy: 0.0,
            handle: -1,
            press_x: 0.0,
            press_y: 0.0,
            start: [1.0, 1.0, 0.0, 0.0, 0.0],
            dragging: false,
            mode: TransformMode::Free,
            type_layer: false,
            quad: None,
            base_quad: None,
            free_perspective: false,
            free_distort: false,
            free_skew: false,
            perspective_side: None,
            start_quad: corners,
            pivot: (70.0, 35.0),
            pivot_origin: (70.0, 35.0),
            start_pivot: (70.0, 35.0),
            show_pivot: true,
            skew_x: 0.0,
            skew_y: 0.0,
            interpolation: pictura_render::Interpolation::Bicubic,
            warp: None,
            warp_drag: None,
            warp_editing: false,
            warp_post: None,
            lifted: None,
            untrimmed: Vec::new(),
            history_label: "Free Transform".to_string(),
        }
    }

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    // [pms_keep]
    #[test]
    fn switching_to_skew_keeps_the_box_and_the_reference_point() {
        let mut s = free_session();
        s.scale_x = 1.5;
        s.angle = 30f64.to_radians();
        s.dx = 7.0;
        s.dy = -4.0;
        s.pivot = (40.0, 20.0);
        let before = session_quad(&s);
        let pivot = pivot_point(&s);
        let origin = (s.pivot_origin.0 + s.dx, s.pivot_origin.1 + s.dy);

        assert!(switch_mode(&mut s, TransformMode::Skew));
        assert_eq!(s.mode, TransformMode::Skew);
        let after = session_quad(&s);
        for (a, b) in after.iter().zip(before.iter()) {
            assert!(close(*a, *b), "{after:?} vs {before:?}");
        }
        assert_eq!(s.start_quad, after);
        assert!(close(pivot_point(&s), pivot));
        assert!(close(s.pivot_origin, origin));
        assert!(MapParams::of(&s).is_identity());
        // Skew to Distort keeps the quad as it is.
        assert!(switch_mode(&mut s, TransformMode::Distort));
        assert_eq!(session_quad(&s), after);
    }

    // [pms_type] [pms_warp]
    #[test]
    fn projective_switch_refusals() {
        let mut typed = free_session();
        typed.type_layer = true;
        assert!(!switch_mode(&mut typed, TransformMode::Distort));
        assert!(!switch_mode(&mut typed, TransformMode::Perspective));
        assert_eq!(typed.mode, TransformMode::Free);
        assert!(can_set_mode(&typed, TransformMode::Skew));

        let mut warped = free_session();
        warped.warp = Some(pictura_core::warp::Warp::none([20.0, 10.0, 120.0, 60.0]));
        assert!(!switch_mode(&mut warped, TransformMode::Skew));
        let mut editing = free_session();
        editing.warp_editing = true;
        assert!(!switch_mode(&mut editing, TransformMode::Skew));

        let mut skewed = free_session();
        assert!(switch_mode(&mut skewed, TransformMode::Skew));
        assert!(!switch_mode(&mut skewed, TransformMode::Scale));
        assert_eq!(skewed.mode, TransformMode::Skew);
    }
}
