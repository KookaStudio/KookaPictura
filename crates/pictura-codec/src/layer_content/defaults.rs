//! Per-kind builders for the block an Image > Adjustments dialog opens on and
//! the block a new adjustment layer carries. The catalogue rows point at these
//! hooks directly.

use pictura_core::{AdjustmentData, ShadowsHighlightsOptions, ShadowsHighlightsParams};
use pictura_core::{GradientMapParams, GradientStop, SelectiveColorMethod, SelectiveRange};

use crate::DescValue;

use super::{
    encode_brightness_contrast, encode_channel_mixer, encode_channel_mixer_ink,
    encode_color_balance, encode_color_lookup, encode_curves, encode_gradient_map,
    encode_hue_saturation, encode_invert, encode_photo_filter, encode_posterize,
    encode_selective_color, encode_threshold, identity_cube,
};

/// The document context a default depends on: the foreground/background pair a
/// Gradient Map runs between, and whether the document is CMYK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Defaults {
    pub foreground: [u8; 3],
    pub background: [u8; 3],
    pub cmyk: bool,
}

impl Defaults {
    /// Black to white in an RGB document: the pair every other default assumes.
    pub const fn rgb() -> Self {
        Self {
            foreground: [0, 0, 0],
            background: [255, 255, 255],
            cmyk: false,
        }
    }

    pub const fn with_colors(mut self, foreground: [u8; 3], background: [u8; 3]) -> Self {
        self.foreground = foreground;
        self.background = background;
        self
    }

    pub const fn with_cmyk(mut self, cmyk: bool) -> Self {
        self.cmyk = cmyk;
        self
    }
}

fn descriptor(key: [u8; 4], items: Vec<(&[u8], DescValue)>) -> AdjustmentData {
    let obj = DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: items.into_iter().map(|(k, v)| (k.to_vec(), v)).collect(),
    };
    AdjustmentData {
        key,
        data: crate::write_descriptor(&obj).into(),
    }
}

/// `shdH`: two big-endian `u16` amounts, Shadows then Highlights, each
/// `0..=100`.
pub fn encode_shadows_highlights(shadows: u16, highlights: u16) -> AdjustmentData {
    let mut data = shadows.to_be_bytes().to_vec();
    data.extend_from_slice(&highlights.to_be_bytes());
    AdjustmentData {
        key: *b"shdH",
        data: data.into(),
    }
}

pub fn encode_shadows_highlights_local(params: ShadowsHighlightsParams) -> Option<AdjustmentData> {
    let local = params.local.filter(|_| params.is_valid())?;
    let fields = [
        params.shadows_amount,
        params.highlights_amount,
        1.0,
        local.shadows_width,
        local.shadows_radius,
        local.highlights_width,
        local.highlights_radius,
        local.brightness,
        local.color_correction,
        local.midtone_contrast,
        local.black_clip * 100.0,
        local.white_clip * 100.0,
    ];
    if fields[..10].iter().any(|v| *v != v.round())
        || (local.black_clip * 100.0).round() / 100.0 != local.black_clip
        || (local.white_clip * 100.0).round() / 100.0 != local.white_clip
    {
        return None;
    }
    let data: Vec<u8> = fields
        .into_iter()
        .flat_map(|v| (v.round() as i16).to_be_bytes())
        .collect();
    Some(AdjustmentData {
        key: *b"shd2",
        data: data.into(),
    })
}

pub fn brightness_contrast_dialog(_ctx: Defaults) -> AdjustmentData {
    encode_brightness_contrast(0, 0)
}

pub fn brightness_contrast_layer(_ctx: Defaults) -> AdjustmentData {
    encode_brightness_contrast(20, 0)
}

pub fn levels_dialog(_ctx: Defaults) -> AdjustmentData {
    levels_identity()
}

pub fn curves_dialog(_ctx: Defaults) -> AdjustmentData {
    encode_curves(&[(0, 0), (255, 255)], None, None, None)
}

pub fn exposure_dialog(_ctx: Defaults) -> AdjustmentData {
    exposure_neutral()
}

pub fn vibrance_dialog(_ctx: Defaults) -> AdjustmentData {
    descriptor(
        *b"vibA",
        vec![
            (b"vibrance", DescValue::Long(0)),
            (b"Strt", DescValue::Long(0)),
        ],
    )
}

pub fn hue_saturation_dialog(_ctx: Defaults) -> AdjustmentData {
    encode_hue_saturation(0, 0, 0)
}

pub fn hue_saturation_layer(_ctx: Defaults) -> AdjustmentData {
    encode_hue_saturation(30, 0, 0)
}

pub fn color_balance_dialog(_ctx: Defaults) -> AdjustmentData {
    encode_color_balance([0.0; 3], [0.0; 3], [0.0; 3], true)
}

pub fn black_white_dialog(_ctx: Defaults) -> AdjustmentData {
    black_white_default()
}

pub fn photo_filter_dialog(_ctx: Defaults) -> AdjustmentData {
    encode_photo_filter([236, 138, 0], 25.0, true)
}

pub fn photo_filter_layer(_ctx: Defaults) -> AdjustmentData {
    encode_photo_filter([255, 180, 80], 25.0, true)
}

pub fn channel_mixer_dialog(ctx: Defaults) -> AdjustmentData {
    channel_mixer_identity(ctx.cmyk)
}

pub fn color_lookup_dialog(_ctx: Defaults) -> AdjustmentData {
    encode_color_lookup(&identity_cube(), "None")
}

pub fn posterize_dialog(_ctx: Defaults) -> AdjustmentData {
    encode_posterize(4)
}

pub fn threshold_dialog(_ctx: Defaults) -> AdjustmentData {
    encode_threshold(128)
}

