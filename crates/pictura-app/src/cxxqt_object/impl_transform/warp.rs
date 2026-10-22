//! Edit ▸ Transform ▸ Warp: the Free Transform session's Warp mode. Entering
//! Warp starts a one-patch custom mesh over the source box; the box's current
//! transform stays as a projective frame after the mesh (Photoshop keeps `Trnf`
//! apart from `warp`), so a Distort or Perspective made first carries over
//! exactly. A press on an anchor or handle drags it, a press anywhere else on
//! the grid pulls the surface under the pointer, and the options bar picks a
//! preset style with its bend and distortions. Commit applies the warp and the
//! frame together. Ported from photocraft's `transform_tool` Warp mode (see
//! `NOTICE.md`); its split warps and Grid menu (post-CS6) are not.
//!
//! Free functions over [`PictureView`] (their own bridge, so the `PictureView`
//! declaration list does not grow).
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::document_session::{TransformSession, WarpDrag};
use super::super::qobject::PictureView;
use super::geometry::{projective_coefficients, session_quad, source_corners};
use super::session::transform_preview_layer;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::warp::{BezierMesh, Warp, WarpStyle};

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
        /// Switch the session into Warp mode: a custom mesh following the
        /// current box. True when already warping; false without a session.
        fn transform_enter_warp(view: Pin<&mut PictureView>) -> bool;
        /// Back to the box: the warp edits are dropped, the box is as it was.
        fn transform_leave_warp(view: Pin<&mut PictureView>) -> bool;
        fn transform_warp_active(view: &PictureView) -> bool;
        /// Pick a warp style by id (`none`, `custom`, `arc`, `arcLower`…).
        /// Custom keeps the current shape as a mesh, None resets it to the
        /// box, a preset starts at the current bend (50% from a non-preset).
        fn transform_set_warp_style(view: Pin<&mut PictureView>, id: &QString) -> bool;
        /// Set a preset's bend and horizontal/vertical distortion (percent,
        /// clamped to ±100); false unless a preset is active.
        fn transform_set_warp_values(
            view: Pin<&mut PictureView>,
            bend: f64,
            h_distort: f64,
            v_distort: f64,
        ) -> bool;
        /// Set a preset's orientation; false unless a preset is active.
        fn transform_set_warp_vertical(view: Pin<&mut PictureView>, vertical: bool) -> bool;
        fn transform_warp_style_id(view: &PictureView) -> QString;
        fn transform_warp_bend(view: &PictureView) -> f64;
        fn transform_warp_h_distort(view: &PictureView) -> f64;
        fn transform_warp_v_distort(view: &PictureView) -> f64;
        fn transform_warp_vertical(view: &PictureView) -> bool;
        /// The grid's patch boundaries, each as [`CURVE_POINTS`] document-space
        /// points flattened `[x0, y0, x1, y1, …]`, curve after curve.
        fn transform_warp_grid_lines(view: &PictureView) -> Vec<f64>;
        /// A one-patch grid's rule-of-thirds guides, in the same layout.
        fn transform_warp_guide_lines(view: &PictureView) -> Vec<f64>;
        /// A custom warp's draggable points as `[kind, x, y, anchor_x,
        /// anchor_y]` records: kind 0 an anchor (its own anchor), 1 a handle,
        /// 2 a handle of the anchor being dragged. Empty for a preset.
        fn transform_warp_points(view: &PictureView) -> Vec<f64>;
    }
}

/// A press on a custom warp's anchor or handle.
pub(super) const WARP_POINT_HANDLE: i32 = 11;
/// A press elsewhere on the warped grid: the surface follows the pointer.
pub(super) const WARP_SURFACE_HANDLE: i32 = 12;
/// Grab radius of the warp's control points, in screen pixels.
const HANDLE_PX: f64 = 12.0;
/// Points per drawn grid curve.
const CURVE_POINTS: usize = 41;

fn rect_bounds(rect: pictura_core::PsdRect) -> [f64; 4] {
    [
        f64::from(rect.left),
        f64::from(rect.top),
        f64::from(rect.right),
        f64::from(rect.bottom),
    ]
}

