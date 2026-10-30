//! The colour-model bridge: the picker and the Info panel read Lab from
//! `pictura_color::model`, and the Channels panel thumbnails read the
//! engine's plate split, so the shell keeps no colour formula of its own.

mod imp;

use imp::*;

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

    #[namespace = "pictura"]
    extern "Rust" {
        /// 8-bit sRGB → CIELAB in the D50 connection space.
        fn srgb_to_lab(r: u8, g: u8, b: u8) -> LabValue;

        /// CIELAB D50 → 8-bit sRGB packed `0xRRGGBB`; out-of-gamut clamps.
        fn lab_to_srgb(l: f64, a: f64, b: f64) -> u32;

        /// The Channels panel row `row`'s grey value for each `0xAARRGGBB`
        /// pixel of a `mode` document (`mode` as `channels_mode` names it):
        /// the composite's luminance, an RGB plane, or a CMYK / Multichannel
        /// ink plate, the same split channel-to-selection loads.
        fn channel_plane(pixels: &[u32], mode: &QString, row: i32) -> Vec<u8>;
    }
}
