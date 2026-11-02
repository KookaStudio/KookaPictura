//! Filter ▸ Liquify (#348): the dialog's editing session and its finished
//! meshes. The dialog edits a mesh over the active layer here and asks for
//! its view rendered at screen resolution; OK keeps the mesh and the
//! `liquify` kind's one slot names it by id, so the commit, Fade and Last
//! Filter run the ordinary filter path, as Displace's map does. Free
//! functions (their own bridge); the dialog is modal, so one session at a
//! time.

mod imp;

use super::super::qobject::PictureView;
pub(in crate::cxxqt_object) use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Open a session over the active layer. False when there is no
        /// unmasked pixel layer with colour data.
        fn liquify_begin(view: &PictureView) -> bool;

        /// The session's preview size, `[width, height]` — the whole layer, or
        /// the active selection's bounding rectangle inside it; empty with no
        /// session.
        fn liquify_geometry() -> Vec<i32>;

        /// The preview's top-left in the layer's own coordinates, `[x, y]`;
        /// empty with no session.
        fn liquify_origin() -> Vec<i32>;

        /// A `w`×`h` premultiplied view of the distorted layer whose pixel
        /// `(i, j)` shows layer point `(x + (i + 0.5) / zoom, y + (j + 0.5) /
        /// zoom)`, clear off the layer; null with no session.
        fn liquify_render(x: f64, y: f64, zoom: f64, w: i32, h: i32) -> QImage;

        /// Show Mesh: the source grid `spacing` layer pixels apart as the
        /// distortion draws it, traced where it can show inside the layer
        /// rectangle `(x0, y0)–(x1, y1)` and sampled every `step`. Flattened
        /// `x, y` pairs; a NaN pair ends each line.
        fn liquify_grid_lines(
            x0: f64,
            y0: f64,
            x1: f64,
            y1: f64,
            spacing: f64,
            step: f64,
        ) -> Vec<f64>;

        /// Show Backdrop's picture in the active layer's frame (its size, laid
        /// over it): an empty `path` is All Layers (the document with the
        /// active layer hidden), otherwise the layer at `path`. Null when
        /// unavailable.
        fn liquify_backdrop(view: Pin<&mut PictureView>, path: &QString) -> QImage;

        /// The brush the following strokes use. `tool` indexes Forward Warp,
        /// Reconstruct, Twirl, Pucker, Bloat, Push Left, Mirror, Turbulence,
        /// Freeze Mask, Thaw Mask.
        #[allow(clippy::too_many_arguments)]
        fn liquify_set_brush(
            tool: i32,
            size: f64,
            density: f64,
            pressure: f64,
            rate: f64,
            jitter: f64,
            reverse: bool,
        );

        /// Start a stroke at layer point `(x, y)` with stylus `pressure`
        /// (0..1). Each stroke call returns the layer rectangle it changed,
        /// `[x, y, w, h]`, or empty when nothing changed.
        fn liquify_stroke_begin(x: f64, y: f64, pressure: f64) -> Vec<i32>;

        /// Continue the stroke to `(x, y)`.
        fn liquify_stroke_to(x: f64, y: f64, pressure: f64) -> Vec<i32>;

        /// One Brush Rate tick with the brush held at the stroke's last point.
        fn liquify_hold() -> Vec<i32>;

        /// The Reconstruct button: restore unfrozen areas by `amount` percent.
        fn liquify_reconstruct(amount: f64);

        /// Restore All: drop every distortion, frozen areas included.
        fn liquify_restore_all();

        /// Mask None (0), Mask All (1), Invert All (2).
        fn liquify_mask(op: i32);

        /// Tint frozen areas with `(r, g, b)` in the view, or not.
        fn liquify_set_mask_overlay(show: bool, r: i32, g: i32, b: i32);

        /// Whether the mesh moves anything.
        fn liquify_distorted() -> bool;

        /// The id of the mesh the last OK kept (Load Last Mesh), or 0 when
        /// none is kept any more.
        fn liquify_last_mesh_id() -> i32;

        /// Replace the session's distortion with kept mesh `id`, stretched to
        /// fit this layer; the session's freeze mask stays. False when the id
        /// is not kept.
        fn liquify_load_mesh(id: i32) -> bool;

        /// End the session. With `keep` and a mesh that moves something, keep
        /// it (and remember it for Load Last Mesh) and return its id for the
        /// `liquify` slot; otherwise 0.
        fn liquify_finish(keep: bool) -> i32;
    }
}
