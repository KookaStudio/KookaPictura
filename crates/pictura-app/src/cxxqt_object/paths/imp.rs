use super::*;

use super::super::PictureViewRust;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::path::{HandleSide, VectorPath};
use pictura_core::{Document, Ruler};

/// The Paths panel row the path calls edit (CS6 shows and edits only the
/// selected path).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::cxxqt_object) enum ActivePath {
    /// No row is selected: no path is shown.
    None,
    Work,
    /// `Document::saved_paths[i]`.
    Saved(usize),
}

/// `active` resolved against `doc`: a saved index an undo removed reads as no
/// active path.
pub(in crate::cxxqt_object) fn active_in(doc: &Document, active: ActivePath) -> ActivePath {
    match active {
        ActivePath::Saved(i) if i >= doc.saved_paths.len() => ActivePath::None,
        other => other,
    }
}

pub(super) fn active_path_mut(doc: &mut Document, active: ActivePath) -> Option<&mut VectorPath> {
    match active_in(doc, active) {
        ActivePath::None => None,
        ActivePath::Work => Some(&mut doc.work_path),
        ActivePath::Saved(i) => Some(&mut doc.saved_paths[i].path),
    }
}

/// Make sure a drawing tool has a path to draw on: with no active path, a new
/// (empty) Work Path replaces the old one, as CS6's Pen does when no row is
/// selected. No history: the drawing edit records it.
pub(in crate::cxxqt_object) fn begin_drawing(mut view: Pin<&mut PictureView>) {
    let mut rust = view.as_mut().rust_mut();
    let active = rust.active_path;
    let Some(doc) = rust.session.doc.as_mut() else {
        return;
    };
    if active_in(doc, active) == ActivePath::None {
        doc.work_path = VectorPath::default();
        rust.active_path = ActivePath::Work;
    }
}

/// The path the tools edit: the active shape layer's outline while the
/// layer target is on (Path Selection, Direct Selection, the shape tools) and
/// the active layer is a shape layer; the Work Path otherwise.
pub(super) fn layer_target(rust: &PictureViewRust) -> Option<String> {
    let path = rust.layer_target.single().filter(|_| rust.path_on_layer)?;
    let layer = pictura_render::resolve_path(rust.session.doc.as_ref()?, path)?;
    pictura_render::is_shape_layer(layer).then(|| path.to_string())
}

