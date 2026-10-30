//! Layer > Vector Mask bridge: add (by kind), delete, enable/disable,
//! link/unlink, and rasterize, plus presence/link/disabled reads, over the
//! active layer. Free functions over a [`PictureView`] (their own bridge, so
//! the `PictureView` declaration list does not grow). Each edit recomposites
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
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Add a vector mask of `kind` (`"reveal-all"`, `"hide-all"`,
        /// `"current-path"`) to the active layer; one "Add Vector Mask" state.
        /// False when the kind is unknown, the layer already has a vector mask,
        /// or Current Path has no work path.
        fn vector_mask_add(view: Pin<&mut PictureView>, kind: &QString) -> bool;

        /// Add a vector mask of `kind` to the layer at `path`; one
        /// "Add Vector Mask" state. The row menu edits the clicked row.
        fn vector_mask_add_path(
            view: Pin<&mut PictureView>,
            path: &QString,
            kind: &QString,
        ) -> bool;

        /// Delete the active layer's vector mask; one "Delete Vector Mask"
        /// state. False without a vector mask.
        fn vector_mask_delete(view: Pin<&mut PictureView>) -> bool;

        /// Set the active layer's vector mask enabled; one
        /// "Enable/Disable Vector Mask" state. False without a mask or no change.
        fn vector_mask_set_enabled(view: Pin<&mut PictureView>, enabled: bool) -> bool;

        /// Set the active layer's vector mask linked; one "Link/Unlink Vector
        /// Mask" state. False without a mask or no change.
        fn vector_mask_set_linked(view: Pin<&mut PictureView>, linked: bool) -> bool;

        /// Set the vector mask enabled on the layer at `path`; one undoable
        /// state. The row-click contract edits the clicked row, not the active
        /// layer.
        fn vector_mask_set_enabled_path(
            view: Pin<&mut PictureView>,
            path: &QString,
            enabled: bool,
        ) -> bool;

        /// Set the vector mask linked on the layer at `path`; one undoable state.
        fn vector_mask_set_linked_path(
            view: Pin<&mut PictureView>,
            path: &QString,
            linked: bool,
        ) -> bool;

        /// Convert the active layer's vector mask to a layer mask; one
        /// "Rasterize Vector Mask" state. False without a vector mask.
        fn vector_mask_rasterize(view: Pin<&mut PictureView>) -> bool;

        /// Delete the vector mask on the layer at `path`; one "Delete Vector
        /// Mask" state. The thumbnail menu edits the clicked row.
        fn vector_mask_delete_path(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Convert the vector mask on the layer at `path` to a layer mask; one
        /// "Rasterize Vector Mask" state.
        fn vector_mask_rasterize_path(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Whether the active layer has a vector mask.
        fn vector_mask_present(view: &PictureView) -> bool;

        /// Whether the layer at `path` has a vector mask.
        fn vector_mask_present_path(view: &PictureView, path: &QString) -> bool;

        /// Whether the layer at `path` has a vector mask that is disabled.
        fn vector_mask_disabled_path(view: &PictureView, path: &QString) -> bool;

        /// Whether the active layer's vector mask is linked to the layer.
        fn vector_mask_linked(view: &PictureView) -> bool;

        /// Whether the active layer's vector mask is disabled.
        fn vector_mask_disabled(view: &PictureView) -> bool;

    }
}
