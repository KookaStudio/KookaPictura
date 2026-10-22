//! The transform options bar's bridge: the session's reference point, numeric
//! scale/rotation/skew setters and readouts, and the interpolation choice
//! (Warp mode is `warp.rs`). Free functions over [`PictureView`] (their own bridge, so
//! the `PictureView` declaration list does not grow).
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::document_session::{TransformMode, TransformSession};
use super::super::qobject::PictureView;
use super::geometry::{
    hit_test_quad, projective_coefficients, quad_anchors, quad_driven, rect_center, session_quad,
    set_pivot, source_corners, store_quad, MapParams,
};
use super::perspective::drag_angle_degrees;
use super::session::can_begin_mode;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_render::Interpolation;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The active session's mode name ("free", "skew", "distort",
        /// "perspective"), or empty without a session.
        fn transform_session_mode(view: &PictureView) -> QString;
        /// The session target's content rect as "left top right bottom", or
        /// empty without a session.
        fn transform_session_rect(view: &PictureView) -> QString;
        /// The layer at `path`'s rect as flat `[left, top, right, bottom]`;
        /// empty when the path does not resolve to a layer.
        fn layer_rect_extents(view: &PictureView, path: &QString) -> Vec<i32>;
        /// Whether a transform in `mode` ("skew", "distort", "perspective", or
        /// empty/"free") may begin on `path`, including Distort's and
        /// Perspective's editable-type refusal.
        fn layer_can_transform_mode(view: &PictureView, path: &QString, mode: &QString) -> bool;
        /// The layer at `path`'s ink box (the box round its non-transparent
        /// pixels, as Align computes it) as flat `[left, top, right, bottom]`;
        /// empty when the path does not resolve or has no content.
        fn layer_content_bounds_extents(view: &PictureView, path: &QString) -> Vec<i32>;
        /// The rect the Move tool's controls box hugs on the layer at `path`
        /// (the content Free Transform trims to) as flat `[left, top, right,
        /// bottom]`, changing nothing; empty when the path does not resolve.
        fn controls_box_extents(view: &PictureView, path: &QString) -> Vec<i32>;
        /// Hit-test the box `left, top, right, bottom` at document-space
        /// `(x, y)` as `transform_hit_test` does a session's: 0..=7 scale
        /// handles, 8 rotate, 9 inside, -1 none.
        fn controls_box_hit_test(
            left: i32,
            top: i32,
            right: i32,
            bottom: i32,
            x: f64,
            y: f64,
            zoom: f64,
        ) -> i32;
        /// The session quad's four document-space corners as flat
        /// `[x0, y0, …, x3, y3]`; empty without a session.
        fn transform_quad_extents(view: &PictureView) -> Vec<f64>;
        /// The nine preview-map coefficients in `QTransform` constructor
        /// order; empty without a session.
        fn transform_preview_matrix_extents(view: &PictureView) -> Vec<f64>;
        /// The nine preview-map coefficients in `QTransform` constructor
        /// order, expressed in the moving image's own coordinates (origin at
        /// the image's top-left), for a similarity or projective session;
        /// empty without a session or a moving image.
        fn transform_preview_local_matrix_extents(view: &PictureView) -> Vec<f64>;
        /// Move the reference point to document-space `(x, y)`, leaving the
        /// rendered quad unchanged.
        fn transform_set_pivot(view: Pin<&mut PictureView>, x: f64, y: f64) -> bool;
        /// Move the reference point to the live quad's anchor `0..=8`.
        fn transform_set_pivot_preset(view: Pin<&mut PictureView>, anchor: i32) -> bool;
        fn transform_set_show_pivot(view: Pin<&mut PictureView>, on: bool);
        fn transform_set_width(view: Pin<&mut PictureView>, pct: f64, link: bool) -> bool;
        fn transform_set_height(view: Pin<&mut PictureView>, pct: f64, link: bool) -> bool;
        fn transform_set_angle(view: Pin<&mut PictureView>, degrees: f64) -> bool;
        fn transform_set_skew(view: Pin<&mut PictureView>, h_degrees: f64, v_degrees: f64) -> bool;
        fn transform_set_interpolation(view: Pin<&mut PictureView>, id: &QString) -> bool;
        fn transform_pivot_x(view: &PictureView) -> f64;
        fn transform_pivot_y(view: &PictureView) -> f64;
        fn transform_pivot_origin_x(view: &PictureView) -> f64;
        fn transform_pivot_origin_y(view: &PictureView) -> f64;
        fn transform_show_pivot(view: &PictureView) -> bool;
        fn transform_width_pct(view: &PictureView) -> f64;
        fn transform_height_pct(view: &PictureView) -> f64;
        fn transform_angle_degrees(view: &PictureView) -> f64;
        fn transform_skew_h_degrees(view: &PictureView) -> f64;
        fn transform_skew_v_degrees(view: &PictureView) -> f64;
        fn transform_interpolation_id(view: &PictureView) -> QString;
        /// Turn the `Free` corner drag just pressed into a Ctrl+Alt+Shift
        /// perspective drag. False (the drag stays a scale) outside a `Free`
        /// corner drag, in Warp mode, or on editable type.
        fn transform_begin_free_perspective(view: Pin<&mut PictureView>) -> bool;
        /// The live Perspective drag's readout angle in degrees; NaN without
        /// one.
        fn transform_drag_angle_degrees(view: &PictureView) -> f64;
        /// Apply an exact transform op to the open session: `"rotate180"`,
        /// `"rotate90cw"`, `"rotate90ccw"`, `"flipHorizontal"`, or
        /// `"flipVertical"`. False without a session or for an unknown op.
        fn transform_apply(view: Pin<&mut PictureView>, op: &QString) -> bool;
        /// Restrict an open similarity session to Scale or Rotate gestures.
        /// False without a session, in a projective mode, or in Warp mode.
        fn transform_set_mode(view: Pin<&mut PictureView>, mode: &QString) -> bool;
        /// Whether [`transform_set_mode`] would accept `mode` right now; the
        /// menu uses it so a Scale/Rotate item is disabled where the switch
        /// cannot act.
        fn transform_can_set_mode(view: &PictureView, mode: &QString) -> bool;
        /// Apply the last committed similarity transform to the open session.
        /// False without a session or a repeatable last transform.
        fn transform_repeat(view: Pin<&mut PictureView>) -> bool;
        /// Whether a repeatable last transform exists.
        fn transform_has_repeat(view: &PictureView) -> bool;
        /// Set the label the next commit records.
        fn transform_set_history_label(view: Pin<&mut PictureView>, label: &QString);
    }
}

