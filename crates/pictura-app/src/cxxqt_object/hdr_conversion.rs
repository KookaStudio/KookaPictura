//! HDR Conversion (32 -> 16/8) bridge: the four-method conversion, one
//! `HDR Conversion` history state, and the selection clear. A free-function
//! bridge so the capped `cxxqt_object.rs` declaration list does not grow.
//!
//! [`PictureView`]: super::qobject::PictureView

use super::edit::Edit;
use super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_render::{ExposureGamma, HdrToneMap, LocalAdaptationParams};

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

#[allow(clippy::too_many_arguments)]
fn convert_depth_method(
    mut view: Pin<&mut PictureView>,
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
) -> bool {
    let out = match bits {
        16 => pictura_core::BitDepth::Sixteen,
        8 => pictura_core::BitDepth::Eight,
        _ => return false,
    };
    let tone = match method {
        0 => HdrToneMap::LocalAdaptation(LocalAdaptationParams {
            radius,
            strength,
            gamma: local_gamma,
            exposure: local_exposure,
            detail,
            shadow,
            highlight,
            vibrance,
            saturation,
        }),
        1 => HdrToneMap::EqualizeHistogram,
        2 => HdrToneMap::ExposureGamma(ExposureGamma { exposure_ev, gamma }),
        3 => HdrToneMap::HighlightCompression,
        _ => return false,
    };
    let converted = {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.session.doc.as_mut() else {
            return false;
        };
        pictura_render::convert_depth(doc, out, tone).is_ok()
    };
    if converted {
        {
            let mut rust = view.as_mut().rust_mut();
            rust.session.selection = None;
            if rust
                .session
                .doc
                .as_ref()
                .is_some_and(|doc| !doc.layers.is_empty())
            {
                rust.select_only_layer("0");
            }
        }
        view.as_mut().edit("HDR Conversion", |_| Edit::Full);
    }
    converted
}
