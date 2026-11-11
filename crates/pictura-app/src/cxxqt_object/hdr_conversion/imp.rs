use super::*;

use super::super::impl_core::{JobBar, Outcome};
use super::super::impl_transform::land_hdr_conversion;
use core::pin::Pin;
use pictura_render::{ExposureGamma, HdrToneMap, LocalAdaptationParams};

#[allow(clippy::too_many_arguments)]
pub(crate) fn convert_depth_method(
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
    view.as_mut().document_op(
        "HDR Conversion",
        JobBar::Busy,
        move |doc| {
            pictura_render::convert_depth(doc, out, tone)
                .ok()
                .map(|()| Outcome::default())
        },
        land_hdr_conversion,
    )
}
