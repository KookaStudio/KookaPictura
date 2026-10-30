//! The Eyedropper group's Color Sampler, Note, and Ruler commands. Free
//! functions over a [`PictureView`] (their own bridge, so the `PictureView`
//! declaration list does not grow). `kind` is a [`MarkerKind`] integer: 0 color
//! sampler, 1 note. Every committed sampler/note edit records one history state
//! and emits `changed`; a live drag step records nothing. The ruler is view
//! state: it is neither recorded nor saved.
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
        /// Number of markers of `kind`; 0 without a document or for an unknown kind.
        fn marker_count(view: &PictureView, kind: i32) -> i32;

        /// Marker `index` of `kind` as `[x, y]` in document pixels; empty when out of range.
        fn marker_at(view: &PictureView, kind: i32, index: i32) -> Vec<i32>;

        /// The marker of `kind` nearest `(x, y)` within `radius` document pixels, or -1.
        fn marker_near(view: &PictureView, kind: i32, x: f64, y: f64, radius: f64) -> i32;

        /// Place a marker and record one "Color Sampler" / "New Note" state; its index, or -1 (no state) when the four samplers are placed or without a document.
        fn add_marker(view: Pin<&mut PictureView>, kind: i32, x: i32, y: i32) -> i32;

        /// Move marker `index`. With `commit` false this is a live drag step (no history, no `changed`); with `commit` true it records one "Move Color Sampler" / "Move Note" state. False for a bad index or no document.
        fn move_marker(
            view: Pin<&mut PictureView>,
            kind: i32,
            index: i32,
            x: i32,
            y: i32,
            commit: bool,
        ) -> bool;

        /// Delete marker `index` and record one "Delete Color Sampler" / "Delete Note" state; false for a bad index or no document.
        fn remove_marker(view: Pin<&mut PictureView>, kind: i32, index: i32) -> bool;

        /// Delete every marker of `kind` and record one "Clear Color Samplers" / "Delete All Notes" state; false (no state) when there were none.
        fn clear_markers(view: Pin<&mut PictureView>, kind: i32) -> bool;

        /// Note `index`'s text; empty when out of range.
        fn note_text(view: &PictureView, index: i32) -> QString;

        /// Replace note `index`'s text and record one "Edit Note" state; false (no state) for a bad index or unchanged text.
        fn set_note_text(view: Pin<&mut PictureView>, index: i32, text: &QString) -> bool;

        /// Set the Ruler's line from `(ax, ay)` to `(bx, by)` in document pixels, snapped to 45° when `constrain`. No history.
        fn set_ruler(
            view: Pin<&mut PictureView>,
            ax: f64,
            ay: f64,
            bx: f64,
            by: f64,
            constrain: bool,
        );

        /// Remove the Ruler's line.
        fn clear_ruler(view: Pin<&mut PictureView>);

        /// The Ruler's line as `[ax, ay, bx, by]`; empty when there is none.
        fn ruler_line(view: &PictureView) -> Vec<f64>;

        /// The Ruler readout `[X, Y, W, H, A (degrees), D1]`; empty when there is no line.
        fn ruler_measurement(view: &PictureView) -> Vec<f64>;

        /// The document's `[memory, disk]` byte footprint; empty without a document.
        fn document_size_bytes(view: &PictureView) -> Vec<f64>;

        /// The number of Count groups (always at least one).
        fn count_group_count(view: &PictureView) -> i32;

        /// The active Count group's index.
        fn count_active_group(view: &PictureView) -> i32;

        /// Select Count group `index`; false when out of range. No history.
        fn count_set_active_group(view: Pin<&mut PictureView>, index: i32) -> bool;

        /// Group `index`'s name; empty when out of range.
        fn count_group_name(view: &PictureView, index: i32) -> QString;

        /// Group `index`'s eye visibility.
        fn count_group_visible(view: &PictureView, index: i32) -> bool;

        /// Group `index`'s colour as 0xRRGGBB.
        fn count_group_color(view: &PictureView, index: i32) -> u32;

        /// Group `index`'s marker size (1–10).
        fn count_group_marker_size(view: &PictureView, index: i32) -> i32;

        /// Group `index`'s label size (8–72).
        fn count_group_label_size(view: &PictureView, index: i32) -> i32;

        /// Group `index`'s mark count.
        fn count_group_total(view: &PictureView, index: i32) -> i32;

        /// The active group's mark count (the options bar's Count field).
        fn count_active_total(view: &PictureView) -> i32;

        /// Group `index` mark `mark` as `[x, y]`; empty when out of range.
        fn count_group_mark_at(view: &PictureView, index: i32, mark: i32) -> Vec<i32>;

        /// Add a group named `name` and make it active; records one "Add Count Group" state. Returns its index.
        fn count_add_group(view: Pin<&mut PictureView>, name: &QString) -> i32;

        /// Delete group `index`; false (no state) for the only group or out of range. Records "Delete Count Group".
        fn count_remove_group(view: Pin<&mut PictureView>, index: i32) -> bool;

        /// Set group `index`'s eye visibility; false when out of range. No history.
        fn count_set_visible(view: Pin<&mut PictureView>, index: i32, on: bool) -> bool;

        /// Set group `index`'s colour (0xRRGGBB); false when out of range. No history.
        fn count_set_color(view: Pin<&mut PictureView>, index: i32, rgb: u32) -> bool;

        /// Set group `index`'s marker size (clamped 1–10); false when out of range. No history.
        fn count_set_marker_size(view: Pin<&mut PictureView>, index: i32, size: i32) -> bool;

        /// Set group `index`'s label size (clamped 8–72); false when out of range. No history.
        fn count_set_label_size(view: Pin<&mut PictureView>, index: i32, size: i32) -> bool;

        /// Add a mark to the active group at `(x, y)`; records one "New Count" state. Returns its index.
        fn count_add_mark(view: Pin<&mut PictureView>, x: i32, y: i32) -> i32;

        /// The active group's mark nearest `(x, y)` within `radius`, or -1.
        fn count_mark_near(view: &PictureView, x: f64, y: f64, radius: f64) -> i32;

        /// Move active-group mark `index`. With `commit` false this is a live drag step; with `commit` true it records one "Move Count" state. False for a bad index.
        fn count_move_mark(
            view: Pin<&mut PictureView>,
            index: i32,
            x: i32,
            y: i32,
            commit: bool,
        ) -> bool;

        /// Delete active-group mark `index`; records one "Delete Count" state. False for a bad index.
        fn count_remove_mark(view: Pin<&mut PictureView>, index: i32) -> bool;

        /// Clear the active group's marks; records one "Clear Counts" state. False (no state) when there were none.
        fn count_clear(view: Pin<&mut PictureView>) -> bool;
    }
}
