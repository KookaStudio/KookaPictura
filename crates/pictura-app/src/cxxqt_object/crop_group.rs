//! The Crop tool group's Crop, Perspective Crop, and Slice commands. Free functions
//! over a [`PictureView`] (their own bridge, so the `PictureView` declaration
//! list does not grow). A Perspective Crop records one "Perspective Crop" state,
//! a new slice one "Slice" state, a committed slice move/resize one "Edit
//! Slice" state, and a deletion one "Delete Slice" state; every refusal records
//! nothing.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
use imp::*;

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
        /// Perspective-crop to `quad` (8 values: TL, TR, BR, BL x/y in document pixels): warp every layer so the quad becomes the canvas, drop the selection, recomposite, and record one "Perspective Crop" state. False, changing nothing, without a document, on a refusal, or for a degenerate quad.
        fn perspective_crop_commit(view: Pin<&mut PictureView>, quad: &[f64]) -> bool;

        /// As `perspective_crop_commit`, but a positive `out_w`/`out_h` forces that output dimension (0 = auto from the quad).
        fn perspective_crop_commit_sized(
            view: Pin<&mut PictureView>,
            quad: &[f64],
            out_w: i32,
            out_h: i32,
        ) -> bool;

        /// The Crop tool's commit: crop to the `width`×`height` rect at `(x, y)`, discarding the pixels outside it when `delete_cropped`; drop the selection and record one history state named `label` ("Crop" when empty). A rect that stays inside the canvas is clamped as before; a rect that extends beyond it grows the canvas, filling the added area with the Background layer's colour `fill_rgb` (0xRRGGBB; ignored without a Background layer). False, changing nothing, without a document or for an empty rect.
        fn crop_to(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            delete_cropped: bool,
            fill_rgb: i32,
            label: &QString,
        ) -> bool;

        /// The Crop tool's straighten commit: rotate the document about
        /// `(pivot_x, pivot_y)` by `angle` degrees, then crop to the
        /// `width`×`height` box at `(x, y)` (discarding pixels outside when
        /// `delete_cropped`, growing the canvas for an out-of-bounds box and
        /// filling it with `fill_rgb`); drop the selection and record one
        /// history state named `label` ("Crop" when empty). False, changing nothing, without a document, an invalid angle,
        /// or an empty rect.
        fn straighten_crop(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            angle: f64,
            pivot_x: f64,
            pivot_y: f64,
            delete_cropped: bool,
            fill_rgb: i32,
            label: &QString,
        ) -> bool;

        /// Why the document cannot be perspective-cropped (live type, smart objects, vector masks, 16/32-bit), or empty when it can.
        fn perspective_crop_refusal_reason(view: &PictureView) -> QString;

        /// The union of every layer's rect as "left top right bottom", or empty without a document. Used by the crop tools' snapping.
        fn composite_bounds(view: &PictureView) -> QString;

        /// Number of resolved slices (user and auto); 0 without a document.
        fn slice_count(view: &PictureView) -> i32;

        /// Resolved slice `index` as `[x, y, width, height, number, userIndex]` (`userIndex` -1 for an auto slice); empty when out of range.
        fn slice_at(view: &PictureView, index: i32) -> Vec<i32>;

        /// Add a user slice and record one "Slice" state; its index, or -1 (no state) for an empty rect or without a document.
        fn add_user_slice(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
        ) -> i32;

        /// Move or resize user slice `index`. With `commit` false this is a live drag step (no history, no `changed`); with `commit` true it records one "Edit Slice" state. False for an empty rect, a bad index, or no document.
        fn set_user_slice(
            view: Pin<&mut PictureView>,
            index: i32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            commit: bool,
        ) -> bool;

        /// Delete user slice `index` and record one "Delete Slice" state; false for a bad index or no document.
        fn remove_user_slice(view: Pin<&mut PictureView>, index: i32) -> bool;
    }
}
