//! Perspective, as Photoshop CS6's Perspective mode and Free Transform's
//! Ctrl+Alt+Shift corner drag (#750). No photorust/photocraft code exists for
//! it; the rules come from the owner's CS6 screen recording:
//!
//! - a corner slides along one of its two edges and that edge's other corner
//!   slides the same distance the other way, about the edge's midpoint;
//! - the edge is the one the drag follows most, re-picked on every move;
//! - the two corners may meet and pass through each other (a bowtie);
//! - an edge handle slides only its own edge along itself;
//! - the reference point keeps its bilinear place in the box.

use super::super::state::TransformSession;
use super::distort::map_qtransform;
use super::geometry::{
    handle_source_point, opposite_handle, projective_coefficients, quad_is_degenerate,
    session_quad, set_pivot, source_corners, store_quad, MapParams,
};

type Quad = [(f64, f64); 4];

/// The other end of corner `c`'s top/bottom edge, then of its side edge.
fn partners(corner: usize) -> (usize, usize) {
    match corner {
        0 => (1, 3),
        1 => (0, 2),
        2 => (3, 1),
        _ => (2, 0),
    }
}

/// The Perspective drag of `handle` by `delta` from the press-time quad
/// `start`, and the readout side `(moved corner, unmoved neighbour, top/bottom
/// edge moved)`. `None` for a handle outside 0..=7.
pub(super) fn perspective_quad(
    start: Quad,
    handle: i32,
    delta: (f64, f64),
) -> Option<(Quad, (usize, usize, bool))> {
    let mut quad = start;
    if (0..=3).contains(&handle) {
        let corner = handle as usize;
        let (h, v) = partners(corner);
        let along = |n: usize| {
            let e = (start[corner].0 - start[n].0, start[corner].1 - start[n].1);
            let len = e.0.hypot(e.1);
            if len <= f64::EPSILON {
                return (0.0, (0.0, 0.0));
            }
            let u = (e.0 / len, e.1 / len);
            (delta.0 * u.0 + delta.1 * u.1, u)
        };
        let ((th, uh), (tv, uv)) = (along(h), along(v));
        let (t, u, partner, other) = if th.abs() >= tv.abs() {
            (th, uh, h, v)
        } else {
            (tv, uv, v, h)
        };
        quad[corner] = (start[corner].0 + t * u.0, start[corner].1 + t * u.1);
        quad[partner] = (start[partner].0 - t * u.0, start[partner].1 - t * u.1);
        return Some((quad, (corner, other, partner == h)));
    }
    let (e0, e1) = match handle {
        4 => (0, 1),
        5 => (1, 2),
        6 => (2, 3),
        7 => (3, 0),
        _ => return None,
    };
    let e = (start[e1].0 - start[e0].0, start[e1].1 - start[e0].1);
    let len2 = e.0 * e.0 + e.1 * e.1;
    let t = if len2 > f64::EPSILON {
        (delta.0 * e.0 + delta.1 * e.1) / len2
    } else {
        0.0
    };
    for k in [e0, e1] {
        quad[k] = (start[k].0 + t * e.0, start[k].1 + t * e.1);
    }
    Some((quad, (e0, (e0 + 3) % 4, handle == 4 || handle == 6)))
}

/// The bilinear point at `(u, v)`: `u` runs TL→TR, `v` runs TL→BL.
fn bilinear(q: &Quad, u: f64, v: f64) -> (f64, f64) {
    let w = [(1.0 - u) * (1.0 - v), u * (1.0 - v), u * v, (1.0 - u) * v];
    (
        w.iter().zip(q).map(|(w, p)| w * p.0).sum(),
        w.iter().zip(q).map(|(w, p)| w * p.1).sum(),
    )
}

