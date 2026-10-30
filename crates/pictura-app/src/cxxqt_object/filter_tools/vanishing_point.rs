//! Filter ▸ Vanishing Point (#349): the dialog's session (planes here, the
//! retouch tools in `vanishing_point_retouch.rs`) and its kept edits. OK
//! keeps the planes on the document and, when the session changed pixels or
//! Render Grids is on, keeps the edit the `vanishing-point` kind's one slot
//! names by id, so the commit, Fade and Last Filter run the ordinary filter
//! path, as Liquify's mesh does. Free functions (their own bridge); the
//! dialog is modal, so one session at a time.

mod imp;

use super::super::qobject::PictureView;
pub(in crate::cxxqt_object) use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Open a session over the active layer, on the planes the document
        /// kept, with a working copy of the layer and the document composite
        /// without it. False when the document is not 8-bit RGB or Grayscale
        /// or there is no pixel layer to edit.
        fn vp_begin(view: Pin<&mut PictureView>) -> bool;

        /// Replace the session's planes with those kept edit `id` was made
        /// from (Last Filter Settings). False when it is no longer kept or
        /// was made on a document of another size.
        fn vp_load(id: i32) -> bool;

        /// The session's document size, `[width, height]`; empty with no
        /// session.
        fn vp_geometry() -> Vec<i32>;

        fn vp_plane_count() -> i32;

        /// Plane `index`'s four corners, `x, y` pairs in document pixels.
        fn vp_plane_corners(index: i32) -> Vec<f64>;

        /// Plane `index`'s four edge-middle nodes, `x, y` pairs.
        fn vp_plane_edge_nodes(index: i32) -> Vec<f64>;

        /// 0 valid (blue), 1 no aspect ratio (red), 2 no camera (yellow).
        fn vp_plane_validity(index: i32) -> i32;

        /// Plane `index`'s grid lines, `x0, y0, x1, y1` per line.
        fn vp_plane_grid_lines(index: i32) -> Vec<f64>;

        /// How a plane on these four `x, y` corners would read (0–2).
        fn vp_quad_validity(points: &[f64]) -> i32;

        /// Add a plane on four `x, y` corners and make it active; its index,
        /// or -1 for a malformed list.
        fn vp_create(points: &[f64]) -> i32;

        /// The active plane, or -1.
        fn vp_active() -> i32;
        fn vp_set_active(index: i32);

        /// What document point `(x, y)` is over within `tolerance` px:
        /// `[plane, part, node]`, part 0 a corner, 1 an edge node, 2 the
        /// inside; empty over nothing.
        fn vp_hit(x: f64, y: f64, tolerance: f64) -> Vec<i32>;

        /// Ctrl-click: make the next plane under `(x, y)` active; its index
        /// or -1.
        fn vp_cycle(x: f64, y: f64) -> i32;

        /// Start a drag of `part`/`node` of `index` from `(x, y)`; `mode` 0
        /// plain, 1 tear off (Ctrl on an edge node), 2 turn (Alt on a far
        /// edge node). False when the gesture does not apply.
        fn vp_drag_begin(index: i32, part: i32, node: i32, mode: i32, x: f64, y: f64) -> bool;
        fn vp_drag_to(x: f64, y: f64);
        fn vp_drag_end();

        /// The active plane family's Grid Size (the default with none).
        fn vp_grid_size() -> f64;
        fn vp_set_grid_size(size: f64);

        /// The active plane's angle to its parent, or 0 for a root.
        fn vp_angle() -> f64;
        /// Whether the active plane may turn: a related plane with no
        /// related planes of its own.
        fn vp_angle_editable() -> bool;
        fn vp_set_angle(degrees: f64) -> bool;

        /// Delete the active plane and the planes hinged on it.
        fn vp_delete_active() -> bool;

        fn vp_undo() -> bool;
        fn vp_redo() -> bool;

        /// End the session. With `keep` any floating selection is dropped,
        /// the document keeps the planes (marked modified when they changed,
        /// so a save writes them), and when pixels changed or
        /// `render_grids` is on the edit is kept and its id returned for the
        /// `vanishing-point` slot; otherwise 0.
        fn vp_finish(view: Pin<&mut PictureView>, keep: bool, render_grids: bool) -> i32;
    }
}
