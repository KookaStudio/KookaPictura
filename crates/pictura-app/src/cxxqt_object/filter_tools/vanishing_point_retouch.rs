//! Filter ▸ Vanishing Point's retouch tools (#349): Paste, the Marquee and
//! Transform, Stamp, Brush and Eyedropper over the session in
//! `vanishing_point.rs`, and the preview the dialog draws. Coordinates are
//! document pixels. Free functions (their own bridge).

mod imp;

use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The preview of document rect `(x, y, w, h)`: the document without
        /// the layer, the edited layer over it, and any floating selection.
        fn vp_view(x: i32, y: i32, w: i32, h: i32) -> QImage;

        /// The document rect `[x, y, w, h]` the preview must redraw since the
        /// last call; empty when nothing changed.
        fn vp_view_dirty() -> Vec<i32>;

        /// Paste the clipboard as a floating selection in the active plane:
        /// 0 pasted, 1 no valid plane, 2 nothing on the clipboard.
        fn vp_paste_clipboard() -> i32;

        fn vp_has_float() -> bool;
        fn vp_has_marquee() -> bool;
        /// The floating selection's and the marquee's outlines, `x, y` pairs.
        fn vp_float_outline() -> Vec<f64>;
        fn vp_marquee_outline() -> Vec<f64>;
        /// The floating selection's four corners (Transform's handles).
        fn vp_float_corners() -> Vec<f64>;

        /// Marquee press at `(x, y)` with Alt / Ctrl held; false when it does
        /// nothing (outside every plane).
        fn vp_marquee_press(x: f64, y: f64, alt: bool, ctrl: bool) -> bool;
        /// Marquee drag; `square` (Shift) keeps a new marquee square.
        fn vp_marquee_drag(x: f64, y: f64, square: bool);
        /// End a Marquee, Transform, Stamp or Brush gesture: one undo step.
        fn vp_gesture_release();
        /// Double-click with the Marquee: select the plane under `(x, y)`.
        fn vp_select_plane(x: f64, y: f64) -> bool;

        /// Transform press: a corner within `tolerance` px scales, just past
        /// one rotates, inside moves. False elsewhere or without a float.
        fn vp_transform_press(x: f64, y: f64, tolerance: f64) -> bool;
        /// Transform drag; `uniform` (Shift) keeps the aspect.
        fn vp_transform_drag(x: f64, y: f64, uniform: bool);
        /// Flip (`horizontal`) or Flop the floating selection.
        fn vp_mirror(horizontal: bool) -> bool;

        /// Put the floating selection down into the layer.
        fn vp_drop_float() -> bool;
        /// Drop the floating selection and clear the marquee (Ctrl+D).
        fn vp_deselect();

        /// Alt-click with the Stamp: clone from `(x, y)`.
        fn vp_stamp_source(x: f64, y: f64) -> bool;
        fn vp_has_stamp_source() -> bool;
        /// Start a Brush (`brush`) or Stamp stroke; false outside every plane
        /// or for a Stamp with no source.
        fn vp_stroke_press(brush: bool, x: f64, y: f64) -> bool;
        fn vp_stroke_drag(x: f64, y: f64);

        /// The colour the preview shows at `(x, y)`, `[r, g, b]`.
        fn vp_sample(x: f64, y: f64) -> Vec<i32>;

        /// Marquee options: Feather px, Opacity %, Heal (0 Off, 1 Luminance,
        /// 2 On), Move Mode Source.
        fn vp_set_marquee_options(feather: f64, opacity: f64, heal: i32, source: bool);
        /// Stamp (`brush` false) or Brush options: Diameter px, Hardness %,
        /// Opacity %, Heal.
        fn vp_set_paint_options(brush: bool, diameter: f64, hardness: f64, opacity: f64, heal: i32);
        fn vp_set_aligned(aligned: bool);
        fn vp_set_brush_colour(r: i32, g: i32, b: i32);
    }
}