/// `(u, v)` with `bilinear(q, u, v) == p`, by Newton steps from the centre;
/// `None` when they do not converge.
fn inverse_bilinear(q: &Quad, p: (f64, f64)) -> Option<(f64, f64)> {
    let (mut u, mut v) = (0.5, 0.5);
    for _ in 0..32 {
        let b = bilinear(q, u, v);
        let r = (b.0 - p.0, b.1 - p.1);
        if r.0.hypot(r.1) < 1e-9 {
            return Some((u, v));
        }
        let du = (
            (1.0 - v) * (q[1].0 - q[0].0) + v * (q[2].0 - q[3].0),
            (1.0 - v) * (q[1].1 - q[0].1) + v * (q[2].1 - q[3].1),
        );
        let dv = (
            (1.0 - u) * (q[3].0 - q[0].0) + u * (q[2].0 - q[1].0),
            (1.0 - u) * (q[3].1 - q[0].1) + u * (q[2].1 - q[1].1),
        );
        let det = du.0 * dv.1 - du.1 * dv.0;
        if !det.is_finite() || det.abs() < 1e-12 {
            return None;
        }
        u -= (r.0 * dv.1 - r.1 * dv.0) / det;
        v -= (du.0 * r.1 - du.1 * r.0) / det;
    }
    None
}

/// Where `pivot` lands when the box goes from `from` to `to`, keeping its
/// bilinear position; it stays put when that position cannot be found.
fn carry_pivot(from: &Quad, to: &Quad, pivot: (f64, f64)) -> (f64, f64) {
    inverse_bilinear(from, pivot).map_or(pivot, |(u, v)| bilinear(to, u, v))
}

/// Drag `handle` to `(x, y)` as a Perspective drag, in `Perspective` mode or
/// as a `Free` Ctrl+Alt+Shift drag (which edits the perspective base). A
/// degenerate result is refused, keeping the last box.
pub(super) fn gesture_perspective(
    session: &mut TransformSession,
    handle: i32,
    x: f64,
    y: f64,
) -> bool {
    let start = session.start_quad;
    let delta = (x - session.press_x, y - session.press_y);
    let Some((quad, side)) = perspective_quad(start, handle, delta) else {
        return false;
    };
    let pivot = carry_pivot(&start, &quad, session.start_pivot);
    if session.free_perspective {
        if quad_is_degenerate(&quad) {
            return false;
        }
        // Move the similarity's pivot first (its image is unchanged), then
        // pull the new box back through the similarity into the base.
        set_pivot(session, pivot.0, pivot.1);
        let Some(base) = unmap_quad(&MapParams::of(session), &quad) else {
            return false;
        };
        session.base_quad = Some(base);
    } else {
        if !store_quad(session, quad) {
            return false;
        }
        session.pivot = pivot;
    }
    session.perspective_side = Some(side);
    true
}

/// `quad` pulled back through the similarity `params`, or `None` when its
/// linear part is singular.
fn unmap_quad(params: &MapParams, quad: &Quad) -> Option<Quad> {
    let a = params.linear();
    let det = a[0][0] * a[1][1] - a[0][1] * a[1][0];
    if !det.is_finite() || det.abs() < 1e-12 {
        return None;
    }
    let (px, py) = params.pivot;
    Some(quad.map(|(qx, qy)| {
        let (rx, ry) = (qx - px - params.dx, qy - py - params.dy);
        (
            px + (a[1][1] * rx - a[0][1] * ry) / det,
            py + (a[0][0] * ry - a[1][0] * rx) / det,
        )
    }))
}

