//! The colour-model bridge: the picker and the Info panel read Lab and CMYK
//! from `pictura_color::model`, and the Channels panel thumbnails read the
//! engine's plate split, so the shell keeps no colour formula of its own.

use self::ffi::{CmykValue, LabValue};
use super::super::channels::mode_name;
use super::super::impl_selection::{channel_value, color_rows, q_gray};
use cxx_qt_lib::QString;
use pictura_color::model::{self, CmykConverter};
use pictura_core::ColorMode;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    /// CIELAB D50: `l` in 0–100, `a`/`b` unclamped.
    #[namespace = "pictura"]
    struct LabValue {
        l: f64,
        a: f64,
        b: f64,
    }

    /// CMYK ink percentages, 0–100 each.
    #[namespace = "pictura"]
    struct CmykValue {
        c: f64,
        m: f64,
        y: f64,
        k: f64,
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// 8-bit sRGB → CIELAB in the D50 connection space.
        fn srgb_to_lab(r: u8, g: u8, b: u8) -> LabValue;

        /// CIELAB D50 → 8-bit sRGB packed `0xRRGGBB`; out-of-gamut clamps.
        fn lab_to_srgb(l: f64, a: f64, b: f64) -> u32;

        /// 8-bit sRGB → CMYK through the CMYK working profile.
        fn srgb_to_cmyk(r: u8, g: u8, b: u8) -> CmykValue;

        /// The Channels panel row `row`'s grey value for each `0xAARRGGBB`
        /// pixel of a `mode` document (`mode` as `channels_mode` names it):
        /// the composite's luminance, an RGB plane, or a CMYK / Multichannel
        /// ink plate, the same split channel-to-selection loads.
        fn channel_plane(pixels: &[u32], mode: &QString, row: i32) -> Vec<u8>;
    }
}

fn srgb_to_lab(r: u8, g: u8, b: u8) -> LabValue {
    let [l, a, b] = model::srgb_to_lab([r, g, b]);
    LabValue { l, a, b }
}

fn lab_to_srgb(l: f64, a: f64, b: f64) -> u32 {
    let [r, g, b] = model::lab_to_srgb([l, a, b]);
    (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

fn srgb_to_cmyk(r: u8, g: u8, b: u8) -> CmykValue {
    let [c, m, y, k] = CmykConverter::working().srgb_to_cmyk([r, g, b]);
    CmykValue { c, m, y, k }
}

const MODES: [ColorMode; 8] = [
    ColorMode::Bitmap,
    ColorMode::Grayscale,
    ColorMode::Indexed,
    ColorMode::Rgb,
    ColorMode::Cmyk,
    ColorMode::Multichannel,
    ColorMode::Duotone,
    ColorMode::Lab,
];

fn channel_plane(pixels: &[u32], mode: &QString, row: i32) -> Vec<u8> {
    let name = mode.to_string();
    let mode = MODES
        .into_iter()
        .find(|m| mode_name(*m) == name)
        .unwrap_or(ColorMode::Rgb);
    let (composite, _) = color_rows(mode);
    let row = usize::try_from(row).unwrap_or(0);
    pixels
        .iter()
        .map(|&px| {
            let [_, r, g, b] = px.to_be_bytes();
            if composite && row == 0 {
                q_gray(r, g, b)
            } else {
                channel_value(mode, row - usize::from(composite), r, g, b)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pre-move C++ `channelValue` for an ink row, transcribed literally.
    fn naive_ink_row(r: u8, g: u8, b: u8, index: usize) -> u8 {
        let (rf, gf, bf) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
        let k = 1.0 - rf.max(gf).max(bf);
        let inv = if k < 1.0 { 1.0 / (1.0 - k) } else { 0.0 };
        let inks = [
            (1.0 - rf - k) * inv,
            (1.0 - gf - k) * inv,
            (1.0 - bf - k) * inv,
            k,
        ];
        255 - (inks[index] * 255.0).round() as u8
    }

    #[test]
    fn channel_plane_matches_selection_values() {
        let samples = [
            (255u8, 255u8, 255u8),
            (0, 0, 0),
            (200, 120, 80),
            (10, 200, 90),
        ];
        let pixels: Vec<u32> = samples
            .iter()
            .map(|&(r, g, b)| {
                0xff00_0000 | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
            })
            .collect();
        for row in 1..=4 {
            let plane = channel_plane(&pixels, &QString::from("cmyk"), row);
            for (i, &(r, g, b)) in samples.iter().enumerate() {
                let index = row as usize - 1;
                assert_eq!(plane[i], channel_value(ColorMode::Cmyk, index, r, g, b));
                assert_eq!(
                    plane[i],
                    naive_ink_row(r, g, b, index),
                    "cmyk row {row} pixel {i}"
                );
            }
        }
        for row in 0..3 {
            let plane = channel_plane(&pixels, &QString::from("multichannel"), row);
            for (i, &(r, g, b)) in samples.iter().enumerate() {
                assert_eq!(plane[i], naive_ink_row(r, g, b, row as usize));
            }
        }
        let composite = channel_plane(&pixels, &QString::from("cmyk"), 0);
        assert_eq!(composite[2], q_gray(200, 120, 80));
        let green = channel_plane(&pixels, &QString::from("rgb"), 2);
        assert_eq!(green, vec![255, 0, 120, 200]);
    }

    #[test]
    fn bridge_lab_and_cmyk_follow_the_model() {
        let white = srgb_to_lab(255, 255, 255);
        assert_eq!(white.l.round(), 100.0);
        assert_eq!(lab_to_srgb(100.0, 0.0, 0.0), 0x00ff_ffff);
        let inks = srgb_to_cmyk(255, 255, 255);
        assert!([inks.c, inks.m, inks.y, inks.k]
            .iter()
            .all(|v| v.round() == 0.0));
    }
}
