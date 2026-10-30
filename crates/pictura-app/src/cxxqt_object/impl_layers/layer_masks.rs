//! Layer > Layer Mask bridge: add (by kind), delete, apply, enable/disable, and
//! link, plus presence/link/disabled reads, over the active layer, and per-row
//! disabled/link reads. Free functions over a [`PictureView`] (their own bridge,
//! so the `PictureView` declaration list does not grow). Each edit recomposites
//! and records one state.
//!
//! [`PictureView`]: super::super::qobject::PictureView

mod imp;

use super::super::qobject::PictureView;
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
        /// Add a mask of `kind` (`"reveal-all"`, `"hide-all"`, `"reveal-selection"`,
        /// `"hide-selection"`, `"from-transparency"`) to the active layer; one "Add Layer Mask" state. False when the kind is unknown or the layer already has a mask.
        fn layer_mask_add(view: Pin<&mut PictureView>, kind: &QString) -> bool;

        /// Delete the active layer's mask; one "Delete Layer Mask" state. False without a mask.
        fn layer_mask_delete(view: Pin<&mut PictureView>) -> bool;

        /// Fold the active layer's mask into its alpha and clear it; one "Apply Layer Mask" state. False without a mask or on a smart object.
        fn layer_mask_apply(view: Pin<&mut PictureView>) -> bool;

        /// Fold the mask on the layer at `path` into its alpha and clear it;
        /// one "Apply Layer Mask" state. False without a mask or on a smart
        /// object. The mask-thumbnail menu acts on the clicked row, not the
        /// active layer.
        fn layer_mask_apply_path(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Set the active layer's mask enabled; one "Enable/Disable Layer Mask" state. False without a mask or no change.
        fn layer_mask_set_enabled(view: Pin<&mut PictureView>, enabled: bool) -> bool;

        /// Set the active layer's mask linked to the layer; one "Link/Unlink Layer Mask" state. False without a mask or no change.
        fn layer_mask_set_linked(view: Pin<&mut PictureView>, linked: bool) -> bool;

        /// Add a mask of `kind` to the layer at `path`; one "Add Layer Mask"
        /// state. The row-click contract edits the clicked row, not the active
        /// layer.
        fn layer_mask_add_path(view: Pin<&mut PictureView>, path: &QString, kind: &QString)
            -> bool;

        /// Delete the mask on the layer at `path`; one "Delete Layer Mask" state.
        fn layer_mask_delete_path(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Set the mask enabled on the layer at `path`; one undoable state.
        fn layer_mask_set_enabled_path(
            view: Pin<&mut PictureView>,
            path: &QString,
            enabled: bool,
        ) -> bool;

        /// Set the mask linked on the layer at `path`; one undoable state.
        fn layer_mask_set_linked_path(
            view: Pin<&mut PictureView>,
            path: &QString,
            linked: bool,
        ) -> bool;

        /// Whether the active layer has a raster layer mask.
        fn layer_mask_present(view: &PictureView) -> bool;

        /// Whether the layer at `path` has a raster layer mask.
        fn layer_mask_present_path(view: &PictureView, path: &QString) -> bool;

        /// Whether the layer at `path` has a raster layer mask that is disabled.
        fn layer_mask_disabled_path(view: &PictureView, path: &QString) -> bool;

        /// Make the raster mask on the layer at `path` the view's mask edit
        /// target, so the brush, fill, and filters write its coverage; an empty
        /// path clears the target. False without a mask on that layer. Records
        /// no history.
        fn mask_edit_target_set(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// The path of the layer whose raster mask is the edit target, empty
        /// when none. Clears a target whose layer, mask, or active-layer status
        /// no longer holds.
        fn mask_edit_target_get(view: Pin<&mut PictureView>) -> QString;

        /// Show the raster mask on the layer at `path` alone on the canvas, or
        /// restore the composite when `path` is empty. View-only: records no
        /// history and leaves the document unchanged. False when `path` has no
        /// raster mask, or when clearing an already-clear solo.
        fn mask_solo_set(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// The path whose raster mask is shown alone, empty when none. Clears a
        /// solo whose mask is gone.
        fn mask_solo_get(view: Pin<&mut PictureView>) -> QString;

        /// Whether the active layer's mask is linked to the layer.
        fn layer_mask_linked(view: &PictureView) -> bool;

    }
}
