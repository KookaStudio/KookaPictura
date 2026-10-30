//! Filter ▸ Adaptive Wide Angle (#346): the workspace's editing session and
//! its kept constraint sets. The dialog edits the correction and constraints
//! here and asks for its view rendered at screen resolution; OK keeps the
//! constraints and the `adaptive-wide-angle` kind's slots name them by id, so
//! the commit, Last Filter and Last Filter Settings run the ordinary filter
//! path, as Liquify's mesh does. Free functions (their own bridge); the
//! dialog is modal, so one session at a time. Points cross the bridge in
//! layer pixels; "display" points are output pixels with Preview on and
//! layer pixels with it off.

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
        /// Open a session over the active layer. False when the document is
        /// not one the filter takes or there is no unmasked pixel layer with
        /// colour data.
        fn awa_begin(view: &PictureView) -> bool;

        /// The session's layer size, `[width, height]`; empty with no session.
        fn awa_geometry() -> Vec<i32>;

        /// The lens EXIF gives, `[focal length, crop factor]` in the
        /// dialog's ranges; empty when the image has no focal length.
        fn awa_exif_lens() -> Vec<f64>;

        /// Whether the session's layer is the 2:1 panorama Full Spherical
        /// needs.
        fn awa_full_spherical_allowed() -> bool;

        /// The session's `[focal length, crop factor]`.
        fn awa_lens_settings() -> Vec<f64>;

        /// The Focal Length field's `[minimum, maximum]` at `crop`.
        fn awa_focal_range(crop: f64) -> Vec<f64>;

        /// EXIF Make and Model, or empty.
        fn awa_camera_model() -> QString;

        /// EXIF Lens Model, or empty.
        fn awa_lens_model() -> QString;

        /// Set Correction (0 Fisheye, 1 Perspective, 2 Full Spherical),
        /// Scale (%), Focal Length (mm) and Crop Factor, and re-solve.
        fn awa_set_lens(correction: i32, scale: f64, focal: f64, crop: f64);

        /// The Move tool's offset, in layer pixels.
        fn awa_set_offset(x: f64, y: f64);
        fn awa_offset() -> Vec<f64>;

        /// Add a line from layer point `(ax, ay)` to `(bx, by)`; `orientation`
        /// is 0 Unfixed, 1 Horizontal, 2 Vertical. Returns its index.
        fn awa_add_line(ax: f64, ay: f64, bx: f64, by: f64, orientation: i32) -> i32;

        /// Add a polygon through flattened layer points. Returns its index,
        /// or -1 with fewer than three corners.
        fn awa_add_polygon(points: &[f64]) -> i32;

        /// Set constraint `index`'s orientation: 0 Unfixed, 1 Horizontal,
        /// 2 Vertical, 3 Arbitrary (fixed at its current output angle).
        fn awa_set_orientation(index: i32, orientation: i32) -> bool;

        /// Move point `point` of constraint `index` to layer point `(x, y)`.
        fn awa_move_point(index: i32, point: i32, x: f64, y: f64) -> bool;

        fn awa_remove(index: i32) -> bool;
        fn awa_constraint_count() -> i32;

        /// 0 Unfixed, 1 Horizontal, 2 Vertical, 3 Fixed, 4 invalid; -1 when
        /// there is no such constraint.
        fn awa_constraint_state(index: i32) -> i32;

        fn awa_constraint_polygon(index: i32) -> bool;

        /// Constraint `index`'s traced edges in display pixels, flattened
        /// `x, y` pairs with a NaN pair after each edge.
        fn awa_constraint_paths(index: i32, preview: bool) -> Vec<f64>;

        /// Constraint `index`'s points (its handles) in display pixels.
        fn awa_constraint_points(index: i32, preview: bool) -> Vec<f64>;

        /// The traced great circle between two layer points, in display
        /// pixels; empty when it cannot be computed.
        fn awa_arc(ax: f64, ay: f64, bx: f64, by: f64, preview: bool) -> Vec<f64>;

        /// The layer point display point `(x, y)` shows; empty off the
        /// picture.
        fn awa_to_layer(x: f64, y: f64, preview: bool) -> Vec<f64>;

        /// A `w`×`h` premultiplied view whose pixel `(i, j)` shows display
        /// point `(x + (i + 0.5) / zoom, y + (j + 0.5) / zoom)`.
        fn awa_render(x: f64, y: f64, zoom: f64, w: i32, h: i32, preview: bool) -> QImage;

        /// Show Mesh: the solved grid's lines in display pixels, flattened
        /// with a NaN pair after each line.
        fn awa_mesh_lines(preview: bool) -> Vec<f64>;

        /// Restore committed slots (Last Filter Settings): the lens, offset
        /// and constraints. False when the constraint set is no longer kept
        /// or the slots ask for Full Spherical on a layer that is not 2:1.
        fn awa_load(values: &[f64]) -> bool;

        /// End the session. With `keep`, keep the constraints and return the
        /// kind's slots; otherwise empty.
        fn awa_finish(keep: bool) -> Vec<f64>;
    }
}
