//! HDR Conversion (32 -> 16/8) bridge: the four-method conversion, one
//! `HDR Conversion` history state, and the selection clear. A free-function
//! bridge so the capped `cxxqt_object.rs` declaration list does not grow.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Convert a 32-bit document with the selected HDR Conversion method
        /// (0 Local Adaptation, 1 Equalize Histogram, 2 Exposure and Gamma,
        /// 3 Highlight Compression), recomposite, and record one
        /// `HDR Conversion` state. False without a document or when refused.
        #[allow(clippy::too_many_arguments)]
        fn convert_depth_method(
            view: Pin<&mut PictureView>,
            bits: i32,
            method: i32,
            exposure_ev: f64,
            gamma: f64,
            radius: f64,
            strength: f64,
            local_gamma: f64,
            local_exposure: f64,
            detail: f64,
            shadow: f64,
            highlight: f64,
            vibrance: f64,
            saturation: f64,
        ) -> bool;
    }
}
