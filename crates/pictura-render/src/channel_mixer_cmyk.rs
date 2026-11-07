//! RGB <-> unit-CMYK conversion for a CMYK Channel Mixer.
//!
//! The working canvas is always RGB, so a CMYK ink matrix is applied by
//! converting the pixels to unit ink (`0` no ink, `1` full ink), mixing, and
//! converting back. An 8-bit document with an embedded four-channel profile
//! uses the profile transform; every other document uses the exact f32 GCR
//! round trip, so a 16/32-bit document is not quantized to 8 bits.

use pictura_adjust::{apply_channel_mixer_ink, ChannelMixerParams};
use pictura_codec::{
    cmyk_to_rgb_with_profile, document_cmyk_profile, rgb_to_cmyk_with_profile, ConversionOptions,
    Profile,
};
use pictura_core::{ColorMode, Document, PixelBuffer};

use crate::composite::Canvas;
use crate::document_color_mode;

/// The document state a CMYK Channel Mixer needs.
pub struct CmykMix {
    profile: Option<Profile>,
    eight_bit: bool,
}

impl CmykMix {
    /// `Some` only for a CMYK document.
    ///
    /// ponytail: the embedded profile is parsed per adjustment; cache it on the
    /// document if a profile-heavy document shows up in a composite profile.
    pub fn for_document(doc: &Document) -> Option<Self> {
        (document_color_mode(doc) == ColorMode::Cmyk).then(|| Self {
            profile: document_cmyk_profile(doc),
            eight_bit: !doc.retains_source_depth(),
        })
    }
}

/// Apply `params`' ink matrix to `canvas`. Returns false (the canvas unchanged)
/// for a monochrome mixer or invalid parameters, so the caller can use the RGB
/// path.
pub(crate) fn apply_canvas(
    canvas: &mut Canvas,
    params: &ChannelMixerParams,
    mix: &CmykMix,
) -> bool {
    if params.monochrome {
        return false;
    }
    let n = canvas.w * canvas.h;
    if n == 0 {
        return true;
    }
    if mix.eight_bit {
        if let Some(profile) = mix.profile.as_ref() {
            return apply_canvas_profile(canvas, params, profile);
        }
    }
    apply_canvas_gcr(canvas, params)
}

/// Apply `params`' ink matrix to a planar RGB buffer's first three planes. The
/// destructive path's buffer may carry a fourth (alpha) plane, which is left
/// untouched.
pub(crate) fn apply_planes(
    buf: &mut PixelBuffer,
    params: &ChannelMixerParams,
    mix: &CmykMix,
) -> bool {
    if params.monochrome || buf.channels < 3 {
        return false;
    }
    let n = buf.width as usize * buf.height as usize;
    if n == 0 {
        return true;
    }
    if mix.eight_bit {
        if let Some(profile) = mix.profile.as_ref() {
            return apply_planes_profile(buf, params, profile);
        }
    }
    apply_planes_gcr(buf, params)
}

fn to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn unit(value: u8) -> f32 {
    f32::from(value) / 255.0
}

/// The exact profile-free round trip: `k` is the darkest component, the other
/// inks carry the remainder.
fn rgb_to_unit(r: f32, g: f32, b: f32) -> [f32; 4] {
    let k = 1.0 - r.max(g).max(b);
    if k >= 1.0 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let inv = 1.0 - k;
    [
        (1.0 - r - k) / inv,
        (1.0 - g - k) / inv,
        (1.0 - b - k) / inv,
        k,
    ]
}

fn unit_to_rgb(c: f32, m: f32, y: f32, k: f32) -> [f32; 3] {
    [
        (1.0 - c) * (1.0 - k),
        (1.0 - m) * (1.0 - k),
        (1.0 - y) * (1.0 - k),
    ]
}

fn apply_canvas_gcr(canvas: &mut Canvas, params: &ChannelMixerParams) -> bool {
    let n = canvas.w * canvas.h;
    let mut ink = vec![0.0f32; n * 4];
    for (i, px) in canvas.px.iter().enumerate() {
        let [c, m, y, k] = rgb_to_unit(px.r, px.g, px.b);
        ink[i] = c;
        ink[n + i] = m;
        ink[2 * n + i] = y;
        ink[3 * n + i] = k;
    }
    if apply_channel_mixer_ink(params, &mut ink, n).is_err() {
        return false;
    }
    for (i, px) in canvas.px.iter_mut().enumerate() {
        let [r, g, b] = unit_to_rgb(ink[i], ink[n + i], ink[2 * n + i], ink[3 * n + i]);
        px.r = r;
        px.g = g;
        px.b = b;
    }
    true
}

