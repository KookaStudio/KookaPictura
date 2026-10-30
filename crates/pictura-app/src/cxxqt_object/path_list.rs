//! The Paths panel's document queries and edits: the saved paths and the Work
//! Path, which row is active (the path the Pen group and the path selection
//! tools edit, and the only one shown), New / Save / Rename / Duplicate /
//! Delete Path, and Fill Path, Stroke Path, and Load Path as a Selection. Free
//! functions over a [`PictureView`] (their own bridge).
//!
//! A row is named by index: `-1` the Work Path, `0..` a saved path, `-2` none.
//! Fill / Stroke / Load act on the path the path calls see (`paths::path`),
//! so the panel can point them at a shape layer's outline with
//! `path_set_layer_target`.
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
        /// Saved paths, in panel order; 0 without a document.
        fn paths_saved_count(view: &PictureView) -> i32;

        /// Saved path `i`'s name; empty when out of range.
        fn paths_saved_name(view: &PictureView, i: i32) -> QString;

        /// Whether the Work Path holds any subpath (CS6 lists it only then).
        fn paths_has_work_path(view: &PictureView) -> bool;

        /// The active row: -1 the Work Path, `0..` a saved path, -2 none.
        fn paths_active(view: &PictureView) -> i32;

        /// Make row `i` active (-2 hides every path). No history; emits `changed`. False when `i` names no row.
        fn paths_set_active(view: Pin<&mut PictureView>, i: i32) -> bool;

        /// Row `i`'s outline flattened for a thumbnail: per subpath its point count `n`, then `n` `x, y` pairs (a closed subpath ends on its first point). Empty when `i` names no row.
        fn paths_outline(view: &PictureView, i: i32) -> Vec<f64>;

        /// The first free default name, `"Path <n>"`.
        fn paths_next_name(view: &PictureView) -> QString;

        /// Append an empty saved path named `name` and make it active; records "New Path". Its index, or -1.
        fn paths_new(view: Pin<&mut PictureView>, name: &QString) -> i32;

        /// Save the Work Path as a saved path named `name`, which becomes active; records "Save Path". Its index, or -1 when there is no Work Path.
        fn paths_save_work(view: Pin<&mut PictureView>, name: &QString) -> i32;

        /// Rename saved path `i`; records "Rename Path". False for an empty or unchanged name.
        fn paths_rename(view: Pin<&mut PictureView>, i: i32, name: &QString) -> bool;

        /// Copy saved path `i` below itself as `"<name> copy"`, made active; records "Duplicate Path". Its index, or -1.
        fn paths_duplicate(view: Pin<&mut PictureView>, i: i32) -> i32;

        /// Delete row `i` (-1 the Work Path) and leave no row active; records "Delete Path".
        fn paths_delete(view: Pin<&mut PictureView>, i: i32) -> bool;

        /// Fill the targeted path's nonzero interior with `foreground` (`0xAARRGGBB`) on the active pixel layer, inside the selection; records "Fill Path". False when it encloses nothing or the layer cannot be painted.
        fn paths_fill(view: Pin<&mut PictureView>, foreground: u32) -> bool;

        /// Stroke every subpath of the targeted path with a round brush (`diameter` px, `hardness`, `opacity`, `flow` 0-100, `spacing` % of the diameter) in `foreground` on the active pixel layer; records "Stroke Path".
        fn paths_stroke(
            view: Pin<&mut PictureView>,
            foreground: u32,
            diameter: i32,
            hardness: i32,
            opacity: i32,
            flow: i32,
            spacing: i32,
        ) -> bool;

        /// Load the targeted path's nonzero interior as a selection, feathered by `feather` px and combined by `mode` (`"new"`, `"add"`, `"subtract"`, `"intersect"`); records "Make Selection". False when it encloses nothing.
        fn paths_make_selection(view: Pin<&mut PictureView>, feather: f64, mode: &QString) -> bool;
    }
}