/// Scale a `Free` box that has a perspective base, in the box's own projective
/// frame: the pointer is pulled back to source space, the source rect scales
/// about the opposite handle there (Shift locks a corner's aspect ratio), and
/// the result goes back through the press-time homography, so the opposite
/// edge stays exactly where it is. The similarity is unchanged; the base takes
/// the new box. Inferred (no Adobe oracle).
pub(super) fn gesture_scale_base(
    session: &mut TransformSession,
    x: f64,
    y: f64,
    shift: bool,
) -> bool {
    let rect = session.orig_rect;
    let h = session.handle;
    if !(0..=7).contains(&h) {
        return false;
    }
    let source = source_corners(rect);
    let (Some(forward), Some(inverse)) = (
        projective_coefficients(source, session.start_quad),
        projective_coefficients(session.start_quad, source),
    ) else {
        return false;
    };
    let (Some(pointer), Some(pressed)) = (
        map_qtransform(&inverse, x, y),
        map_qtransform(&inverse, session.press_x, session.press_y),
    ) else {
        return false;
    };
    let handle = handle_source_point(rect, h);
    let anchor = handle_source_point(rect, opposite_handle(h));
    let drag = (
        handle.0 + pointer.0 - pressed.0,
        handle.1 + pointer.1 - pressed.1,
    );
    let factor = |d: f64, a: f64, hp: f64| {
        if (hp - a).abs() > 1e-12 {
            (d - a) / (hp - a)
        } else {
            1.0
        }
    };
    let mut sx = factor(drag.0, anchor.0, handle.0);
    let mut sy = factor(drag.1, anchor.1, handle.1);
    if shift && h <= 3 {
        let uniform = if sx.abs() > sy.abs() { sx } else { sy };
        sx = uniform;
        sy = uniform;
    }
    let min = |s: f64, len: i32| {
        let floor = 1.0 / f64::from(len.max(1));
        if s.abs() < floor {
            floor.copysign(s)
        } else {
            s
        }
    };
    let (sx, sy) = (min(sx, rect.width()), min(sy, rect.height()));
    let mut quad = source;
    for p in &mut quad {
        let scaled = (
            anchor.0 + sx * (p.0 - anchor.0),
            anchor.1 + sy * (p.1 - anchor.1),
        );
        let Some(mapped) = map_qtransform(&forward, scaled.0, scaled.1) else {
            return false;
        };
        *p = mapped;
    }
    if quad_is_degenerate(&quad) {
        return false;
    }
    let Some(base) = unmap_quad(&MapParams::of(session), &quad) else {
        return false;
    };
    session.base_quad = Some(base);
    true
}

/// The live Perspective drag's readout angle in degrees: the signed turn of the
/// side joining the moved corner to its unmoved neighbour since the press,
/// negated for a top/bottom edge, so spreading reads negative. Inferred from
/// the CS6 recording.
pub(super) fn drag_angle_degrees(session: &TransformSession) -> Option<f64> {
    let (moved, fixed, horizontal) = session.perspective_side?;
    let angle = |q: &Quad| (q[moved].1 - q[fixed].1).atan2(q[moved].0 - q[fixed].0);
    let mut turn = (angle(&session_quad(session)) - angle(&session.start_quad)).to_degrees();
    if turn > 180.0 {
        turn -= 360.0;
    } else if turn <= -180.0 {
        turn += 360.0;
    }
    Some(if horizontal { -turn } else { turn })
}

#[cfg(test)]
mod tests {
    use super::super::super::state::{TransformMode, TransformSession};
    use super::super::geometry::{session_quad, source_corners};
    use super::*;
    use pictura_core::PsdRect;