fn apply_canvas_profile(
    canvas: &mut Canvas,
    params: &ChannelMixerParams,
    profile: &Profile,
) -> bool {
    let n = canvas.w * canvas.h;
    let mut rgb = vec![0u8; n * 3];
    for (i, px) in canvas.px.iter().enumerate() {
        rgb[i] = to_u8(px.r);
        rgb[n + i] = to_u8(px.g);
        rgb[2 * n + i] = to_u8(px.b);
    }
    let Some(cmyk) = rgb_to_cmyk_with_profile(&rgb, profile, ConversionOptions::default()) else {
        return false;
    };
    let mut ink: Vec<f32> = cmyk.iter().map(|&v| unit(v)).collect();
    if apply_channel_mixer_ink(params, &mut ink, n).is_err() {
        return false;
    }
    let cmyk: Vec<u8> = ink.iter().map(|&v| to_u8(v)).collect();
    let Some(rgb) = cmyk_to_rgb_with_profile(&cmyk, profile, ConversionOptions::default()) else {
        return false;
    };
    for (i, px) in canvas.px.iter_mut().enumerate() {
        px.r = unit(rgb[i]);
        px.g = unit(rgb[n + i]);
        px.b = unit(rgb[2 * n + i]);
    }
    true
}

fn apply_planes_gcr(buf: &mut PixelBuffer, params: &ChannelMixerParams) -> bool {
    let n = buf.width as usize * buf.height as usize;
    let data = buf.data.as_mut_slice();
    let mut ink = vec![0.0f32; n * 4];
    for i in 0..n {
        let [c, m, y, k] = rgb_to_unit(unit(data[i]), unit(data[n + i]), unit(data[2 * n + i]));
        ink[i] = c;
        ink[n + i] = m;
        ink[2 * n + i] = y;
        ink[3 * n + i] = k;
    }
    if apply_channel_mixer_ink(params, &mut ink, n).is_err() {
        return false;
    }
    for i in 0..n {
        let [r, g, b] = unit_to_rgb(ink[i], ink[n + i], ink[2 * n + i], ink[3 * n + i]);
        data[i] = to_u8(r);
        data[n + i] = to_u8(g);
        data[2 * n + i] = to_u8(b);
    }
    true
}

fn apply_planes_profile(
    buf: &mut PixelBuffer,
    params: &ChannelMixerParams,
    profile: &Profile,
) -> bool {
    let n = buf.width as usize * buf.height as usize;
    let data = buf.data.as_mut_slice();
    let rgb: Vec<u8> = (0..n * 3).map(|i| data[i]).collect();
    let Some(cmyk) = rgb_to_cmyk_with_profile(&rgb, profile, ConversionOptions::default()) else {
        return false;
    };
    let mut ink: Vec<f32> = cmyk.iter().map(|&v| unit(v)).collect();
    if apply_channel_mixer_ink(params, &mut ink, n).is_err() {
        return false;
    }
    let cmyk: Vec<u8> = ink.iter().map(|&v| to_u8(v)).collect();
    let Some(rgb) = cmyk_to_rgb_with_profile(&cmyk, profile, ConversionOptions::default()) else {
        return false;
    };
    data[..n * 3].copy_from_slice(&rgb);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcr_round_trips_rgb_exactly() {
        for rgb in [
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.2, 0.6, 0.9],
            [1.0, 0.0, 0.5],
        ] {
            let [c, m, y, k] = rgb_to_unit(rgb[0], rgb[1], rgb[2]);
            let back = unit_to_rgb(c, m, y, k);
            for (a, b) in rgb.iter().zip(back) {
                assert!((a - b).abs() < 1e-6, "{rgb:?} -> {back:?}");
            }
        }
    }

    #[test]
    fn a_black_pixel_has_full_black_ink() {
        assert_eq!(rgb_to_unit(0.0, 0.0, 0.0), [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn native_channel_mixer_cmyk_keeps_precision() {
        let mut canvas = Canvas::new(1, 1);
        canvas.px[0] = crate::composite::Px {
            r: 100.0 / 65535.0,
            g: 20000.0 / 65535.0,
            b: 40000.0 / 65535.0,
            a: 1.0,
        };
        let mix = CmykMix {
            profile: None,
            eight_bit: false,
        };
        let params = ChannelMixerParams {
            ink: [
                [0.0, 100.0, 0.0, 0.0],
                [0.0, 100.0, 0.0, 0.0],
                [0.0, 0.0, 100.0, 0.0],
                [0.0, 0.0, 0.0, 100.0],
            ],
            ..Default::default()
        };
        assert!(apply_canvas(&mut canvas, &params, &mix));
        let r = canvas.px[0].r;
        assert!(
            (r * 255.0).fract() > 1e-4,
            "a native value must not snap to the 8-bit grid"
        );
    }
}