pub(in crate::cxxqt_object) fn path(view: &PictureView) -> Option<VectorPath> {
    let rust = view.rust();
    let doc = rust.session.doc.as_ref()?;
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
pub(in crate::cxxqt_object) fn edit<T>(
    view: Pin<&mut PictureView>,
    edit: impl FnOnce(&mut VectorPath) -> T,
) -> Option<T> {
    edit_target(view, None, edit)
}

/// Apply `edit` to the target path. On a shape layer a change rewrites its
/// outline and recomposites; it keeps the shape live only when `moved_by`
/// says the edit moved it whole (the live box moves with it), and otherwise
/// turns it into a regular path.
pub(super) fn edit_target<T>(
    mut view: Pin<&mut PictureView>,
    moved_by: Option<(f64, f64)>,
    edit: impl FnOnce(&mut VectorPath) -> T,
) -> Option<T> {
    let Some(target) = layer_target(view.rust()) else {
        let mut rust = view.rust_mut();
        let active = rust.active_path;
        return rust
            .session
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
            let doc = rust.session.doc.as_mut()?;
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

pub(super) fn path_set_layer_target(mut view: Pin<&mut PictureView>, on: bool) {
    view.as_mut().rust_mut().path_on_layer = on;
}

pub(super) fn path_target_is_live_shape(view: &PictureView) -> bool {
    let rust = view.rust();
    layer_target(rust)
        .and_then(|target| pictura_render::resolve_path(rust.session.doc.as_ref()?, &target))
        .and_then(pictura_render::layer_live_shape)
        .is_some()
}

pub(super) fn path_layer_target(view: &PictureView) -> bool {
    view.rust().path_on_layer
}

pub(super) fn path_target_name(view: &PictureView) -> String {
    let rust = view.rust();
    layer_target(rust)
        .and_then(|target| pictura_render::resolve_path(rust.session.doc.as_ref()?, &target))
        .map_or_else(String::new, |layer| layer.name.clone())
}

pub(super) fn path_target_is_layer(view: &PictureView) -> bool {
    layer_target(view.rust()).is_some()
}

pub(super) fn path_convert_live_shape(mut view: Pin<&mut PictureView>) -> bool {
    if !path_target_is_live_shape(&view) {
        return false;
    }
    let mut rust = view.as_mut().rust_mut();
    let target = layer_target(&rust);
    target
        .and_then(|target| pictura_render::resolve_path_mut(rust.session.doc.as_mut()?, &target))
        .map(|layer| pictura_render::set_layer_live_shape(layer, None))
        .is_some()
}

pub(super) fn commit(mut view: Pin<&mut PictureView>, label: &str) {
    view.as_mut().edit(label, |_| Edit::Meta);
}

pub(super) fn index(i: i32) -> Option<usize> {
    usize::try_from(i).ok()
}

pub(super) fn snap(from: (f64, f64), to: (f64, f64), constrain: bool) -> (f64, f64) {
    if constrain {
        Ruler::constrained(from, to).b
    } else {
        to
    }
}

pub(super) fn path_subpath_count(view: &PictureView) -> i32 {
    path(view).map_or(0, |p| p.subpaths.len() as i32)
}

pub(super) fn path_point_count(view: &PictureView, sp: i32) -> i32 {
    index(sp)
        .and_then(|sp| Some(path(view)?.subpaths.get(sp)?.points.len() as i32))
        .unwrap_or(0)
}

pub(super) fn path_subpath_closed(view: &PictureView, sp: i32) -> bool {
    index(sp)
        .and_then(|sp| Some(path(view)?.subpaths.get(sp)?.closed))
        .unwrap_or(false)
}

pub(super) fn path_point(view: &PictureView, sp: i32, pt: i32) -> Vec<f64> {
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

pub(super) fn path_editing_subpath(view: &PictureView) -> i32 {
    path(view)
        .and_then(|p| p.editing_subpath())
        .map_or(-1, |s| s as i32)
}

pub(super) fn path_append_corner(
    mut view: Pin<&mut PictureView>,
    x: f64,
    y: f64,
    constrain: bool,
) -> bool {
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

pub(super) fn path_update_last_handle(
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

pub(super) fn path_commit_anchor(view: Pin<&mut PictureView>) {
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

pub(super) fn path_close(mut view: Pin<&mut PictureView>) -> bool {
    let closed = edit(view.as_mut(), VectorPath::close_active_subpath) == Some(true);
    if closed {
        commit(view, "Close Path");
    }
    closed
}

pub(super) fn path_finish(view: Pin<&mut PictureView>) {
    edit(view, VectorPath::finish_editing);
}

pub(super) fn path_gesture_begin(mut view: Pin<&mut PictureView>) {
    view.as_mut().rust_mut().begin_path_gesture();
}

impl PictureViewRust {
    /// Back up the path a gesture is about to edit, so an abort can restore it.
    /// The `None` row backs up the Work Path slot: `begin_drawing` replaces it
    /// when a drawing press finds no active path. A shape layer's outline lives
    /// in the layer, so it is backed up there.
    pub(in crate::cxxqt_object) fn begin_path_gesture(&mut self) {
        if !self.slot_allows(super::super::document_session::SessionKind::Path) {
            return;
        }
        if let Some(target) = layer_target(self) {
            let backup = self.session.doc.as_ref().and_then(|doc| {
                let (width, height) = (doc.width, doc.height);
                pictura_render::resolve_path(doc, &target)
                    .and_then(|layer| pictura_render::layer_shape_paths(layer, width, height))
                    .map(
                        |subpaths| super::super::document_session::PathRestore::Layer {
                            target,
                            subpaths,
                        },
                    )
            });
            match backup {
                Some(backup) => self.session.live.set_path(backup),
                None => self.session.live.clear_path(),
            }
            return;
        }
        let active = self.active_path;
        let backup = self.session.doc.as_ref().map(|doc| {
            let path = match active_in(doc, active) {
                ActivePath::None | ActivePath::Work => doc.work_path.clone(),
                ActivePath::Saved(i) => doc.saved_paths[i].path.clone(),
            };
            super::super::document_session::PathRestore::WorkOrSaved { active, path }
        });
        match backup {
            Some(backup) => self.session.live.set_path(backup),
            None => self.session.live.clear_path(),
        }
    }
}

pub(super) fn path_gesture_abort(mut view: Pin<&mut PictureView>) -> bool {
    let change = view.as_mut().rust_mut().cancel_path_gesture();
    if !change.changed_display() {
        return false;
    }
    view.as_mut().apply_session_change(change);
    true
}

pub(super) fn path_gesture_end(mut view: Pin<&mut PictureView>) {
    view.as_mut().rust_mut().session.live.clear_path();
}

pub(super) fn path_resume(view: Pin<&mut PictureView>, sp: i32, pt: i32) -> bool {
    let Some((sp, pt)) = index(sp).zip(index(pt)) else {
        return false;
    };
    edit(view, |p| p.resume_at(sp, pt)) == Some(true)
}

pub(super) fn path_hit_anchor(view: &PictureView, x: f64, y: f64, radius: f64) -> Vec<i32> {
    path(view)
        .and_then(|p| p.hit_anchor(x, y, radius.max(0.0)))
        .map_or_else(Vec::new, |(s, p)| vec![s as i32, p as i32])
}

pub(super) fn path_hit_handle(view: &PictureView, x: f64, y: f64, radius: f64) -> Vec<i32> {
    path(view)
        .and_then(|p| p.hit_handle(x, y, radius.max(0.0)))
        .map_or_else(Vec::new, |(s, p, side)| {
            vec![s as i32, p as i32, i32::from(side == HandleSide::Out)]
        })
}

pub(super) fn path_hit_segment(view: &PictureView, x: f64, y: f64, radius: f64) -> Vec<f64> {
    path(view)
        .and_then(|p| p.hit_segment(x, y, radius.max(0.0)))
        .map_or_else(Vec::new, |(s, seg, t)| vec![s as f64, seg as f64, t])
}

pub(super) fn path_insert_anchor(
    mut view: Pin<&mut PictureView>,
    sp: i32,
    seg: i32,
    t: f64,
) -> bool {
    let Some((sp, seg)) = index(sp).zip(index(seg)) else {
        return false;
    };
    let inserted = edit(view.as_mut(), |p| p.insert_anchor(sp, seg, t)) == Some(true);
    if inserted {
        commit(view, "Add Anchor Point");
    }
    inserted
}

pub(super) fn path_delete_anchor(mut view: Pin<&mut PictureView>, sp: i32, pt: i32) -> bool {
    let Some((sp, pt)) = index(sp).zip(index(pt)) else {
        return false;
    };
    let deleted = edit(view.as_mut(), |p| p.delete_anchor(sp, pt)) == Some(true);
    if deleted {
        commit(view, "Delete Anchor Point");
    }
    deleted
}

pub(super) fn path_set_corner(mut view: Pin<&mut PictureView>, sp: i32, pt: i32) -> bool {
    let Some((sp, pt)) = index(sp).zip(index(pt)) else {
        return false;
    };
    let converted = edit(view.as_mut(), |p| p.set_corner(sp, pt)) == Some(true);
    if converted {
        commit(view, "Convert Point");
    }
    converted
}

pub(super) fn path_drag_new_handles(
    view: Pin<&mut PictureView>,
    sp: i32,
    pt: i32,
    x: f64,
    y: f64,
) -> bool {
    let Some((sp, pt)) = index(sp).zip(index(pt)) else {
        return false;
    };
    edit(view, |p| p.drag_new_handles(sp, pt, x, y)) == Some(true)
}

pub(super) fn path_move_handle(
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

pub(super) fn path_commit_convert(view: Pin<&mut PictureView>) {
    commit(view, "Convert Point");
}

pub(super) fn path_add_freeform(
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

pub(super) fn path_hit_subpath(view: &PictureView, x: f64, y: f64, radius: f64) -> i32 {
    path(view)
        .and_then(|p| p.hit_subpath(x, y, radius.max(0.0)))
        .map_or(-1, |s| s as i32)
}

pub(super) fn path_subpath_bounds(view: &PictureView, sp: i32) -> Vec<f64> {
    index(sp)
        .and_then(|sp| path(view)?.subpath_bounds(sp))
        .map_or_else(Vec::new, |(l, t, r, b)| vec![l, t, r, b])
}

pub(super) fn path_move_anchor(
    view: Pin<&mut PictureView>,
    sp: i32,
    pt: i32,
    x: f64,
    y: f64,
) -> bool {
    let Some((sp, pt)) = index(sp).zip(index(pt)) else {
        return false;
    };
    edit(view, |p| p.move_anchor(sp, pt, x, y)) == Some(true)
}

pub(super) fn path_move_subpath(view: Pin<&mut PictureView>, sp: i32, dx: f64, dy: f64) -> bool {
    let Some(sp) = index(sp) else {
        return false;
    };
    edit_target(view, Some((dx, dy)), |p| p.move_subpath(sp, dx, dy)) == Some(true)
}

pub(super) fn path_duplicate_subpath(view: Pin<&mut PictureView>, sp: i32) -> i32 {
    index(sp)
        .and_then(|sp| edit(view, |p| p.duplicate_subpath(sp))?)
        .map_or(-1, |s| s as i32)
}

pub(super) fn path_commit_drag(view: Pin<&mut PictureView>, label: &str) {
    commit(view, label);
}

pub(super) fn path_remove_subpath(mut view: Pin<&mut PictureView>, sp: i32) -> bool {
    let Some(sp) = index(sp) else {
        return false;
    };
    let removed = edit(view.as_mut(), |p| p.remove_subpath(sp)) == Some(true);
    if removed {
        commit(view, "Delete Path");
    }
    removed
}
