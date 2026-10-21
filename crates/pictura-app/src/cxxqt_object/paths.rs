//! The Work Path commands of the Pen tool group (Pen, Freeform Pen, Add /
//! Delete Anchor Point, Convert Point) and the path selection tools (Path
//! Selection, Direct Selection). Free functions over a [`PictureView`]
//! (their own bridge, so the `PictureView` declaration list does not grow).
//!
//! The calls edit the Paths panel's active path (the Work Path or a saved
//! path, see [`ActivePath`]), or the active shape layer's outline while
//! [`path_set_layer_target`] is on: a change there rewrites the layer's `vmsk`
//! and recomposites, and every edit but a whole-component move turns a live
//! shape into a regular path. With no active path the calls see an empty path,
//! and drawing starts a new Work Path in place of the old one.
//!
//! Atomic edits (insert, delete, close, convert-click, freeform) record one
//! history state and emit `changed`. A drag's live steps (placing an anchor and
//! pulling its handles, dragging a handle, anchor, or component) record nothing
//! until the handler calls `path_commit_anchor` / `path_commit_convert` /
//! `path_commit_drag` on release. Ending or
//! resuming a drawing session records nothing.
//!
//! [`PictureView`]: super::qobject::PictureView

use super::qobject::PictureView;
use super::PictureViewRust;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::path::{HandleSide, VectorPath};
use pictura_core::{Document, Ruler};

/// The Paths panel row the path calls edit (CS6 shows and edits only the
/// selected path).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ActivePath {
    /// No row is selected: no path is shown.
    None,
    Work,
    /// `Document::saved_paths[i]`.
    Saved(usize),
}

/// `active` resolved against `doc`: a saved index an undo removed reads as no
/// active path.
pub(super) fn active_in(doc: &Document, active: ActivePath) -> ActivePath {
    match active {
        ActivePath::Saved(i) if i >= doc.saved_paths.len() => ActivePath::None,
        other => other,
    }
}

fn active_path_mut(doc: &mut Document, active: ActivePath) -> Option<&mut VectorPath> {
    match active_in(doc, active) {
        ActivePath::None => None,
        ActivePath::Work => Some(&mut doc.work_path),
        ActivePath::Saved(i) => Some(&mut doc.saved_paths[i].path),
    }
}

