pub use pictura_codec::layer_content::selective_color::*;

#[cfg(test)]
use pictura_core::*;
#[cfg(test)]
mod tests {
    use super::*;

    /// Build an 84-byte payload: version 1, `method`, a reserved plate of
    /// `reserved`, and the nine `ranges`.
    fn payload(method: u16, reserved: i16, ranges: [SelectiveRange; 9]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&1u16.to_be_bytes());
        data.extend_from_slice(&method.to_be_bytes());
        for _ in 0..4 {
            data.extend_from_slice(&reserved.to_be_bytes());
        }
        for range in ranges {
            for value in [range.c, range.m, range.y, range.k] {
                data.extend_from_slice(&value.to_be_bytes());
            }
        }
        data
    }

    fn decode(data: &[u8]) -> Option<SelectiveColorParams> {
        match decode_selective_color(data)? {
            Adjustment::SelectiveColor(params) => Some(params),
            _ => panic!("selc must decode to SelectiveColor"),
        }
    }

    fn authored() -> [SelectiveRange; 9] {
        let mut ranges = [SelectiveRange::default(); 9];
        ranges[0] = SelectiveRange {
            c: 10,
            m: -20,
            y: 30,
            k: 0,
        };
        ranges[8] = SelectiveRange {
            c: 0,
            m: 0,
            y: 0,
            k: -40,
        };
        ranges
    }

    #[test]
    fn decodes_relative_and_absolute() {
        let ranges = authored();
        assert_eq!(
            decode(&payload(0, 0, ranges)),
            Some(SelectiveColorParams {
                method: SelectiveColorMethod::Relative,
                ranges,
            })
        );
        assert_eq!(
            decode(&payload(7, 0, ranges)).map(|p| p.method),
            Some(SelectiveColorMethod::Absolute),
            "any nonzero method is absolute"
        );
    }

    #[test]
    fn ignores_reserved_plate_and_trailing_bytes() {
        let mut data = payload(0, -100, authored());
        data.extend_from_slice(b"trailing");
        assert_eq!(decode(&data), decode(&payload(0, 0, authored())));
    }

    #[test]
    fn rejects_malformed_never_panics() {
        let base = payload(0, 0, authored());
        let mut bad_version = base.clone();
        bad_version[1] = 2;
        assert_eq!(decode_selective_color(&bad_version), None);
        assert_eq!(decode_selective_color(&base[..LEN - 1]), None);
        let mut out_of_range = base.clone();
        out_of_range[12..14].copy_from_slice(&101i16.to_be_bytes());
        assert_eq!(decode_selective_color(&out_of_range), None);
        let mut reserved_out_of_range = base;
        reserved_out_of_range[4..6].copy_from_slice(&300i16.to_be_bytes());
        assert!(
            decode_selective_color(&reserved_out_of_range).is_some(),
            "the reserved plate is not validated"
        );
    }

    #[test]
    fn encoder_round_trips_and_clamps() {
        let neutral = encode_selective_color(
            SelectiveColorMethod::Relative,
            &[SelectiveRange::default(); 9],
        );
        assert_eq!(neutral.key, *b"selc");
        assert_eq!(neutral.data.len(), LEN);
        assert_eq!(&neutral.data[..2], &1u16.to_be_bytes());
        assert_eq!(&neutral.data[2..4], &0u16.to_be_bytes());
        assert_eq!(&neutral.data[4..12], &[0u8; 8]);
        assert_eq!(decode(&neutral.data), Some(SelectiveColorParams::default()));

        let ranges = authored();
        let encoded = encode_selective_color(SelectiveColorMethod::Absolute, &ranges);
        assert_eq!(
            decode(&encoded.data),
            Some(SelectiveColorParams {
                method: SelectiveColorMethod::Absolute,
                ranges,
            })
        );

        let mut clamped = [SelectiveRange::default(); 9];
        clamped[0] = SelectiveRange {
            c: 1000,
            m: -1000,
            y: 0,
            k: 0,
        };
        assert_eq!(
            decode(&encode_selective_color(SelectiveColorMethod::Relative, &clamped).data),
            Some(SelectiveColorParams {
                method: SelectiveColorMethod::Relative,
                ranges: {
                    let mut expected = [SelectiveRange::default(); 9];
                    expected[0] = SelectiveRange {
                        c: 100,
                        m: -100,
                        y: 0,
                        k: 0,
                    };
                    expected
                },
            })
        );
    }

    #[test]
    fn non_zero_layer_changes_non_uniform_backdrop() {
        use pictura_core::{BitDepth, BlendMode, Channel, ColorMode, Document, Layer, PsdRect};

        let base = pictura_codec::layer_from_parts(pictura_core::LayerParts {
            name: "base".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 1,
                right: 2,
            },
            blend: BlendMode::Normal,
            opacity: 255,
            fill: 255,
            visible: true,
            channels: vec![
                Channel {
                    id: 0,
                    data: vec![200, 10].into(),
                },
                Channel {
                    id: 1,
                    data: vec![100, 20].into(),
                },
                Channel {
                    id: 2,
                    data: vec![50, 30].into(),
                },
                Channel {
                    id: -1,
                    data: vec![255, 255].into(),
                },
            ],
            ..Default::default()
        });
        let mut ranges = [SelectiveRange::default(); 9];
        ranges[0].m = 50;
        let adjustment = pictura_codec::layer_from_parts(pictura_core::LayerParts {
            name: "selective-color".into(),
            blend: BlendMode::Normal,
            opacity: 255,
            visible: true,
            adjustment: Some(encode_selective_color(
                SelectiveColorMethod::Relative,
                &ranges,
            )),
            ..Default::default()
        });

        let mut plain = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
        plain.layers = pictura_core::LayerStack::from_layers(vec![base.clone()]);
        let mut adjusted = Document::new(2, 1, ColorMode::Rgb, BitDepth::Eight);
        adjusted.layers = pictura_core::LayerStack::from_layers(vec![base, adjustment]);

        assert_ne!(
            crate::composite_rgba(&adjusted).data,
            crate::composite_rgba(&plain).data,
            "a non-zero Selective Color layer must change a non-uniform backdrop"
        );
    }
}