/// The projective-mode readouts derived from the live quad: `(width_pct,
/// height_pct, angle_deg, h_skew_deg, v_skew_deg)`. The rotation is the
/// balanced estimate `(top_edge_angle − left_edge_deviation)/2`; the skews are
/// the edges' deviations from their axes relative to it. Inferred (no Adobe
/// oracle); a readout, not state.
fn derived_readouts(session: &TransformSession) -> (f64, f64, f64, f64, f64) {
    let quad = session_quad(session);
    let (tl, tr, bl) = (quad[0], quad[1], quad[3]);
    let u = (tr.0 - tl.0, tr.1 - tl.1);
    let v = (bl.0 - tl.0, bl.1 - tl.1);
    let top_angle = u.1.atan2(u.0);
    let left_dev = v.0.atan2(v.1);
    let rotation = (top_angle - left_dev) / 2.0;
    let (w, h) = (
        f64::from(session.orig_rect.width()),
        f64::from(session.orig_rect.height()),
    );
    let width_pct = if w > 0.0 {
        u.0.hypot(u.1) / w * 100.0
    } else {
        100.0
    };
    let height_pct = if h > 0.0 {
        v.0.hypot(v.1) / h * 100.0
    } else {
        100.0
    };
    (
        width_pct,
        height_pct,
        rotation.to_degrees(),
        (left_dev + rotation).to_degrees(),
        (top_angle - rotation).to_degrees(),
    )
}

fn transform_session_mode(view: &PictureView) -> QString {
    view.rust()
        .session
        .live
        .transform()
        .map(|session| QString::from(session.mode.name()))
        .unwrap_or_default()
}

fn transform_session_rect(view: &PictureView) -> QString {
    view.rust()
        .session
        .live
        .transform()
        .map(|session| {
            let rect = session.orig_rect;
            QString::from(format!(
                "{} {} {} {}",
                rect.left, rect.top, rect.right, rect.bottom
            ))
        })
        .unwrap_or_default()
}