/// Make sure a drawing tool has a path to draw on: with no active path, a new
/// (empty) Work Path replaces the old one, as CS6's Pen does when no row is
/// selected. No history: the drawing edit records it.
pub(super) fn begin_drawing(mut view: Pin<&mut PictureView>) {
    let mut rust = view.as_mut().rust_mut();
    let active = rust.active_path;
    let Some(doc) = rust.doc.as_mut() else {
        return;
    };
    if active_in(doc, active) == ActivePath::None {
        doc.work_path = VectorPath::default();
        rust.active_path = ActivePath::Work;
    }
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Point the path calls at the active shape layer's outline (`on`, while the active layer is a shape layer) or always at the Work Path. Off by default; the Pen tools leave it off.
        fn path_set_layer_target(view: Pin<&mut PictureView>, on: bool);

        /// Whether the layer target is on (see `path_set_layer_target`).
        fn path_layer_target(view: &PictureView) -> bool;

        /// Whether the path calls currently edit a shape layer's outline.
        fn path_target_is_layer(view: &PictureView) -> bool;

        /// The targeted shape layer's name; empty while the calls edit the Work Path.
        fn path_target_name(view: &PictureView) -> String;

        /// Whether the targeted shape layer is still a live shape (it carries its parametric origin).
        fn path_target_is_live_shape(view: &PictureView) -> bool;

        /// Turn the targeted live shape into a regular path. No history: the edit that follows records it, and undoing that edit restores the live shape. False when there is no live shape.
        fn path_convert_live_shape(view: Pin<&mut PictureView>) -> bool;

        /// Subpaths in the targeted path; 0 without a document.
        fn path_subpath_count(view: &PictureView) -> i32;

        /// Anchor points on subpath `sp`; 0 when out of range.
        fn path_point_count(view: &PictureView, sp: i32) -> i32;

        /// Whether subpath `sp` is closed.
        fn path_subpath_closed(view: &PictureView, sp: i32) -> bool;

        /// Point `pt` of subpath `sp` as `[ax, ay, has_in, ix, iy, has_out, ox, oy, smooth]` (flags 0/1); empty when out of range.
        fn path_point(view: &PictureView, sp: i32, pt: i32) -> Vec<f64>;

        /// The subpath the Pen is extending, or -1 between drawing sessions.
        fn path_editing_subpath(view: &PictureView) -> i32;

        /// Live: append a corner anchor (starting a subpath if none is being drawn), snapped to 45° from the previous anchor when `constrain`. No history until `path_commit_anchor`.
        fn path_append_corner(view: Pin<&mut PictureView>, x: f64, y: f64, constrain: bool)
            -> bool;

        /// Live: drag the last anchor's handles to `(x, y)` (snapped to 45° when `constrain`); `independent` (Alt) sets the outgoing handle only.
        fn path_update_last_handle(
            view: Pin<&mut PictureView>,
            x: f64,
            y: f64,
            independent: bool,
            constrain: bool,
        ) -> bool;

        /// Record the anchor just placed: "New Work Path" when it is the path's only point, otherwise "Add Anchor Point".
        fn path_commit_anchor(view: Pin<&mut PictureView>);

        /// Close the subpath being drawn and record one "Close Path" state; false (no state) under two points or with nothing being drawn.
        fn path_close(view: Pin<&mut PictureView>) -> bool;

        /// End the drawing session, leaving the subpath open. No history.
        fn path_finish(view: Pin<&mut PictureView>);

        /// Capture the active path as the live gesture's backup; Edit > Undo
        /// restores it when the gesture is cancelled. No history.
        fn path_gesture_begin(view: Pin<&mut PictureView>);

        /// Restore the path captured by `path_gesture_begin` and clear the
        /// backup; false when none. No history.
        fn path_gesture_abort(view: Pin<&mut PictureView>) -> bool;

        /// Drop the gesture backup after a commit; no-op when none.
        fn path_gesture_end(view: Pin<&mut PictureView>);

        /// Resume drawing from an open subpath's endpoint `pt`. No history; false for a closed subpath or an interior point.
        fn path_resume(view: Pin<&mut PictureView>, sp: i32, pt: i32) -> bool;

        /// The anchor nearest `(x, y)` within `radius` as `[sp, pt]`; empty when none.
        fn path_hit_anchor(view: &PictureView, x: f64, y: f64, radius: f64) -> Vec<i32>;

        /// The handle nearest `(x, y)` within `radius` as `[sp, pt, side]` (0 in, 1 out); empty when none.
        fn path_hit_handle(view: &PictureView, x: f64, y: f64, radius: f64) -> Vec<i32>;

        /// The segment point nearest `(x, y)` within `radius` as `[sp, seg, t]`; empty when none.
        fn path_hit_segment(view: &PictureView, x: f64, y: f64, radius: f64) -> Vec<f64>;

        /// Split segment `seg` of subpath `sp` at `t`, keeping the shape; records one "Add Anchor Point" state.
        fn path_insert_anchor(view: Pin<&mut PictureView>, sp: i32, seg: i32, t: f64) -> bool;

        /// Remove anchor `pt` of subpath `sp`; records one "Delete Anchor Point" state.
        fn path_delete_anchor(view: Pin<&mut PictureView>, sp: i32, pt: i32) -> bool;

        /// Convert Point's click: make the anchor a plain corner and record one "Convert Point" state; false (no state) when it already was one.
        fn path_set_corner(view: Pin<&mut PictureView>, sp: i32, pt: i32) -> bool;

        /// Live: Convert Point's drag from an anchor, pulling a symmetric pair of handles to `(x, y)`.
        fn path_drag_new_handles(
            view: Pin<&mut PictureView>,
            sp: i32,
            pt: i32,
            x: f64,
            y: f64,
        ) -> bool;

        /// Live: drag handle `side` (0 in, 1 out) of point `pt` to `(x, y)`; `independent` breaks a smooth point.
        fn path_move_handle(
            view: Pin<&mut PictureView>,
            sp: i32,
            pt: i32,
            side: i32,
            x: f64,
            y: f64,
            independent: bool,
        ) -> bool;

        /// Record a Convert Point drag as one "Convert Point" state.
        fn path_commit_convert(view: Pin<&mut PictureView>);

        /// The subpath (path component) under `(x, y)`: within `radius` of its outline or a lone anchor, else inside a closed one; -1 when none.
        fn path_hit_subpath(view: &PictureView, x: f64, y: f64, radius: f64) -> i32;

        /// Subpath `sp`'s curve bounds as `[left, top, right, bottom]`; empty when out of range or empty.
        fn path_subpath_bounds(view: &PictureView, sp: i32) -> Vec<f64>;

        /// Live: move anchor `pt` of subpath `sp` to `(x, y)`, carrying its handles.
        fn path_move_anchor(view: Pin<&mut PictureView>, sp: i32, pt: i32, x: f64, y: f64) -> bool;

        /// Live: move subpath `sp` by `(dx, dy)`.
        fn path_move_subpath(view: Pin<&mut PictureView>, sp: i32, dx: f64, dy: f64) -> bool;

        /// Live: append a copy of subpath `sp` (Alt-drag with Path Selection); the copy's index, or -1.
        fn path_duplicate_subpath(view: Pin<&mut PictureView>, sp: i32) -> i32;

        /// Record a Path Selection or Direct Selection drag as one state named `label`.
        fn path_commit_drag(view: Pin<&mut PictureView>, label: &str);

        /// Remove subpath `sp` and record one "Delete Path" state.
        fn path_remove_subpath(view: Pin<&mut PictureView>, sp: i32) -> bool;

        /// Add a Freeform Pen drag (`[x0, y0, x1, y1, …]`) as a new subpath of corner anchors within `tolerance` px, closed when `close`; records one "Freeform Pen" state. False (no state) when fewer than two anchors survive.
        fn path_add_freeform(
            view: Pin<&mut PictureView>,
            points: &[f64],
            tolerance: f64,
            close: bool,
        ) -> bool;
    }
}

