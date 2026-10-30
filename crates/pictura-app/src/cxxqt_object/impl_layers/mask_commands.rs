//! The rubylith overlay of the Layers panel's mask thumbnails and its Layer
//! Mask Display Options. The overlay and its options are view state: they are
//! drawn over the canvas and never reach the composite.
//!
//! [`PictureView`]: super::super::qobject::PictureView

mod imp;

use super::super::qobject::PictureView;
pub(in crate::cxxqt_object) use imp::*;

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
        /// Toggle the rubylith overlay for the mask of the layer at `path`:
        /// show it, or hide it when it is already shown for that layer. Returns
        /// whether it is now shown. False without a mask.
        fn mask_overlay_toggle(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// The layer whose mask the overlay shows, empty when it is off.
        fn mask_overlay_path(view: &PictureView) -> QString;

        /// Changes whenever the overlay or its display options change.
        fn mask_overlay_revision(view: &PictureView) -> u64;

        /// The rubylith colour (ARGB) of the Layer Mask Display Options.
        fn mask_display_color(view: &PictureView) -> u32;

        /// The rubylith opacity (0–100) of the Layer Mask Display Options.
        fn mask_display_opacity(view: &PictureView) -> i32;

        /// Set the Layer Mask Display Options; display only, no history.
        fn set_mask_display_options(view: Pin<&mut PictureView>, argb: u32, opacity: i32);

        /// The overlay over the whole document, at most `max_side` px on its long
        /// side: the rubylith colour at `opacity × (255 − coverage) / 255`. A
        /// null image while the overlay is off or its layer has no mask.
        fn mask_overlay_image(view: &PictureView, max_side: i32) -> QImage;
    }
}