pub fn gradient_map_dialog(ctx: Defaults) -> AdjustmentData {
    gradient_map_default(ctx.foreground, ctx.background)
}

pub fn gradient_map_layer(_ctx: Defaults) -> AdjustmentData {
    gradient_map_default([0, 0, 0], [255, 255, 255])
}

pub fn selective_color_dialog(_ctx: Defaults) -> AdjustmentData {
    encode_selective_color(
        SelectiveColorMethod::Relative,
        &[SelectiveRange::default(); 9],
    )
}

pub fn shadows_highlights_dialog(_ctx: Defaults) -> AdjustmentData {
    encode_shadows_highlights_local(ShadowsHighlightsParams {
        shadows_amount: 35.0,
        highlights_amount: 0.0,
        local: Some(ShadowsHighlightsOptions::FACTORY),
    })
    .expect("valid Shadows/Highlights factory settings")
}

pub fn invert_layer(_ctx: Defaults) -> AdjustmentData {
    encode_invert()
}

/// Levels at identity: version 2, then the composite record and 28 channel
/// records, each input `0..255`, output `0..255`, gamma `1.00`.
fn levels_identity() -> AdjustmentData {
    let mut data = 2u16.to_be_bytes().to_vec();
    for _ in 0..29 {
        for v in [0u16, 255, 0, 255, 100] {
            data.extend_from_slice(&v.to_be_bytes());
        }
    }
    AdjustmentData {
        key: *b"levl",
        data: data.into(),
    }
}

fn exposure_neutral() -> AdjustmentData {
    let mut data = 1u16.to_be_bytes().to_vec();
    for v in [0.0f32, 0.0, 1.0] {
        data.extend_from_slice(&v.to_be_bytes());
    }
    AdjustmentData {
        key: *b"expA",
        data: data.into(),
    }
}

fn black_white_default() -> AdjustmentData {
    descriptor(
        *b"blwh",
        [
            (&b"Rd  "[..], 40),
            (b"Yllw", 60),
            (b"Grn ", 40),
            (b"Cyn ", 60),
            (b"Bl  ", 20),
            (b"Mgnt", 80),
        ]
        .into_iter()
        .map(|(k, v)| (k, DescValue::Long(v)))
        .chain([
            (&b"useTint"[..], DescValue::Bool(false)),
            // The Tint swatch's default (hue 42°, saturation 20%), 0..1.
            (
                &b"tintColor"[..],
                DescValue::Object {
                    name: String::new(),
                    class_id: b"RGBC".to_vec(),
                    items: [(b"Rd  ", 0xe1), (b"Grn ", 0xd3), (b"Bl  ", 0xb4)]
                        .into_iter()
                        .map(|(k, v)| (k.to_vec(), DescValue::Double(f64::from(v) / 255.0)))
                        .collect(),
                },
            ),
        ])
        .collect(),
    )
}

fn channel_mixer_identity(cmyk: bool) -> AdjustmentData {
    if cmyk {
        encode_channel_mixer_ink(
            [
                [100.0, 0.0, 0.0, 0.0],
                [0.0, 100.0, 0.0, 0.0],
                [0.0, 0.0, 100.0, 0.0],
                [0.0, 0.0, 0.0, 100.0],
            ],
            [0.0; 4],
        )
    } else {
        encode_channel_mixer(
            false,
            [100.0, 0.0, 0.0],
            [0.0, 100.0, 0.0],
            [0.0, 0.0, 100.0],
            [0.0; 3],
        )
    }
}

fn gradient_map_default(foreground: [u8; 3], background: [u8; 3]) -> AdjustmentData {
    encode_gradient_map(
        &GradientMapParams {
            stops: vec![
                GradientStop {
                    location: 0,
                    color: foreground,
                },
                GradientStop {
                    location: 4096,
                    color: background,
                },
            ],
            reverse: false,
            transparency: Vec::new(),
        },
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmyk_channel_mixer_layer_default() {
        let rgb = channel_mixer_dialog(Defaults::rgb());
        let cmyk = channel_mixer_dialog(Defaults::rgb().with_cmyk(true));
        assert_ne!(rgb.data, cmyk.data, "the two identities differ");
        let Some(pictura_core::Adjustment::ChannelMixer(params)) = crate::decode_adjustment(&cmyk)
        else {
            panic!("a channel-mixer block decodes");
        };
        assert_eq!(
            params.ink,
            [
                [100.0, 0.0, 0.0, 0.0],
                [0.0, 100.0, 0.0, 0.0],
                [0.0, 0.0, 100.0, 0.0],
                [0.0, 0.0, 0.0, 100.0],
            ]
        );
        assert_eq!(params.ink_constant, [0.0; 4]);
    }

    #[test]
    fn representative_default_hooks_preserve_former_payloads() {
        let ctx = Defaults::rgb();
        assert_eq!(threshold_dialog(ctx), encode_threshold(128));
        assert_eq!(posterize_dialog(ctx), encode_posterize(4));
        assert_eq!(levels_dialog(ctx), levels_identity());
        assert_eq!(exposure_dialog(ctx), exposure_neutral());
        assert_eq!(
            color_lookup_dialog(ctx),
            encode_color_lookup(&identity_cube(), "None")
        );
    }

    #[test]
    fn gradient_map_dialog_follows_the_context_but_a_layer_does_not() {
        let ctx = Defaults::rgb().with_colors([10, 20, 30], [40, 50, 60]);
        let dialog_block = gradient_map_dialog(ctx);
        let layer_block = gradient_map_layer(ctx);
        assert_ne!(dialog_block.data, layer_block.data);
        assert_eq!(
            layer_block.data,
            gradient_map_dialog(Defaults::rgb()).data,
            "the layer stays black to white"
        );
    }
}
