//! The block an Image > Adjustments dialog opens on: each kind at CS6's dialog
//! defaults, in the same encoding an adjustment layer stores, so the dialog and
//! the Properties panel share [`crate::adjustment_editor`]'s controls.

use pictura_adjust::{GradientMapParams, GradientStop, SelectiveColorMethod, SelectiveRange};
use pictura_codec::DescValue;
use pictura_core::AdjustmentData;

use crate::{
    encode_brightness_contrast, encode_channel_mixer, encode_color_balance, encode_color_lookup,
    encode_curves, encode_gradient_map, encode_hue_saturation, encode_photo_filter,
    encode_posterize, encode_selective_color, encode_threshold, identity_cube,
};

fn descriptor(key: [u8; 4], items: Vec<(&[u8], DescValue)>) -> AdjustmentData {
    let obj = DescValue::Object {
        name: String::new(),
        class_id: b"null".to_vec(),
        items: items.into_iter().map(|(k, v)| (k.to_vec(), v)).collect(),
    };
    AdjustmentData {
        key,
        data: pictura_codec::write_descriptor(&obj).into(),
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

/// `kind`'s block at CS6's dialog defaults; Gradient Map runs from
/// `foreground` to `background`, as CS6's does. `None` for an unknown kind.
pub fn default_adjustment_block(
    kind: &str,
    foreground: [u8; 3],
    background: [u8; 3],
) -> Option<AdjustmentData> {
    Some(match kind {
        "brightness-contrast" => encode_brightness_contrast(0, 0),
        "levels" => {
            // Version 2, then the composite record and 28 channel records,
            // each input 0..255, output 0..255, gamma 1.00.
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
        "curves" => encode_curves(&[(0, 0), (255, 255)], None, None, None),
        "exposure" => {
            let mut data = 1u16.to_be_bytes().to_vec();
            for v in [0.0f32, 0.0, 1.0] {
                data.extend_from_slice(&v.to_be_bytes());
            }
            AdjustmentData {
                key: *b"expA",
                data: data.into(),
            }
        }
        "vibrance" => descriptor(
            *b"vibA",
            vec![
                (b"vibrance", DescValue::Long(0)),
                (b"Strt", DescValue::Long(0)),
            ],
        ),
        "hue-saturation" => encode_hue_saturation(0, 0, 0),
        "color-balance" => encode_color_balance([0.0; 3], [0.0; 3], [0.0; 3], true),
        "black-white" => descriptor(
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
        ),
        // CS6's default, Warming Filter (85).
        "photo-filter" => encode_photo_filter([236, 138, 0], 25.0, true),
        "channel-mixer" => encode_channel_mixer(
            false,
            [100.0, 0.0, 0.0],
            [0.0, 100.0, 0.0],
            [0.0, 0.0, 100.0],
            [0.0; 3],
        ),
        "posterize" => encode_posterize(4),
        "threshold" => encode_threshold(128),
        "gradient-map" => encode_gradient_map(
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
        ),
        "selective-color" => encode_selective_color(
            SelectiveColorMethod::Relative,
            &[SelectiveRange::default(); 9],
        ),
        "shadows-highlights" => encode_shadows_highlights(0, 0),
        "color-lookup" => encode_color_lookup(&identity_cube(), "None"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{adjustment_editor, ImageCommand, ADJUSTMENT_CATALOGUE};

    #[test]
    fn every_dialog_opens_on_its_defaults() {
        let dialogs = ADJUSTMENT_CATALOGUE
            .iter()
            .filter(|entry| entry.image == ImageCommand::AdjustmentsDialog);
        for entry in dialogs {
            let kind = entry.kind;
            let block = default_adjustment_block(kind, [0, 0, 0], [255, 255, 255])
                .unwrap_or_else(|| panic!("{kind} has no default"));
            let editor = adjustment_editor(&block).unwrap_or_else(|| panic!("{kind} decodes"));
            assert_eq!(editor.title, entry.title);
            for param in &editor.params {
                assert_eq!(
                    param.value, param.default,
                    "{kind}.{} starts at its default",
                    param.key
                );
            }
        }
        assert!(default_adjustment_block("match-color", [0; 3], [0; 3]).is_none());
    }
}