/// The box's projective map from the source box onto the session quad, and
/// its inverse (the coefficient layout of [`projective_coefficients`]).
struct Frame {
    h: [f64; 9],
    inv: [f64; 9],
}

impl Frame {
    fn new(src: [(f64, f64); 4], dst: [(f64, f64); 4]) -> Option<Frame> {
        Some(Frame {
            h: projective_coefficients(src, dst)?,
            inv: projective_coefficients(dst, src)?,
        })
    }

    fn of(session: &TransformSession) -> Option<Frame> {
        Frame::new(source_corners(session.orig_rect), session_quad(session))
    }

    fn to_doc(&self, p: [f64; 2]) -> [f64; 2] {
        apply(&self.h, p)
    }

    /// The source point under document point `p`, `None` past the horizon.
    fn to_source(&self, p: [f64; 2]) -> Option<[f64; 2]> {
        let q = apply(&self.inv, p);
        (q[0].is_finite() && q[1].is_finite()).then_some(q)
    }
}

fn apply(c: &[f64; 9], p: [f64; 2]) -> [f64; 2] {
    let w = c[2] * p[0] + c[5] * p[1] + c[8];
    [
        (c[0] * p[0] + c[3] * p[1] + c[6]) / w,
        (c[1] * p[0] + c[4] * p[1] + c[7]) / w,
    ]
}

/// The warp's four corner anchors as a quad (TL, TR, BR, BL), or `None` when
/// the session is not warping.
pub(super) fn warp_corners(session: &TransformSession) -> Option<[(f64, f64); 4]> {
    let m = session.warp.as_ref()?.to_mesh(1, 1);
    let frame = Frame::of(session)?;
    let (nx, ny) = (m.nx(), m.ny());
    let p = |i: usize, j: usize| {
        let q = frame.to_doc(m.point(i, j));
        (q[0], q[1])
    };
    Some([p(0, 0), p(nx - 1, 0), p(nx - 1, ny - 1), p(0, ny - 1)])
}

/// An anchor or a handle on a section line. The inner control points of a
/// patch are not shown.
fn on_section_edge(i: usize, j: usize) -> bool {
    i.is_multiple_of(3) || j.is_multiple_of(3)
}

/// The anchor a handle belongs to, as `(i, j)` in the control grid (an anchor
/// is its own).
fn handle_anchor(i: usize, j: usize) -> (usize, usize) {
    if i.is_multiple_of(3) {
        let aj = match j % 3 {
            0 => j,
            1 => j - 1,
            _ => j + 1,
        };
        (i, aj)
    } else {
        (if i % 3 == 1 { i - 1 } else { i + 1 }, j)
    }
}

