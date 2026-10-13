//! Owning settings model for the Pictura Raw smart filter.
//!
//! Pictura Raw controls live in two on-disk vocabularies: the `crs:` XMP
//! long-name property set (CS6 raw-as-Smart-Object) and the CC `Fltr` descriptor
//! short-key map. [`crate::CrsSettings`] remains the typed `crs:` view and is
//! reused here as the `basic` tab, so the same 11 PV2012 Basic controls are not
//! duplicated.
//!
//! Every control is optional, so an unset control is distinguishable from one
//! set to its default; the codec emits only set controls.

use crate::crs::CrsSettings;

/// The eight HSL / grayscale hue ranges, in on-disk key order.
pub const HSL_RANGES: [u8; 8] = *b"ROYGABPM";

/// Mode and process controls.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RawMode {
    /// `CMod` module mode, stored verbatim including the trailing NUL.
    pub module: Option<String>,
    /// `WBal` white-balance preset enum code.
    pub white_balance: Option<[u8; 4]>,
    /// `AWBV` auto white-balance version.
    pub auto_wb_version: Option<i32>,
    /// `Sett` settings enum code.
    pub settings_mode: Option<[u8; 4]>,
    /// `PrVe` process version.
    pub process_version: Option<i32>,
    /// `CtoG` convert to grayscale.
    pub grayscale: Option<bool>,
}

/// Tone-curve controls: four parametric regions, three split points, and four
/// point curves stored as flattened input/output pairs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ToneCurve {
    pub parametric_shadows: Option<i32>,
    pub parametric_darks: Option<i32>,
    pub parametric_lights: Option<i32>,
    pub parametric_highlights: Option<i32>,
    pub shadow_split: Option<i32>,
    pub midtone_split: Option<i32>,
    pub highlight_split: Option<i32>,
    pub curve_rgb: Vec<i32>,
    pub curve_red: Vec<i32>,
    pub curve_green: Vec<i32>,
    pub curve_blue: Vec<i32>,
}

/// Detail controls: sharpening plus luminance and color noise reduction.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Detail {
    pub sharpen_amount: Option<i32>,
    pub sharpen_radius: Option<f64>,
    pub sharpen_detail: Option<i32>,
    pub sharpen_masking: Option<i32>,
    pub noise_luminance: Option<i32>,
    pub noise_luminance_detail: Option<i32>,
    pub noise_luminance_contrast: Option<i32>,
    pub noise_color: Option<i32>,
    pub noise_color_detail: Option<i32>,
    pub noise_color_smoothness: Option<i32>,
}

/// HSL color ranges, the legacy red/green/blue sliders, and the grayscale mix.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Hsl {
    pub hue: [Option<i32>; 8],
    pub saturation: [Option<i32>; 8],
    pub luminance: [Option<i32>; 8],
    pub legacy_red_hue: Option<i32>,
    pub legacy_red_saturation: Option<i32>,
    pub legacy_green_hue: Option<i32>,
    pub legacy_green_saturation: Option<i32>,
    pub legacy_blue_hue: Option<i32>,
    pub legacy_blue_saturation: Option<i32>,
    pub grayscale_mix: [Option<i32>; 8],
}

/// Split toning.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SplitToning {
    pub shadow_hue: Option<i32>,
    pub shadow_saturation: Option<i32>,
    pub highlight_hue: Option<i32>,
    pub highlight_saturation: Option<i32>,
    pub balance: Option<i32>,
}

/// Lens-correction controls: profile/manual distortion, vignette, perspective,
/// and defringe.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Lens {
    pub profile_enabled: Option<i32>,
    pub distortion: Option<i32>,
    pub vignette_amount: Option<i32>,
    pub vignette_midpoint: Option<i32>,
    pub perspective_vertical: Option<i32>,
    pub perspective_horizontal: Option<i32>,
    pub perspective_scale: Option<i32>,
    pub perspective_aspect: Option<i32>,
    pub perspective_upright: Option<i32>,
    pub rotate: Option<f64>,
    pub offset_x: Option<f64>,
    pub offset_y: Option<f64>,
    pub defringe_purple_amount: Option<i32>,
    pub defringe_purple_hue_lo: Option<i32>,
    pub defringe_purple_hue_hi: Option<i32>,
    pub defringe_green_amount: Option<i32>,
    pub defringe_green_hue_lo: Option<i32>,
    pub defringe_green_hue_hi: Option<i32>,
}

/// Effects: film grain and post-crop vignette.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Effects {
    pub grain_amount: Option<i32>,
    pub grain_size: Option<i32>,
    pub grain_frequency: Option<i32>,
    pub vignette_amount: Option<i32>,
    pub vignette_midpoint: Option<i32>,
    pub vignette_feather: Option<i32>,
    pub vignette_roundness: Option<i32>,
    pub vignette_style: Option<i32>,
    pub vignette_highlights: Option<i32>,
}

/// Camera calibration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CameraCalibration {
    /// `CamP` camera profile name, stored verbatim including the trailing NUL.
    pub camera_profile: Option<String>,
    /// `CP_D` camera profile digest, stored verbatim.
    pub profile_digest: Option<String>,
}

/// The full Pictura Raw settings model: the CC `Fltr` key set in per-tab
/// sub-structs. `basic` reuses [`CrsSettings`] (the same 11 PV2012 controls).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PicturaRawSettings {
    pub basic: CrsSettings,
    /// `BlkB` legacy (PV2010) blacks.
    pub legacy_blacks: Option<i32>,
    pub mode: RawMode,
    pub tone_curve: ToneCurve,
    pub detail: Detail,
    pub hsl: Hsl,
    pub split_toning: SplitToning,
    pub lens: Lens,
    pub effects: Effects,
    pub calibration: CameraCalibration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_tab_value_is_cloneable_and_equatable() {
        let mut value = PicturaRawSettings::default();
        value.basic.exposure = Some(0.5);
        value.detail.sharpen_amount = Some(25);
        value.tone_curve.curve_rgb = vec![0, 0, 255, 255];
        value.mode.grayscale = Some(true);
        assert_eq!(value.clone(), value);
    }

    #[test]
    fn unset_control_differs_from_one_set_to_default() {
        let unset = PicturaRawSettings::default();
        let mut set_to_default = PicturaRawSettings::default();
        set_to_default.detail.sharpen_amount = Some(0);
        assert_ne!(unset, set_to_default);
    }
}