    fn rect100() -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: 100,
            right: 100,
        }
    }

    fn session(mode: TransformMode) -> TransformSession {
        let corners = source_corners(rect100());
        TransformSession {
            path: "0".to_string(),
            orig_rect: rect100(),
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
            mode,
            type_layer: false,
            quad: mode.is_projective().then_some(corners),
            base_quad: None,
            free_perspective: false,
            perspective_side: None,
            start_quad: corners,
            pivot: (50.0, 50.0),
            pivot_origin: (50.0, 50.0),
            start_pivot: (50.0, 50.0),
            show_pivot: true,
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

    /// Press `handle` at `(x, y)` the way `transform_press` does.
    fn press(s: &mut TransformSession, handle: i32, x: f64, y: f64) {
        s.handle = handle;
        s.press_x = x;
        s.press_y = y;
        s.start = [s.scale_x, s.scale_y, s.angle, s.dx, s.dy];
        s.start_quad = session_quad(s);
        s.start_pivot = s.pivot;
        s.perspective_side = None;
    }

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6
    }

    #[test]
    fn perspective_corner_spreads_its_edge() {
        let mut s = session(TransformMode::Perspective);
        press(&mut s, 1, 100.0, 0.0);
        assert!(gesture_perspective(&mut s, 1, 120.0, -5.0));
        assert_eq!(
            s.quad.unwrap(),
            [(-20.0, 0.0), (120.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        );
    }

    #[test]
    fn perspective_repicks_the_edge_mid_drag() {
        let mut s = session(TransformMode::Perspective);
        press(&mut s, 1, 100.0, 0.0);
        assert!(gesture_perspective(&mut s, 1, 120.0, -5.0));
        assert!(gesture_perspective(&mut s, 1, 105.0, -30.0));
        assert_eq!(
            s.quad.unwrap(),
            [(0.0, 0.0), (100.0, -30.0), (100.0, 130.0), (0.0, 100.0)]
        );
    }

    #[test]
    fn perspective_tilted_edge_spreads_along_itself() {
        let mut s = session(TransformMode::Perspective);
        // A top edge tilted along (4, 3), already a 5-unit step per corner.
        s.quad = Some([(0.0, 0.0), (80.0, 60.0), (80.0, 160.0), (0.0, 100.0)]);
        press(&mut s, 1, 80.0, 60.0);
        assert!(gesture_perspective(&mut s, 1, 88.0, 66.0));
        let q = s.quad.unwrap();
        assert!(close(q[1], (88.0, 66.0)), "{q:?}");
        assert!(close(q[0], (-8.0, -6.0)), "{q:?}");
        assert_eq!([q[2], q[3]], [(80.0, 160.0), (0.0, 100.0)]);
    }

    #[test]
    fn perspective_corners_cross_into_a_bowtie() {
        let mut s = session(TransformMode::Perspective);
        press(&mut s, 1, 100.0, 0.0);
        assert!(gesture_perspective(&mut s, 1, 20.0, 0.0));
        let q = s.quad.unwrap();
        assert_eq!([q[0], q[1]], [(80.0, 0.0), (20.0, 0.0)]);
        assert_eq!([q[2], q[3]], [(100.0, 100.0), (0.0, 100.0)]);
    }

    #[test]
    fn perspective_degenerate_quad_is_refused() {
        let mut s = session(TransformMode::Perspective);
        press(&mut s, 1, 100.0, 0.0);
        assert!(gesture_perspective(&mut s, 1, 90.0, 0.0));
        let kept = s.quad.unwrap();
        // The two top corners meet at the midpoint: three corners on a line.
        assert!(!gesture_perspective(&mut s, 1, 50.0, 0.0));
        assert_eq!(s.quad.unwrap(), kept);
        assert!(!gesture_perspective(&mut s, 1, f64::NAN, 0.0));
        assert_eq!(s.quad.unwrap(), kept);
    }

    #[test]
    fn perspective_edge_slides_only_its_edge() {
        let mut s = session(TransformMode::Perspective);
        press(&mut s, 4, 50.0, 0.0);
        assert!(gesture_perspective(&mut s, 4, 30.0, 25.0));
        assert_eq!(
            s.quad.unwrap(),
            [(-20.0, 0.0), (80.0, 0.0), (100.0, 100.0), (0.0, 100.0)]
        );
    }

    #[test]
    fn perspective_carries_the_pivot_bilinearly() {
        let mut s = session(TransformMode::Perspective);
        press(&mut s, 2, 100.0, 100.0);
        assert!(gesture_perspective(&mut s, 2, 100.0, 140.0));
        let q = s.quad.unwrap();
        let mean = (
            q.iter().map(|p| p.0).sum::<f64>() / 4.0,
            q.iter().map(|p| p.1).sum::<f64>() / 4.0,
        );
        assert!(close(s.pivot, mean), "{:?} vs {mean:?}", s.pivot);
        // An edge slide moves the centre by half the slide.
        let mut s = session(TransformMode::Perspective);
        press(&mut s, 4, 50.0, 0.0);
        assert!(gesture_perspective(&mut s, 4, 70.0, 0.0));
        assert!(close(s.pivot, (60.0, 50.0)), "{:?}", s.pivot);
    }

    #[test]
    fn perspective_readout_angle_signs() {
        let mut s = session(TransformMode::Perspective);
        press(&mut s, 1, 100.0, 0.0);
        assert!(gesture_perspective(&mut s, 1, 200.0, 0.0));
        assert!((drag_angle_degrees(&s).unwrap() + 45.0).abs() < 1e-9);
        // Pinching reads positive.
        assert!(gesture_perspective(&mut s, 1, 80.0, 0.0));
        assert!(drag_angle_degrees(&s).unwrap() > 0.0);
        // A side edge spreading reads negative too.
        assert!(gesture_perspective(&mut s, 1, 100.0, -20.0));
        assert!(drag_angle_degrees(&s).unwrap() < 0.0);
        let idle = session(TransformMode::Perspective);
        assert!(drag_angle_degrees(&idle).is_none());
    }

    fn free_perspective_session() -> TransformSession {
        let mut s = session(TransformMode::Free);
        press(&mut s, 1, 100.0, 0.0);
        s.base_quad = Some(source_corners(rect100()));
        s.free_perspective = true;
        s
    }

    #[test]
    fn free_perspective_drag_matches_the_mode() {
        let mut s = free_perspective_session();
        assert!(gesture_perspective(&mut s, 1, 120.0, -5.0));
        assert!(s.quad.is_none());
        let q = session_quad(&s);
        let want = [(-20.0, 0.0), (120.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
        assert!(q.iter().zip(&want).all(|(a, b)| close(*a, *b)), "{q:?}");
    }

    #[test]
    fn free_base_composes_with_the_similarity() {
        let mut s = free_perspective_session();
        s.scale_x = 2.0;
        s.angle = 0.3;
        s.dx = 7.0;
        let corner = s.start_quad[1];
        press(&mut s, 1, corner.0, corner.1);
        s.free_perspective = true;
        assert!(gesture_perspective(
            &mut s,
            1,
            corner.0 + 10.0,
            corner.1 + 2.0
        ));
        let live = session_quad(&s);
        let (want, _) = perspective_quad(s.start_quad, 1, (10.0, 2.0)).unwrap();
        assert!(
            live.iter().zip(&want).all(|(a, b)| close(*a, *b)),
            "{live:?}"
        );
        // The similarity still applies after the base.
        let base = s.base_quad.unwrap();
        let params = MapParams::of(&s);
        assert!(base
            .iter()
            .zip(&live)
            .all(|(b, l)| close(params.map_point(*b), *l)));
    }

    #[test]
    fn free_scale_anchors_on_the_base() {
        let mut s = free_perspective_session();
        assert!(gesture_perspective(&mut s, 1, 120.0, 0.0));
        s.free_perspective = false;
        let before = session_quad(&s);
        let right = (
            (before[1].0 + before[2].0) / 2.0,
            (before[1].1 + before[2].1) / 2.0,
        );
        press(&mut s, 5, right.0, right.1);
        assert!(gesture_scale_base(&mut s, right.0 + 30.0, right.1, false));
        let after = session_quad(&s);
        assert!(
            close(after[0], before[0]) && close(after[3], before[3]),
            "{after:?}"
        );
        assert!(
            after[1].0 > before[1].0 && after[2].0 > before[2].0,
            "{after:?}"
        );
        // Still a perspective box: the side edges are not parallel.
        let left = (after[3].0 - after[0].0, after[3].1 - after[0].1);
        let right = (after[2].0 - after[1].0, after[2].1 - after[1].1);
        assert!(
            (left.0 * right.1 - left.1 * right.0).abs() > 1e-6,
            "{after:?}"
        );
    }
}