/// The path the tools edit: the active shape layer's outline while the
/// layer target is on (Path Selection, Direct Selection, the shape tools) and
/// the active layer is a shape layer; the Work Path otherwise.
fn layer_target(rust: &PictureViewRust) -> Option<String> {
    let path = rust.layer_target.single().filter(|_| rust.path_on_layer)?;
    let layer = pictura_render::resolve_path(rust.doc.as_ref()?, path)?;
    pictura_render::is_shape_layer(layer).then(|| path.to_string())
}

pub(super) fn path(view: &PictureView) -> Option<VectorPath> {
    let rust = view.rust();
    let doc = rust.doc.as_ref()?;
    let Some(target) = layer_target(rust) else {
        return Some(match active_in(doc, rust.active_path) {
            ActivePath::None => VectorPath::default(),
            ActivePath::Work => doc.work_path.clone(),
            ActivePath::Saved(i) => doc.saved_paths[i].path.clone(),
        });
    };
    let layer = pictura_render::resolve_path(doc, &target)?;
    let mut path = VectorPath::default();
    for subpath in pictura_render::layer_shape_paths(layer, doc.width, doc.height)? {
        path.add_subpath(subpath);
    }
    Some(path)
}

/// Apply `edit` to the active path; `None` without a document or active path.
pub(super) fn edit<T>(
    view: Pin<&mut PictureView>,
    edit: impl FnOnce(&mut VectorPath) -> T,
) -> Option<T> {
    edit_target(view, None, edit)
}

/// Apply `edit` to the target path. On a shape layer a change rewrites its
/// outline and recomposites; it keeps the shape live only when `moved_by`
/// says the edit moved it whole (the live box moves with it), and otherwise
/// turns it into a regular path.
fn edit_target<T>(
    mut view: Pin<&mut PictureView>,
    moved_by: Option<(f64, f64)>,
    edit: impl FnOnce(&mut VectorPath) -> T,
) -> Option<T> {
    let Some(target) = layer_target(view.rust()) else {
        let mut rust = view.rust_mut();
        let active = rust.active_path;
        return rust
            .doc
            .as_mut()
            .and_then(|doc| active_path_mut(doc, active))
            .map(edit);
    };
    let before = path(&view)?;
    let mut after = before.clone();
    let out = edit(&mut after);
    if after != before {
        {
            let mut rust = view.as_mut().rust_mut();
            let doc = rust.doc.as_mut()?;
            let (width, height) = (doc.width, doc.height);
            let layer = pictura_render::resolve_path_mut(doc, &target)?;
            pictura_render::set_layer_shape_paths(layer, &after.subpaths, width, height);
            let live = pictura_render::layer_live_shape(layer)
                .zip(moved_by)
                .map(|(live, (dx, dy))| live.translated(dx, dy));
            pictura_render::set_layer_live_shape(layer, live.as_ref());
        }
        view.as_mut().recomposite();
    }
    Some(out)
}