/// Index of the control point within `tol` of `p` (nearest anchor or handle),
/// `m` in document space.
fn warp_hit(m: &BezierMesh, p: [f64; 2], tol: f64) -> Option<usize> {
    let nx = m.nx();
    m.points
        .iter()
        .enumerate()
        .filter(|(i, _)| on_section_edge(i % nx, i / nx))
        .map(|(i, q)| (i, (q[0] - p[0]).hypot(q[1] - p[1])))
        .filter(|(_, d)| *d < tol)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// The surface parameters under document point `p` when it is inside the
/// warped mesh (`m` in source space, seen through `frame`).
fn surface_at(m: &BezierMesh, frame: &Frame, p: [f64; 2]) -> Option<(f64, f64)> {
    let (s, t) = m.param_at(frame.to_source(p)?);
    let q = frame.to_doc(m.eval(s, t));
    ((q[0] - p[0]).hypot(q[1] - p[1]) < 1.0).then_some((s, t))
}

/// The custom form of `w`: itself when it is a custom mesh, else its shape as a
/// one-patch mesh.
fn as_custom(w: &Warp) -> Warp {
    match (&w.mesh, w.style) {
        (Some(_), WarpStyle::Custom) => w.clone(),
        _ => Warp::custom(w.to_mesh(1, 1), w.bounds),
    }
}

/// What a press at document point `p` would grab: a control point, the
/// surface, or nothing. A preset hides its points, so only the surface grabs
/// it. The drag's `start` is the press in source space.
fn warp_grab(w: &Warp, frame: &Frame, p: [f64; 2], zoom: f64) -> Option<WarpDrag> {
    let custom = as_custom(w);
    let m = custom.mesh.as_ref()?;
    let start = frame.to_source(p)?;
    let pts0 = m.points.clone();
    let point = match (w.style, &w.mesh) {
        (WarpStyle::Custom, Some(_)) => {
            let shown = m.map_points(|q| frame.to_doc(q));
            warp_hit(&shown, p, HANDLE_PX / zoom.max(0.01))
        }
        _ => None,
    };
    match point {
        Some(index) => Some(WarpDrag::Point { index, start, pts0 }),
        None => surface_at(m, frame, p).map(|(s, t)| WarpDrag::Surface { s, t, start, pts0 }),
    }
}

fn drag_handle(drag: &WarpDrag) -> i32 {
    match drag {
        WarpDrag::Point { .. } => WARP_POINT_HANDLE,
        WarpDrag::Surface { .. } => WARP_SURFACE_HANDLE,
    }
}

/// The handle a press at `(x, y)` would grab in Warp mode, or -1.
pub(super) fn warp_hit_test(session: &TransformSession, x: f64, y: f64, zoom: f64) -> i32 {
    let Some(frame) = Frame::of(session) else {
        return -1;
    };
    session
        .warp
        .as_ref()
        .and_then(|w| warp_grab(w, &frame, [x, y], zoom))
        .map_or(-1, |d| drag_handle(&d))
}

/// Begin a Warp-mode drag. A preset warp turns into a custom mesh only when the
/// press grabs something.
pub(super) fn warp_press(session: &mut TransformSession, x: f64, y: f64, zoom: f64) -> i32 {
    let Some(frame) = Frame::of(session) else {
        return -1;
    };
    let Some(w) = session.warp.as_mut() else {
        return -1;
    };
    let Some(drag) = warp_grab(w, &frame, [x, y], zoom) else {
        return -1;
    };
    *w = as_custom(w);
    let hit = drag_handle(&drag);
    session.warp_drag = Some(drag);
    hit
}

/// Follow the pointer with the grabbed point or surface (the move is taken in
/// source space, through the box's frame).
pub(super) fn warp_move(session: &mut TransformSession, x: f64, y: f64) -> bool {
    let Some(p) = Frame::of(session).and_then(|f| f.to_source([x, y])) else {
        return false;
    };
    let (Some(drag), Some(m)) = (
        session.warp_drag.as_ref(),
        session.warp.as_mut().and_then(|w| w.mesh.as_mut()),
    ) else {
        return false;
    };
    match drag {
        WarpDrag::Point { index, start, pts0 } => {
            m.drag_point(pts0, *index, [p[0] - start[0], p[1] - start[1]])
        }
        WarpDrag::Surface { s, t, start, pts0 } => {
            m.pull(pts0, *s, *t, [p[0] - start[0], p[1] - start[1]])
        }
    }
    true
}

/// Recompute the warped preview: the layer warped on a document clone, handed
/// to the canvas as the preview layer (the base stays: only the layer moves).
/// Mutates nothing.
// ponytail: re-warps the whole layer at full resolution on every edit; a large
// layer drags slowly. Ceiling: warp a downsampled preview, as photocraft's GPU
// texture mesh does.
pub(super) fn refresh_warp_preview(mut view: Pin<&mut PictureView>) -> bool {
    let gpu_compute = view.rust().gpu_compute;
    let prepared = (|| {
        let rust = view.rust();
        let doc = rust.session.doc.as_ref()?;
        let session = rust.session.live.transform()?;
        let warp = session.warp.as_ref()?;
        let mut clone = doc.clone();
        // An identity (or refused) warp previews the untouched layer.
        pictura_render::transform_layer_warp(
            &mut clone,
            &session.path,
            warp,
            Some(session_quad(session)),
            session.interpolation,
        );
        transform_preview_layer(&clone, &session.path, gpu_compute)
    })();
    let Some((image, x, y)) = prepared else {
        return false;
    };
    let mut rust = view.as_mut().rust_mut();
    rust.move_layer = Some(image);
    rust.move_x = x;
    rust.move_y = y;
    true
}

/// Run `edit` on the session's warp and refresh the preview when it changed
/// something.
fn edit_warp(
    mut view: Pin<&mut PictureView>,
    edit: impl FnOnce(&mut TransformSession) -> bool,
) -> bool {
    let changed = {
        let mut rust = view.as_mut().rust_mut();
        match rust.session.live.transform_mut() {
            Some(session) if session.warp.is_some() => edit(session),
            _ => return false,
        }
    };
    changed && refresh_warp_preview(view)
}

fn transform_enter_warp(mut view: Pin<&mut PictureView>) -> bool {
    {
        let mut rust = view.as_mut().rust_mut();
        let Some(session) = rust.session.live.transform_mut() else {
            return false;
        };
        if session.warp.is_some() {
            return true;
        }
        if Frame::of(session).is_none() {
            return false;
        }
        let b = rect_bounds(session.orig_rect);
        session.warp = Some(Warp::custom(BezierMesh::identity(b, 1, 1), b));
        session.warp_drag = None;
        session.dragging = false;
        session.handle = -1;
    }
    refresh_warp_preview(view)
}

fn transform_leave_warp(mut view: Pin<&mut PictureView>) -> bool {
    let path = {
        let mut rust = view.as_mut().rust_mut();
        let Some(session) = rust.session.live.transform_mut() else {
            return false;
        };
        if session.warp.take().is_none() {
            return true;
        }
        session.warp_drag = None;
        session.dragging = false;
        session.handle = -1;
        session.path.clone()
    };
    view.as_mut().compute_transform_preview(&path)
}

fn transform_warp_active(view: &PictureView) -> bool {
    view.rust()
        .session
        .live
        .transform()
        .is_some_and(|s| s.warp.is_some())
}

fn transform_set_warp_style(view: Pin<&mut PictureView>, id: &QString) -> bool {
    let Some(style) = WarpStyle::parse(&id.to_string()) else {
        return false;
    };
    edit_warp(view, |session| {
        let Some(w) = session.warp.as_ref() else {
            return false;
        };
        let b = w.bounds;
        let next = match style {
            WarpStyle::Custom => Warp::custom(w.to_mesh(1, 1), b),
            WarpStyle::None => Warp::custom(BezierMesh::identity(b, 1, 1), b),
            s => Warp {
                style: s,
                bend: if w.style.is_preset() { w.bend } else { 50.0 },
                ..Warp::none(b)
            },
        };
        session.warp = Some(next);
        session.warp_drag = None;
        true
    })
}

fn transform_set_warp_values(
    view: Pin<&mut PictureView>,
    bend: f64,
    h_distort: f64,
    v_distort: f64,
) -> bool {
    if !bend.is_finite() || !h_distort.is_finite() || !v_distort.is_finite() {
        return false;
    }
    edit_warp(view, |session| match session.warp.as_mut() {
        Some(w) if w.style.is_preset() => {
            w.bend = bend.clamp(-100.0, 100.0);
            w.h_distort = h_distort.clamp(-100.0, 100.0);
            w.v_distort = v_distort.clamp(-100.0, 100.0);
            true
        }
        _ => false,
    })
}

fn transform_set_warp_vertical(view: Pin<&mut PictureView>, vertical: bool) -> bool {
    edit_warp(view, |session| match session.warp.as_mut() {
        Some(w) if w.style.is_preset() => {
            w.vertical = vertical;
            true
        }
        _ => false,
    })
}

fn warp_of(view: &PictureView) -> Option<&Warp> {
    view.rust().session.live.transform()?.warp.as_ref()
}

fn transform_warp_style_id(view: &PictureView) -> QString {
    warp_of(view)
        .map(|w| QString::from(w.style.id()))
        .unwrap_or_default()
}

fn transform_warp_bend(view: &PictureView) -> f64 {
    warp_of(view).map_or(0.0, |w| w.bend)
}

fn transform_warp_h_distort(view: &PictureView) -> f64 {
    warp_of(view).map_or(0.0, |w| w.h_distort)
}

fn transform_warp_v_distort(view: &PictureView) -> f64 {
    warp_of(view).map_or(0.0, |w| w.v_distort)
}

fn transform_warp_vertical(view: &PictureView) -> bool {
    warp_of(view).is_some_and(|w| w.vertical)
}

/// A grid curve at fixed `u` or fixed `v` in document space, appended to `out`.
fn push_curve(
    m: &BezierMesh,
    frame: &Frame,
    fixed_u: Option<f64>,
    fixed_v: Option<f64>,
    out: &mut Vec<f64>,
) {
    for k in 0..CURVE_POINTS {
        let s = k as f64 / (CURVE_POINTS - 1) as f64;
        let p = frame.to_doc(m.eval(fixed_u.unwrap_or(s), fixed_v.unwrap_or(s)));
        out.extend_from_slice(&p);
    }
}

/// The session's warp as a one-patch-or-more mesh and its frame.
fn warp_mesh(view: &PictureView) -> Option<(BezierMesh, Frame)> {
    let session = view.rust().session.live.transform()?;
    Some((session.warp.as_ref()?.to_mesh(1, 1), Frame::of(session)?))
}

fn grid_lines(m: &BezierMesh, frame: &Frame) -> Vec<f64> {
    let mut out = Vec::new();
    for u in &m.us {
        push_curve(m, frame, Some(*u), None, &mut out);
    }
    for v in &m.vs {
        push_curve(m, frame, None, Some(*v), &mut out);
    }
    out
}

fn transform_warp_grid_lines(view: &PictureView) -> Vec<f64> {
    warp_mesh(view).map_or_else(Vec::new, |(m, frame)| grid_lines(&m, &frame))
}

fn transform_warp_guide_lines(view: &PictureView) -> Vec<f64> {
    let Some((m, frame)) = warp_mesh(view) else {
        return Vec::new();
    };
    // Only a single patch (the default grid) draws the rule-of-thirds guides.
    if m.us.len() != 2 || m.vs.len() != 2 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for third in [1.0 / 3.0, 2.0 / 3.0] {
        push_curve(&m, &frame, Some(third), None, &mut out);
        push_curve(&m, &frame, None, Some(third), &mut out);
    }
    out
}

fn transform_warp_points(view: &PictureView) -> Vec<f64> {
    let Some(session) = view.rust().session.live.transform() else {
        return Vec::new();
    };
    let Some(m) = session
        .warp
        .as_ref()
        .filter(|w| w.style == WarpStyle::Custom)
        .and_then(|w| w.mesh.as_ref())
    else {
        return Vec::new();
    };
    let Some(frame) = Frame::of(session) else {
        return Vec::new();
    };
    let (nx, ny) = (m.nx(), m.ny());
    let dragged = match &session.warp_drag {
        Some(WarpDrag::Point { index, .. }) => Some(handle_anchor(index % nx, index / nx)),
        _ => None,
    };
    let mut out = Vec::new();
    for j in 0..ny {
        for i in 0..nx {
            if !on_section_edge(i, j) {
                continue;
            }
            let (ai, aj) = handle_anchor(i, j);
            let kind = if (ai, aj) == (i, j) {
                0.0
            } else if dragged == Some((ai, aj)) {
                2.0
            } else {
                1.0
            };
            let (p, a) = (frame.to_doc(m.point(i, j)), frame.to_doc(m.point(ai, aj)));
            out.extend_from_slice(&[kind, p[0], p[1], a[0], a[1]]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const B: [f64; 4] = [0.0, 0.0, 90.0, 60.0];
    const CORNERS: [(f64, f64); 4] = [(0.0, 0.0), (90.0, 0.0), (90.0, 60.0), (0.0, 60.0)];

    fn unmoved() -> Frame {
        Frame::new(CORNERS, CORNERS).unwrap()
    }

    #[test]
    fn handles_belong_to_their_anchor() {
        assert_eq!(handle_anchor(0, 0), (0, 0));
        assert_eq!(handle_anchor(1, 0), (0, 0));
        assert_eq!(handle_anchor(2, 0), (3, 0));
        assert_eq!(handle_anchor(0, 1), (0, 0));
        assert_eq!(handle_anchor(3, 2), (3, 3));
        assert!(!on_section_edge(1, 1));
    }

    #[test]
    fn a_press_grabs_a_point_the_surface_or_nothing() {
        let w = Warp::custom(BezierMesh::identity(B, 1, 1), B);
        // The top-left anchor is index 0; (30, 0) is the first handle.
        assert!(matches!(
            warp_grab(&w, &unmoved(), [1.0, 1.0], 1.0),
            Some(WarpDrag::Point { index: 0, .. })
        ));
        assert!(matches!(
            warp_grab(&w, &unmoved(), [30.5, 0.5], 1.0),
            Some(WarpDrag::Point { index: 1, .. })
        ));
        assert!(matches!(
            warp_grab(&w, &unmoved(), [45.0, 30.0], 1.0),
            Some(WarpDrag::Surface { .. })
        ));
        assert!(warp_grab(&w, &unmoved(), [200.0, 200.0], 1.0).is_none());
    }

    #[test]
    fn grabbing_a_preset_turns_it_into_the_same_custom_shape() {
        let w = Warp::preset(WarpStyle::Arc, 50.0, B);
        let c = as_custom(&w);
        assert_eq!(c.style, WarpStyle::Custom);
        for (x, y) in [(10.0, 10.0), (45.0, 30.0), (80.0, 55.0)] {
            let (a, b) = (w.map(x, y), c.map(x, y));
            assert!((a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6);
        }
    }

    #[test]
    fn a_preset_hides_its_points_from_the_press() {
        // Bulge's handles stand off its surface: a press on one that misses
        // the surface grabs nothing, and the preset stays.
        let w = Warp::preset(WarpStyle::Bulge, 50.0, B);
        let frame = unmoved();
        let mesh = w.to_mesh(1, 1);
        let off_surface: Vec<[f64; 2]> = mesh
            .points
            .iter()
            .copied()
            .filter(|p| surface_at(&mesh, &frame, *p).is_none())
            .collect();
        assert!(
            !off_surface.is_empty(),
            "Bulge has a handle off its surface"
        );
        for p in off_surface {
            assert!(warp_grab(&w, &frame, p, 1.0).is_none(), "{p:?}");
        }
    }

    #[test]
    fn the_grid_follows_a_perspective_box_exactly() {
        // The identity mesh seen through a perspective frame: the top edge's
        // midpoint is where the homography sends it, not where a Bezier edge
        // through the projected control points would put it.
        let quad = [(0.0, 0.0), (90.0, 18.0), (90.0, 42.0), (0.0, 60.0)];
        let frame = Frame::new(CORNERS, quad).unwrap();
        let w = Warp::custom(BezierMesh::identity(B, 1, 1), B);
        let lines = grid_lines(&w.to_mesh(1, 1), &frame);
        // The third curve is v = 0 (the top edge); its middle point is s = 0.5.
        let mid = 2 * CURVE_POINTS + CURVE_POINTS / 2;
        let want = frame.to_doc([45.0, 0.0]);
        assert!((lines[2 * mid] - want[0]).abs() < 1e-9);
        assert!((lines[2 * mid + 1] - want[1]).abs() < 1e-9);
        let corner = frame.to_doc([90.0, 60.0]);
        assert!((corner[0] - 90.0).abs() < 1e-9 && (corner[1] - 42.0).abs() < 1e-9);
    }

    #[test]
    fn a_point_drag_follows_the_pointer_through_the_frame() {
        let quad = [(0.0, 0.0), (90.0, 18.0), (90.0, 42.0), (0.0, 60.0)];
        let frame = Frame::new(CORNERS, quad).unwrap();
        let w = Warp::custom(BezierMesh::identity(B, 1, 1), B);
        let press = frame.to_doc([90.0, 60.0]);
        let Some(WarpDrag::Point { index, start, pts0 }) = warp_grab(&w, &frame, press, 1.0) else {
            panic!("the bottom-right anchor is grabbed");
        };
        let mut m = w.mesh.clone().unwrap();
        let target = [press[0] + 7.0, press[1] - 5.0];
        let p = frame.to_source(target).unwrap();
        m.drag_point(&pts0, index, [p[0] - start[0], p[1] - start[1]]);
        let shown = frame.to_doc(m.points[index]);
        assert!((shown[0] - target[0]).abs() < 1e-9 && (shown[1] - target[1]).abs() < 1e-9);
    }
}