fn layer_rect_extents(view: &PictureView, path: &QString) -> Vec<i32> {
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::resolve_path(doc, &path.to_string()))
        .map(|layer| layer.rect)
        .map_or_else(Vec::new, |rect| {
            vec![rect.left, rect.top, rect.right, rect.bottom]
        })
}

fn layer_can_transform_mode(view: &PictureView, path: &QString, mode: &QString) -> bool {
    let mode = TransformMode::parse(&mode.to_string()).unwrap_or(TransformMode::Free);
    can_begin_mode(view.rust(), &path.to_string(), mode)
}

fn layer_content_bounds_extents(view: &PictureView, path: &QString) -> Vec<i32> {
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::layer_content_bounds(doc, &path.to_string()))
        .map_or_else(Vec::new, |rect| {
            vec![rect.left, rect.top, rect.right, rect.bottom]
        })
}

fn controls_box_extents(view: &PictureView, path: &QString) -> Vec<i32> {
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::content_rect(doc, &path.to_string()))
        .map_or_else(Vec::new, |rect| {
            vec![rect.left, rect.top, rect.right, rect.bottom]
        })
}

fn controls_box_hit_test(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    x: f64,
    y: f64,
    zoom: f64,
) -> i32 {
    let rect = pictura_core::PsdRect {
        top,
        left,
        bottom,
        right,
    };
    hit_test_quad(&source_corners(rect), false, x, y, zoom)
}

fn transform_quad_extents(view: &PictureView) -> Vec<f64> {
    let Some(session) = view.rust().session.live.transform() else {
        return Vec::new();
    };
    let quad = super::warp::warp_corners(session).unwrap_or_else(|| session_quad(session));
    let mut out = Vec::with_capacity(8);
    for (x, y) in quad {
        out.push(x);
        out.push(y);
    }
    out
}

fn transform_preview_matrix_extents(view: &PictureView) -> Vec<f64> {
    let Some(session) = view.rust().session.live.transform() else {
        return Vec::new();
    };
    if session.warp.is_some() {
        return vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    }
    let coeffs = if quad_driven(session) {
        projective_coefficients(source_corners(session.orig_rect), session_quad(session))
    } else {
        let params = MapParams::of(session);
        let a = params.linear();
        let (px, py) = params.pivot;
        let tx = px + params.dx - (a[0][0] * px + a[0][1] * py);
        let ty = py + params.dy - (a[1][0] * px + a[1][1] * py);
        Some([a[0][0], a[1][0], 0.0, a[0][1], a[1][1], 0.0, tx, ty, 1.0])
    };
    coeffs.map_or_else(Vec::new, |c| c.to_vec())
}

fn transform_preview_local_matrix_extents(view: &PictureView) -> Vec<f64> {
    let rust = view.rust();
    let Some(session) = rust.session.live.transform() else {
        return Vec::new();
    };
    if session.warp.is_some() {
        return vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    }
    if rust.move_layer.is_none() {
        return Vec::new();
    }
    let (x, y) = (f64::from(rust.move_x), f64::from(rust.move_y));
    if !quad_driven(session) {
        // The similarity map conjugated into the image's own coordinates:
        // `q_local = A·(p_local + o) + t − o` for the image origin `o`.
        let doc = transform_preview_matrix_extents(view);
        let [xx, xy, _, yx, yy, _, dx, dy, _] = doc[..] else {
            return Vec::new();
        };
        return vec![
            xx,
            xy,
            0.0,
            yx,
            yy,
            0.0,
            xx * x + yx * y + dx - x,
            xy * x + yy * y + dy - y,
            1.0,
        ];
    }
    let rect = session.orig_rect;
    let source = [
        (f64::from(rect.left) - x, f64::from(rect.top) - y),
        (f64::from(rect.right) - x, f64::from(rect.top) - y),
        (f64::from(rect.right) - x, f64::from(rect.bottom) - y),
        (f64::from(rect.left) - x, f64::from(rect.bottom) - y),
    ];
    let target = session_quad(session).map(|(qx, qy)| (qx - x, qy - y));
    projective_coefficients(source, target).map_or_else(Vec::new, |c| c.to_vec())
}

fn transform_set_pivot(mut view: Pin<&mut PictureView>, x: f64, y: f64) -> bool {
    if !x.is_finite() || !y.is_finite() {
        return false;
    }
    let mut rust = view.as_mut().rust_mut();
    let Some(session) = rust.session.live.transform_mut() else {
        return false;
    };
    set_pivot(session, x, y);
    true
}