fn path_set_layer_target(mut view: Pin<&mut PictureView>, on: bool) {
    view.as_mut().rust_mut().path_on_layer = on;
}

fn path_target_is_live_shape(view: &PictureView) -> bool {
    let rust = view.rust();
    layer_target(rust)
        .and_then(|target| pictura_render::resolve_path(rust.doc.as_ref()?, &target))
        .and_then(pictura_render::layer_live_shape)
        .is_some()
}

fn path_layer_target(view: &PictureView) -> bool {
    view.rust().path_on_layer
}

fn path_target_name(view: &PictureView) -> String {
    let rust = view.rust();
    layer_target(rust)
        .and_then(|target| pictura_render::resolve_path(rust.doc.as_ref()?, &target))
        .map_or_else(String::new, |layer| layer.name.clone())
}

fn path_target_is_layer(view: &PictureView) -> bool {
    layer_target(view.rust()).is_some()
}

fn path_convert_live_shape(mut view: Pin<&mut PictureView>) -> bool {
    if !path_target_is_live_shape(&view) {
        return false;
    }
    let mut rust = view.as_mut().rust_mut();
    let target = layer_target(&rust);
    target
        .and_then(|target| pictura_render::resolve_path_mut(rust.doc.as_mut()?, &target))
        .map(|layer| pictura_render::set_layer_live_shape(layer, None))
        .is_some()
}

fn commit(mut view: Pin<&mut PictureView>, label: &str) {
    view.as_mut().edit(label, |_| Edit::Meta);
}

fn index(i: i32) -> Option<usize> {
    usize::try_from(i).ok()
}

fn snap(from: (f64, f64), to: (f64, f64), constrain: bool) -> (f64, f64) {
    if constrain {
        Ruler::constrained(from, to).b
    } else {
        to
    }
}

fn path_subpath_count(view: &PictureView) -> i32 {
    path(view).map_or(0, |p| p.subpaths.len() as i32)
}

fn path_point_count(view: &PictureView, sp: i32) -> i32 {
    index(sp)
        .and_then(|sp| Some(path(view)?.subpaths.get(sp)?.points.len() as i32))
        .unwrap_or(0)
}

fn path_subpath_closed(view: &PictureView, sp: i32) -> bool {
    index(sp)
        .and_then(|sp| Some(path(view)?.subpaths.get(sp)?.closed))
        .unwrap_or(false)
}

fn path_point(view: &PictureView, sp: i32, pt: i32) -> Vec<f64> {
    let point = index(sp)
        .zip(index(pt))
        .and_then(|(sp, pt)| path(view)?.subpaths.get(sp)?.points.get(pt).copied());
    let Some(p) = point else {
        return Vec::new();
    };
    let flag = |b: bool| if b { 1.0 } else { 0.0 };
    let (ix, iy) = p.in_handle.unwrap_or(p.anchor);
    let (ox, oy) = p.out_handle.unwrap_or(p.anchor);
    vec![
        p.anchor.0,
        p.anchor.1,
        flag(p.in_handle.is_some()),
        ix,
        iy,
        flag(p.out_handle.is_some()),
        ox,
        oy,
        flag(p.smooth),
    ]
}

fn path_editing_subpath(view: &PictureView) -> i32 {
    path(view)
        .and_then(|p| p.editing_subpath())
        .map_or(-1, |s| s as i32)
}

fn path_append_corner(mut view: Pin<&mut PictureView>, x: f64, y: f64, constrain: bool) -> bool {
    begin_drawing(view.as_mut());
    edit(view, |p| {
        let (x, y) = match p.last_anchor() {
            Some(from) => snap(from, (x, y), constrain),
            None => (x, y),
        };
        p.append_corner(x, y);
    })
    .is_some()
}

