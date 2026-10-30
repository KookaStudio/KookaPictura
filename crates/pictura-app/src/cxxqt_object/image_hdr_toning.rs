//! Image > Adjustments > HDR Toning: the Local Adaptation neighborhood operator.
//!
//! A free-function bridge over [`PictureView`]: the nine Local Adaptation
//! controls preview on the canvas through the Filter menu's preview machinery
//! (re-applied from the pre-preview pixels, restricted to the visible section)
//! and apply as one "HDR Toning" state. The kernel lives in
//! `pictura_filters::hdr_toning`.
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
        /// Preview HDR Toning's Local Adaptation on the active layer,
        /// re-applied from the pre-preview pixels; no history. The whole layer
        /// is previewed regardless of `(x, y, w, h)`, because the operator's
        /// global pivot makes a cropped section differ from the commit. False
        /// when refused.
        #[allow(clippy::too_many_arguments)]
        fn hdr_toning_preview(
            view: Pin<&mut PictureView>,
            radius: f64,
            strength: f64,
            gamma: f64,
            exposure: f64,
            detail: f64,
            shadow: f64,
            highlight: f64,
            vibrance: f64,
            saturation: f64,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
        ) -> bool;

        /// Apply HDR Toning's Local Adaptation to the whole active layer as one
        /// `label` state (an open preview is replaced). False when refused.
        #[allow(clippy::too_many_arguments)]
        fn hdr_toning_apply(
            view: Pin<&mut PictureView>,
            radius: f64,
            strength: f64,
            gamma: f64,
            exposure: f64,
            detail: f64,
            shadow: f64,
            highlight: f64,
            vibrance: f64,
            saturation: f64,
            label: &QString,
        ) -> bool;
    }
}