fn transform_set_pivot_preset(mut view: Pin<&mut PictureView>, anchor: i32) -> bool {
    if !(0..=8).contains(&anchor) {
        return false;
    }
    let mut rust = view.as_mut().rust_mut();
    let Some(session) = rust.session.live.transform_mut() else {
        return false;
    };
    let point = quad_anchors(&session_quad(session))[anchor as usize];
    set_pivot(session, point.0, point.1);
    true
}

fn transform_set_show_pivot(mut view: Pin<&mut PictureView>, on: bool) {
    if let Some(session) = view.as_mut().rust_mut().session.live.transform_mut() {
        session.show_pivot = on;
    }
}

fn transform_set_width(view: Pin<&mut PictureView>, pct: f64, link: bool) -> bool {
    set_scale_axis(view, pct, link, true)
}

fn transform_set_height(view: Pin<&mut PictureView>, pct: f64, link: bool) -> bool {
    set_scale_axis(view, pct, link, false)
}

fn set_scale_axis(mut view: Pin<&mut PictureView>, pct: f64, link: bool, horizontal: bool) -> bool {
    if !pct.is_finite() || pct.abs() < 1e-3 {
        return false;
    }
    let mut rust = view.as_mut().rust_mut();
    let Some(session) = rust.session.live.transform_mut() else {
        return false;
    };
    if session.mode.is_projective() {
        // W/H are readouts in a projective mode; angle/H/V edit the quad.
        return false;
    }
    let target = pct / 100.0;
    if horizontal {
        let factor = target / session.scale_x;
        session.scale_x = target;
        if link {
            session.scale_y *= factor;
        }
    } else {
        let factor = target / session.scale_y;
        session.scale_y = target;
        if link {
            session.scale_x *= factor;
        }
    }
    true
}

fn transform_set_angle(mut view: Pin<&mut PictureView>, degrees: f64) -> bool {
    if !degrees.is_finite() {
        return false;
    }
    let mut rust = view.as_mut().rust_mut();
    let Some(session) = rust.session.live.transform_mut() else {
        return false;
    };
    if session.mode.is_projective() {
        let delta = degrees.to_radians() - derived_readouts(session).2.to_radians();
        let quad = session_quad(session);
        return rotate_quad_about(session, &quad, delta);
    }
    session.angle = degrees.to_radians();
    true
}

fn transform_set_skew(mut view: Pin<&mut PictureView>, h_degrees: f64, v_degrees: f64) -> bool {
    if !h_degrees.is_finite() || !v_degrees.is_finite() {
        return false;
    }
    let mut rust = view.as_mut().rust_mut();
    let Some(session) = rust.session.live.transform_mut() else {
        return false;
    };
    if session.mode.is_projective() {
        let (_, _, _, h, v) = derived_readouts(session);
        let quad = session_quad(session);
        return shear_quad_about(
            session,
            &quad,
            (h_degrees - h).to_radians(),
            (v_degrees - v).to_radians(),
        );
    }
    session.skew_x = h_degrees.to_radians();
    session.skew_y = v_degrees.to_radians();
    true
}

fn transform_set_interpolation(mut view: Pin<&mut PictureView>, id: &QString) -> bool {
    let Some(interpolation) = Interpolation::from_id(&id.to_string()) else {
        return false;
    };
    let mut rust = view.as_mut().rust_mut();
    let Some(session) = rust.session.live.transform_mut() else {
        return false;
    };
    session.interpolation = interpolation;
    true
}

fn transform_pivot_x(view: &PictureView) -> f64 {
    view.rust()
        .session
        .live
        .transform()
        .map_or(0.0, |s| s.pivot.0)
}

fn transform_pivot_y(view: &PictureView) -> f64 {
    view.rust()
        .session
        .live
        .transform()
        .map_or(0.0, |s| s.pivot.1)
}

/// The reference point at session begin (the source-rect centre); the base of
/// the relative-positioning display.
fn transform_pivot_origin_x(view: &PictureView) -> f64 {
    view.rust()
        .session
        .live
        .transform()
        .map_or(0.0, |s| s.pivot_origin.0)
}

fn transform_pivot_origin_y(view: &PictureView) -> f64 {
    view.rust()
        .session
        .live
        .transform()
        .map_or(0.0, |s| s.pivot_origin.1)
}