fn path_update_last_handle(
    view: Pin<&mut PictureView>,
    x: f64,
    y: f64,
    independent: bool,
    constrain: bool,
) -> bool {
    edit(view, |p| {
        let Some(anchor) = p.last_anchor() else {
            return false;
        };
        let (x, y) = snap(anchor, (x, y), constrain);
        p.update_last_handle(x, y, independent)
    }) == Some(true)
}

fn path_commit_anchor(view: Pin<&mut PictureView>) {
    let only =
        path(&view).is_some_and(|p| p.subpaths.iter().map(|s| s.points.len()).sum::<usize>() == 1);
    commit(
        view,
        if only {
            "New Work Path"
        } else {
            "Add Anchor Point"
        },
    );
}

fn path_close(mut view: Pin<&mut PictureView>) -> bool {
    let closed = edit(view.as_mut(), VectorPath::close_active_subpath) == Some(true);
    if closed {
        commit(view, "Close Path");
    }
    closed
}

fn path_finish(view: Pin<&mut PictureView>) {
    edit(view, VectorPath::finish_editing);
}

fn path_gesture_begin(mut view: Pin<&mut PictureView>) {
    let mut rust = view.as_mut().rust_mut();
    let active = rust.active_path;
    // The `None` row backs up the Work Path slot: `begin_drawing` replaces it
    // when a drawing press finds no active path.
    let backup = rust.doc.as_ref().map(|doc| {
        let path = match active_in(doc, active) {
            ActivePath::None | ActivePath::Work => doc.work_path.clone(),
            ActivePath::Saved(i) => doc.saved_paths[i].path.clone(),
        };
        (active, path)
    });
    rust.path_gesture = backup;
}

fn path_gesture_abort(mut view: Pin<&mut PictureView>) -> bool {
    let mut rust = view.as_mut().rust_mut();
    let Some((active, path)) = rust.path_gesture.take() else {
        return false;
    };
    {
        let Some(doc) = rust.doc.as_mut() else {
            return false;
        };
        match active {
            ActivePath::None | ActivePath::Work => doc.work_path = path,
            ActivePath::Saved(i) => match doc.saved_paths.get_mut(i) {
                Some(saved) => saved.path = path,
                None => return false,
            },
        }
    }
    rust.active_path = active;
    view.as_mut().changed();
    true
}

fn path_gesture_end(mut view: Pin<&mut PictureView>) {
    view.as_mut().rust_mut().path_gesture = None;
}

fn path_resume(view: Pin<&mut PictureView>, sp: i32, pt: i32) -> bool {
    let Some((sp, pt)) = index(sp).zip(index(pt)) else {
        return false;
    };
    edit(view, |p| p.resume_at(sp, pt)) == Some(true)
}

fn path_hit_anchor(view: &PictureView, x: f64, y: f64, radius: f64) -> Vec<i32> {
    path(view)
        .and_then(|p| p.hit_anchor(x, y, radius.max(0.0)))
        .map_or_else(Vec::new, |(s, p)| vec![s as i32, p as i32])
}

fn path_hit_handle(view: &PictureView, x: f64, y: f64, radius: f64) -> Vec<i32> {
    path(view)
        .and_then(|p| p.hit_handle(x, y, radius.max(0.0)))
        .map_or_else(Vec::new, |(s, p, side)| {
            vec![s as i32, p as i32, i32::from(side == HandleSide::Out)]
        })
}

fn path_hit_segment(view: &PictureView, x: f64, y: f64, radius: f64) -> Vec<f64> {
    path(view)
        .and_then(|p| p.hit_segment(x, y, radius.max(0.0)))
        .map_or_else(Vec::new, |(s, seg, t)| vec![s as f64, seg as f64, t])
}

fn path_insert_anchor(mut view: Pin<&mut PictureView>, sp: i32, seg: i32, t: f64) -> bool {
    let Some((sp, seg)) = index(sp).zip(index(seg)) else {
        return false;
    };
    let inserted = edit(view.as_mut(), |p| p.insert_anchor(sp, seg, t)) == Some(true);
    if inserted {
        commit(view, "Add Anchor Point");
    }
    inserted
}

