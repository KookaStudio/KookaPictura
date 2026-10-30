//! Select > Refine Edge and the mask menu's Refine Mask: the dialog's preview
//! mask and its OK, refining the selection or a layer's mask. Free functions
//! over a [`PictureView`] (their own bridge, so the `PictureView` declaration
//! list does not grow). Ports CS6 Refine Edge (SEL-003) onto
//! [`pictura_select::refine`].
//!
//! [`PictureView`]: super::qobject::PictureView

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
        /// Whether Refine Edge can run: a document with a non-empty selection.
        fn refine_edge_available(view: &PictureView) -> bool;

        /// The refined coverage mask (one byte a pixel, row-major) scaled to fit
        /// `max_side`, with its size in `width` / `height`. Empty when
        /// unavailable.
        #[allow(clippy::too_many_arguments)]
        fn refine_edge_preview(
            view: &PictureView,
            smart_radius: bool,
            radius: i32,
            smooth: i32,
            feather: i32,
            contrast: i32,
            shift_edge: i32,
            max_side: i32,
            width: &mut i32,
            height: &mut i32,
        ) -> Vec<u8>;

        /// Apply the settings once: refine the active selection and write the
        /// result to the chosen `output` target (0 Selection, 1 Layer Mask, 2
        /// New Layer, 3 New Layer with Mask). `decontaminate` / `amount` add
        /// colour decontamination for the new-layer targets. One undo state;
        /// false when unavailable or refused.
        #[allow(clippy::too_many_arguments)]
        fn refine_edge_apply(
            view: Pin<&mut PictureView>,
            smart_radius: bool,
            radius: i32,
            smooth: i32,
            feather: i32,
            contrast: i32,
            shift_edge: i32,
            decontaminate: bool,
            amount: i32,
            output: i32,
        ) -> bool;

        /// Whether Refine Mask can run on the layer at `path`: it has a layer
        /// mask.
        fn refine_mask_available(view: &PictureView, path: &QString) -> bool;

        /// As `refine_edge_preview`, refining the mask of the layer at `path`.
        #[allow(clippy::too_many_arguments)]
        fn refine_mask_preview(
            view: &PictureView,
            path: &QString,
            smart_radius: bool,
            radius: i32,
            smooth: i32,
            feather: i32,
            contrast: i32,
            shift_edge: i32,
            max_side: i32,
            width: &mut i32,
            height: &mut i32,
        ) -> Vec<u8>;

        /// As `refine_edge_apply`, refining the mask of the layer at `path` and
        /// writing a Layer Mask or new-layer output for that layer; one "Refine
        /// Mask" state.
        #[allow(clippy::too_many_arguments)]
        fn refine_mask_apply(
            view: Pin<&mut PictureView>,
            path: &QString,
            smart_radius: bool,
            radius: i32,
            smooth: i32,
            feather: i32,
            contrast: i32,
            shift_edge: i32,
            decontaminate: bool,
            amount: i32,
            output: i32,
        ) -> bool;
    }
}