fn transform_show_pivot(view: &PictureView) -> bool {
    view.rust()
        .session
        .live
        .transform()
        .is_some_and(|s| s.show_pivot)
}

/// The horizontal scale as a percentage of the source rect: the stored scalar
/// in `Free`, the live quad's top edge in a projective mode.
fn transform_width_pct(view: &PictureView) -> f64 {
    let Some(session) = view.rust().session.live.transform() else {
        return 100.0;
    };
    if session.mode.is_projective() {
        derived_readouts(session).0
    } else {
        session.scale_x.abs() * 100.0
    }
}

fn transform_height_pct(view: &PictureView) -> f64 {
    let Some(session) = view.rust().session.live.transform() else {
        return 100.0;
    };
    if session.mode.is_projective() {
        derived_readouts(session).1
    } else {
        session.scale_y.abs() * 100.0
    }
}

fn transform_angle_degrees(view: &PictureView) -> f64 {
    let Some(session) = view.rust().session.live.transform() else {
        return 0.0;
    };
    if session.mode.is_projective() {
        derived_readouts(session).2
    } else {
        session.angle.to_degrees()
    }
}

fn transform_skew_h_degrees(view: &PictureView) -> f64 {
    let Some(session) = view.rust().session.live.transform() else {
        return 0.0;
    };
    if session.mode.is_projective() {
        derived_readouts(session).3
    } else {
        session.skew_x.to_degrees()
    }
}

fn transform_skew_v_degrees(view: &PictureView) -> f64 {
    let Some(session) = view.rust().session.live.transform() else {
        return 0.0;
    };
    if session.mode.is_projective() {
        derived_readouts(session).4
    } else {
        session.skew_y.to_degrees()
    }
}

fn transform_interpolation_id(view: &PictureView) -> QString {
    view.rust()
        .session
        .live
        .transform()
        .map(|s| QString::from(s.interpolation.id()))
        .unwrap_or_else(|| QString::from(Interpolation::Bicubic.id()))
}

/// Rotate `quad` about the reference point by `delta`; degenerate results are
/// refused.
fn rotate_quad_about(session: &mut TransformSession, quad: &[(f64, f64); 4], delta: f64) -> bool {
    if !delta.is_finite() {
        return false;
    }
    let (sin, cos) = delta.sin_cos();
    let (px, py) = session.pivot;
    let mut out = *quad;
    for corner in &mut out {
        let (x, y) = (corner.0 - px, corner.1 - py);
        *corner = (px + cos * x - sin * y, py + sin * x + cos * y);
    }
    store_quad(session, out)
}

/// Shear `quad` about the reference point in document space by `(dh, dv)`;
/// degenerate results are refused.
fn shear_quad_about(
    session: &mut TransformSession,
    quad: &[(f64, f64); 4],
    dh: f64,
    dv: f64,
) -> bool {
    if !dh.is_finite() || !dv.is_finite() {
        return false;
    }
    let (th, tv) = (dh.tan(), dv.tan());
    let (px, py) = session.pivot;
    let mut out = *quad;
    for corner in &mut out {
        let (x, y) = (corner.0 - px, corner.1 - py);
        *corner = (px + x + th * y, py + tv * x + y);
    }
    store_quad(session, out)
}

fn transform_begin_free_perspective(mut view: Pin<&mut PictureView>) -> bool {
    let mut rust = view.as_mut().rust_mut();
    let Some(session) = rust.session.live.transform_mut() else {
        return false;
    };
    if session.mode != TransformMode::Free
        || !session.dragging
        || !(0..=3).contains(&session.handle)
        || session.warp.is_some()
        || session.type_layer
    {
        return false;
    }
    session
        .base_quad
        .get_or_insert(source_corners(session.orig_rect));
    session.free_perspective = true;
    true
}

fn transform_drag_angle_degrees(view: &PictureView) -> f64 {
    view.rust()
        .session
        .live
        .transform()
        .and_then(drag_angle_degrees)
        .unwrap_or(f64::NAN)
}

/// The exact transform commands that apply to the open session in one step.
#[derive(Clone, Copy)]
enum TransformOp {
    Rotate180,
    Rotate90Cw,
    Rotate90Ccw,
    FlipHorizontal,
    FlipVertical,
}

