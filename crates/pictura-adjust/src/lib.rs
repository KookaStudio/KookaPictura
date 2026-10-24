//! Adjustment operations for Kooka Pictura.
//!
//! M4 scope: the destructive `Image > Adjustments` math as pure functions over a
//! planar 8-bit [`PixelBuffer`] (channels 3 or 4; alpha is never modified).
//! Specs live in `docs/04-image-ops/adjustments/`. Adjustment *layers* (model,
//! PSD serialization, compositor integration) are a later task.
//!
//! The closed kernels are approximated where the specs say so; each
//! approximation is marked inline. Everything is deterministic and returns
//! [`AdjustError`] instead of panicking on bad input.

mod apply;
mod auto;
mod color;
mod common;
mod hdr_toning;
mod lut;
mod match_color;
mod native;
mod pictura_raw;
mod replace_color;
mod tonal;
mod types;

#[cfg(test)]
mod tests;

pub use apply::{apply, apply_with_coverage};
pub use hdr_toning::{exposure_gamma, tone_map, ExposureGamma, HdrToneMap, LocalAdaptationParams};
pub use lut::parse_cube;
pub use match_color::{match_color_stats, MatchColorParams, MatchColorStats};
pub use native::apply_native;
pub use pictura_raw::render_pictura_raw;
pub use replace_color::{
    replace_color_mask, replace_color_result, replace_color_shift_for, ReplaceColorParams,
    ReplaceColorSample,
};
pub use types::{
    AdjustError, Adjustment, AutoKind, BlackWhiteParams, BrightnessContrastParams,
    ChannelMixerParams, ColorBalanceParams, ColorLookupKind, ColorLookupParams, CurvesParams,
    ExposureParams, GradientFillParams, GradientKind, GradientMapParams, GradientStop, HueRange,
    HueSaturationParams, LevelsChannel, LevelsParams, Lut3d, OpacityStop, PatternFillParams,
    PhotoFilterParams, SelectiveColorMethod, SelectiveColorParams, SelectiveRange,
    ShadowsHighlightsParams, VibranceParams,
};