fn path_delete_anchor(mut view: Pin<&mut PictureView>, sp: i32, pt: i32) -> bool {
    let Some((sp, pt)) = index(sp).zip(index(pt)) else {
        return false;
    };
    let deleted = edit(view.as_mut(), |p| p.delete_anchor(sp, pt)) == Some(true);
    if deleted {
        commit(view, "Delete Anchor Point");
    }
    deleted
}

fn path_set_corner(mut view: Pin<&mut PictureView>, sp: i32, pt: i32) -> bool {
    let Some((sp, pt)) = index(sp).zip(index(pt)) else {
        return false;
    };
    let converted = edit(view.as_mut(), |p| p.set_corner(sp, pt)) == Some(true);
    if converted {
        commit(view, "Convert Point");
    }
    converted
}

fn path_drag_new_handles(view: Pin<&mut PictureView>, sp: i32, pt: i32, x: f64, y: f64) -> bool {
    let Some((sp, pt)) = index(sp).zip(index(pt)) else {
        return false;
    };
    edit(view, |p| p.drag_new_handles(sp, pt, x, y)) == Some(true)
}

fn path_move_handle(
    view: Pin<&mut PictureView>,
    sp: i32,
    pt: i32,
    side: i32,
    x: f64,
    y: f64,
    independent: bool,
) -> bool {
    let Some((sp, pt)) = index(sp).zip(index(pt)) else {
        return false;
    };
    let side = if side == 0 {
        HandleSide::In
    } else {
        HandleSide::Out
    };
    edit(view, |p| p.move_handle(sp, pt, side, x, y, independent)) == Some(true)
}

fn path_commit_convert(view: Pin<&mut PictureView>) {
    commit(view, "Convert Point");
}

fn path_add_freeform(
    mut view: Pin<&mut PictureView>,
    points: &[f64],
    tolerance: f64,
    close: bool,
) -> bool {
    let points: Vec<(f64, f64)> = points
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&[x, y]| (x, y))
        .collect();
    begin_drawing(view.as_mut());
    let added = edit(view.as_mut(), |p| {
        p.add_freeform(&points, tolerance.clamp(0.5, 10.0), close)
    }) == Some(true);
    if added {
        commit(view, "Freeform Pen");
    }
    added
}

fn path_hit_subpath(view: &PictureView, x: f64, y: f64, radius: f64) -> i32 {
    path(view)
        .and_then(|p| p.hit_subpath(x, y, radius.max(0.0)))
        .map_or(-1, |s| s as i32)
}

fn path_subpath_bounds(view: &PictureView, sp: i32) -> Vec<f64> {
    index(sp)
        .and_then(|sp| path(view)?.subpath_bounds(sp))
        .map_or_else(Vec::new, |(l, t, r, b)| vec![l, t, r, b])
}

fn path_move_anchor(view: Pin<&mut PictureView>, sp: i32, pt: i32, x: f64, y: f64) -> bool {
    let Some((sp, pt)) = index(sp).zip(index(pt)) else {
        return false;
    };
    edit(view, |p| p.move_anchor(sp, pt, x, y)) == Some(true)
}

fn path_move_subpath(view: Pin<&mut PictureView>, sp: i32, dx: f64, dy: f64) -> bool {
    let Some(sp) = index(sp) else {
        return false;
    };
    edit_target(view, Some((dx, dy)), |p| p.move_subpath(sp, dx, dy)) == Some(true)
}

fn path_duplicate_subpath(view: Pin<&mut PictureView>, sp: i32) -> i32 {
    index(sp)
        .and_then(|sp| edit(view, |p| p.duplicate_subpath(sp))?)
        .map_or(-1, |s| s as i32)
}

fn path_commit_drag(view: Pin<&mut PictureView>, label: &str) {
    commit(view, label);
}

fn path_remove_subpath(mut view: Pin<&mut PictureView>, sp: i32) -> bool {
    let Some(sp) = index(sp) else {
        return false;
    };
    let removed = edit(view.as_mut(), |p| p.remove_subpath(sp)) == Some(true);
    if removed {
        commit(view, "Delete Path");
    }
    removed
}