impl TransformOp {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "rotate180" => Some(Self::Rotate180),
            "rotate90cw" => Some(Self::Rotate90Cw),
            "rotate90ccw" => Some(Self::Rotate90Ccw),
            "flipHorizontal" => Some(Self::FlipHorizontal),
            "flipVertical" => Some(Self::FlipVertical),
            _ => None,
        }
    }
}

fn transform_apply(mut view: Pin<&mut PictureView>, op: &QString) -> bool {
    let Some(op) = TransformOp::parse(&op.to_string()) else {
        return false;
    };
    let mut rust = view.as_mut().rust_mut();
    let Some(session) = rust.session.live.transform_mut() else {
        return false;
    };
    // Warp keeps the mesh in source space; the box's similarity frame still
    // rotates and flips, but the quad belongs to the warp path.
    if session.warp.is_none() && session.mode.is_projective() {
        let quad = session_quad(session);
        return match op {
            TransformOp::Rotate180 => rotate_quad_about(session, &quad, std::f64::consts::PI),
            TransformOp::Rotate90Cw => {
                rotate_quad_about(session, &quad, std::f64::consts::FRAC_PI_2)
            }
            TransformOp::Rotate90Ccw => {
                rotate_quad_about(session, &quad, -std::f64::consts::FRAC_PI_2)
            }
            TransformOp::FlipHorizontal => reflect_quad_about(session, &quad, true),
            TransformOp::FlipVertical => reflect_quad_about(session, &quad, false),
        };
    }
    match op {
        TransformOp::Rotate180 => session.angle += std::f64::consts::PI,
        TransformOp::Rotate90Cw => session.angle += std::f64::consts::FRAC_PI_2,
        TransformOp::Rotate90Ccw => session.angle -= std::f64::consts::FRAC_PI_2,
        TransformOp::FlipHorizontal => session.scale_x = -session.scale_x,
        TransformOp::FlipVertical => session.scale_y = -session.scale_y,
    }
    true
}

fn can_set_mode(session: &TransformSession, mode: TransformMode) -> bool {
    session.warp.is_none() && !session.mode.is_projective() && !mode.is_projective()
}

fn transform_set_mode(mut view: Pin<&mut PictureView>, mode: &QString) -> bool {
    let Some(mode) = TransformMode::parse(&mode.to_string()) else {
        return false;
    };
    let mut rust = view.as_mut().rust_mut();
    let Some(session) = rust.session.live.transform_mut() else {
        return false;
    };
    if !can_set_mode(session, mode) {
        return false;
    }
    session.mode = mode;
    true
}

fn transform_can_set_mode(view: &PictureView, mode: &QString) -> bool {
    let Some(mode) = TransformMode::parse(&mode.to_string()) else {
        return false;
    };
    view.rust()
        .session
        .live
        .transform()
        .is_some_and(|session| can_set_mode(session, mode))
}

fn transform_repeat(mut view: Pin<&mut PictureView>) -> bool {
    let Some(last) = view.rust().last_transform else {
        return false;
    };
    let mut rust = view.as_mut().rust_mut();
    let Some(session) = rust.session.live.transform_mut() else {
        return false;
    };
    if session.warp.is_some() || session.mode.is_projective() {
        return false;
    }
    let (cx, cy) = rect_center(session.orig_rect);
    session.pivot = (cx + last.pivot_offset.0, cy + last.pivot_offset.1);
    session.pivot_origin = (cx, cy);
    session.scale_x = last.scale_x;
    session.scale_y = last.scale_y;
    session.angle = last.angle;
    session.skew_x = last.skew_x;
    session.skew_y = last.skew_y;
    session.dx = last.dx;
    session.dy = last.dy;
    true
}

fn transform_has_repeat(view: &PictureView) -> bool {
    view.rust().last_transform.is_some()
}

fn transform_set_history_label(mut view: Pin<&mut PictureView>, label: &QString) {
    if let Some(session) = view.as_mut().rust_mut().session.live.transform_mut() {
        session.history_label = label.to_string();
    }
}

/// Reflect `quad` about the reference point's horizontal or vertical axis;
/// degenerate results are refused.
fn reflect_quad_about(
    session: &mut TransformSession,
    quad: &[(f64, f64); 4],
    horizontal: bool,
) -> bool {
    let (px, py) = session.pivot;
    let mut out = *quad;
    for corner in &mut out {
        if horizontal {
            corner.0 = 2.0 * px - corner.0;
        } else {
            corner.1 = 2.0 * py - corner.1;
        }
    }
    store_quad(session, out)
}
