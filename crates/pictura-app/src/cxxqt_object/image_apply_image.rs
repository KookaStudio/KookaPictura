//! Image > Apply Image (#609) and Image > Calculations (#610). Both dialogs
//! read their sources and mask once into opaque snapshots, so a preview on the
//! active document cannot feed back into its own source; Apply Image previews
//! and commits through the shared preview session, while Calculations computes
//! a plane and hands it to the selection, a new channel, or a new document.
//! Free functions over a [`PictureView`] (their own bridge).
//!
//! [`PictureView`]: super::qobject::PictureView

mod calculations;
mod imp;

use super::qobject::PictureView;
use calculations::{calculations_apply, calculations_document, calculations_preview};
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
        /// Source or mask planes read once at document size.
        type ApplyImageSnapshot;

        /// Read `channel` of `layer` (empty is Merged): 0 composite, 1..=3 a
        /// colour plane, -1 Transparency, -2 the layer mask, -3 the selection,
        /// 100 + n alpha channel n. An unreadable choice gives an empty
        /// snapshot; any other negative value means "no mask".
        fn apply_image_read(
            source: &PictureView,
            layer: &QString,
            channel: i32,
        ) -> Box<ApplyImageSnapshot>;

        /// Whether the two documents have identical pixel dimensions.
        fn apply_image_same_size(a: &PictureView, b: &PictureView) -> bool;

        /// The active target's display name.
        fn apply_image_target_label(view: &PictureView) -> QString;

        /// Preview over the document rect `(x, y, w, h)`, from the pre-preview
        /// pixels; no history. `mode` is a PSD blend key, `add`, or `subtract`.
        /// False when refused.
        #[allow(clippy::too_many_arguments)]
        fn image_apply_image_preview(
            view: Pin<&mut PictureView>,
            source: &ApplyImageSnapshot,
            invert: bool,
            mode: &QString,
            scale: f64,
            offset: i32,
            opacity: i32,
            preserve_transparency: bool,
            mask: &ApplyImageSnapshot,
            mask_invert: bool,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
        ) -> bool;

        /// Apply to the whole target as one "Apply Image" state, replacing an
        /// open preview. False when refused.
        #[allow(clippy::too_many_arguments)]
        fn image_apply_image_apply(
            view: Pin<&mut PictureView>,
            source: &ApplyImageSnapshot,
            invert: bool,
            mode: &QString,
            scale: f64,
            offset: i32,
            opacity: i32,
            preserve_transparency: bool,
            mask: &ApplyImageSnapshot,
            mask_invert: bool,
        ) -> bool;

        /// Image > Calculations: the full-resolution grayscale blend of the two
        /// sources through the mask. Empty (and zero dimensions) when a plane
        /// cannot be read or the sizes differ. `mode` is a PSD blend key,
        /// `add`, or `subtract`.
        #[allow(clippy::too_many_arguments)]
        fn calculations_preview(
            source1: &ApplyImageSnapshot,
            invert1: bool,
            source2: &ApplyImageSnapshot,
            invert2: bool,
            mode: &QString,
            scale: f64,
            offset: i32,
            opacity: i32,
            mask: &ApplyImageSnapshot,
            mask_invert: bool,
            width: &mut i32,
            height: &mut i32,
        ) -> Vec<u8>;

        /// Commit the blend as the selection or a new alpha channel (`target`
        /// is `selection` or `channel`) under one "Calculations" state. False
        /// when refused, including the 56-channel cap.
        #[allow(clippy::too_many_arguments)]
        fn calculations_apply(
            view: Pin<&mut PictureView>,
            source1: &ApplyImageSnapshot,
            invert1: bool,
            source2: &ApplyImageSnapshot,
            invert2: bool,
            mode: &QString,
            scale: f64,
            offset: i32,
            opacity: i32,
            mask: &ApplyImageSnapshot,
            mask_invert: bool,
            target: &QString,
        ) -> bool;

        /// Fill a fresh view with the blend as an 8-bit grayscale document; the
        /// source documents are untouched. False when refused.
        #[allow(clippy::too_many_arguments)]
        fn calculations_document(
            view: Pin<&mut PictureView>,
            source1: &ApplyImageSnapshot,
            invert1: bool,
            source2: &ApplyImageSnapshot,
            invert2: bool,
            mode: &QString,
            scale: f64,
            offset: i32,
            opacity: i32,
            mask: &ApplyImageSnapshot,
            mask_invert: bool,
        ) -> bool;
    }
}
