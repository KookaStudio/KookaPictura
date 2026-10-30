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

mod imp;

use super::qobject::PictureView;
pub(super) use imp::*;

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
