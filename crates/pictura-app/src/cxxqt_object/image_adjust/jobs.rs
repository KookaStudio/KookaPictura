//! Image > Adjustments dialog previews and applies off the GUI thread.
//!
//! A preview worker adjusts a copy-on-write document clone and returns the
//! adjusted layer plus the composited changed region. The GUI installs the layer
//! and the region when the latest request lands; the live document is untouched
//! until then. A newer request supersedes an older one. OK uses the shared
//! filter commit path with an adjustment op.

mod imp;

use super::super::qobject::PictureView;
pub(in crate::cxxqt_object) use imp::*;

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
        /// Start previewing `block` on the active pixel layer. False for a
        /// layer-mask target, no document, or an unknown adjustment.
        fn image_adjust_preview_start(view: Pin<&mut PictureView>, block: &[u8]) -> bool;

        /// Install the latest landed preview's layer and composited region.
        /// True when one landed.
        fn image_adjust_preview_poll(view: Pin<&mut PictureView>) -> bool;

        /// Whether the latest preview is still being computed.
        fn image_adjust_preview_busy(view: Pin<&mut PictureView>) -> bool;

        /// Drop the preview and restore the pre-dialog pixels.
        fn image_adjust_preview_cancel(view: Pin<&mut PictureView>);

        /// Hold a finished preview for `ms` milliseconds (test seam).
        fn image_adjust_preview_set_delay(view: Pin<&mut PictureView>, ms: u64);

        /// Start applying `block` as one history state named `label` on a
        /// worker. False when refused.
        fn image_adjust_apply_start(
            view: Pin<&mut PictureView>,
            block: &[u8],
            label: &QString,
        ) -> bool;

        /// Hold a finished apply for `ms` milliseconds (test seam).
        fn image_adjust_apply_set_delay(view: Pin<&mut PictureView>, ms: u64);
    }
}
