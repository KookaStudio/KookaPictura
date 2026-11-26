pub use pictura_codec::layer_content::curves::*;

#[cfg(test)]
use pictura_core::*;
#[cfg(test)]
mod tests {
    use super::*;

    fn payload(channels: &[(u16, &[(i16, i16)])]) -> Vec<u8> {
        let bitmask = channels
            .iter()
            .map(|(bit, _)| *bit)
            .fold(0u16, |a, b| a | b);
        let mut data = vec![0u8];
        data.extend_from_slice(&1u16.to_be_bytes());
        data.extend_from_slice(&0u16.to_be_bytes());
        data.extend_from_slice(&bitmask.to_be_bytes());
        for (_, points) in channels {
            data.extend_from_slice(&(points.len() as u16).to_be_bytes());
            for &(output, input) in *points {
                data.extend_from_slice(&output.to_be_bytes());
                data.extend_from_slice(&input.to_be_bytes());
            }
        }
        data
    }

    fn decode(data: Vec<u8>) -> Option<Adjustment> {
        decode_curves(&data)
    }

    #[test]
    fn decodes_composite_and_ignores_duplicate_section() {
        let data = payload(&[(1, &[(0, 0), (32, 64), (224, 192), (255, 255)])]);
        let expected = Some(Adjustment::Curves(CurvesParams {
            points: vec![(0, 0), (64, 32), (192, 224), (255, 255)],
            red: None,
            green: None,
            blue: None,
            black: None,
        }));
        assert_eq!(decode(data.clone()), expected);
        let mut with_extra = data;
        with_extra.extend_from_slice(b"Crv \x00\x04\x00\x00\x00\x01junk");
        assert_eq!(decode(with_extra), expected);
    }

    #[test]
    fn decodes_per_channel_and_disk_to_model_order() {
        let data = payload(&[
            (1, &[(0, 0), (255, 255)]),
            (2, &[(0, 0), (255, 128), (255, 255)]),
            (4, &[(0, 0), (16, 64), (255, 255)]),
            (8, &[(255, 0), (0, 255)]),
        ]);
        assert_eq!(
            decode(data),
            Some(Adjustment::Curves(CurvesParams {
                points: vec![(0, 0), (255, 255)],
                red: Some(vec![(0, 0), (128, 255), (255, 255)]),
                green: Some(vec![(0, 0), (64, 16), (255, 255)]),
                blue: Some(vec![(0, 255), (255, 0)]),
                black: None,
            }))
        );
    }

    #[test]
    fn defaults_missing_rgb_channel_to_identity() {
        let data = payload(&[(2, &[(0, 0), (255, 128), (255, 255)])]);
        assert_eq!(
            decode(data),
            Some(Adjustment::Curves(CurvesParams {
                points: vec![(0, 0), (255, 255)],
                red: Some(vec![(0, 0), (128, 255), (255, 255)]),
                green: None,
                blue: None,
                black: None,
            }))
        );
    }

    #[test]
    fn rejects_malformed_never_panics() {
        let base = payload(&[(1, &[(0, 0), (32, 64), (255, 255)])]);
        let reject = |mutate: &dyn Fn(&mut Vec<u8>)| {
            let mut data = base.clone();
            mutate(&mut data);
            assert_eq!(decode(data), None);
        };
        reject(&|d| d[2] = 2); // version != 1
        reject(&|d| d[0] = 1); // legacy is_map leading byte
        reject(&|d| d[5..7].copy_from_slice(&0u16.to_be_bytes())); // zero bitmask
        reject(&|d| d[5..7].copy_from_slice(&16u16.to_be_bytes())); // unknown bit
        reject(&|d| d[7..9].copy_from_slice(&1u16.to_be_bytes())); // count < 2
        reject(&|d| d[7..9].copy_from_slice(&15u16.to_be_bytes())); // count > 14
        reject(&|d| d[9..11].copy_from_slice(&(-1i16).to_be_bytes())); // output out of range
        reject(&|d| d[11..13].copy_from_slice(&(-1i16).to_be_bytes())); // input out of range
        reject(&|d| d[15..17].copy_from_slice(&0i16.to_be_bytes())); // non-increasing
        let truncated = base[..base.len() - 1].to_vec();
        assert_eq!(decode(truncated), None);
    }

    #[test]
    fn encode_round_trips_and_omits_absent_channels() {
        let composite = encode_curves(
            &[(0, 0), (64, 32), (192, 224), (255, 255)],
            None,
            None,
            None,
        );
        assert_eq!(composite.key, *b"curv");
        assert_eq!(composite.data[5..7], [0, 1], "only the rgb bit is set");
        assert!(
            !composite.data.windows(4).any(|w| w == b"Crv "),
            "the encoder must not write the duplicate Crv section"
        );
        assert_eq!(
            decode_curves(&composite.data),
            Some(Adjustment::Curves(CurvesParams {
                points: vec![(0, 0), (64, 32), (192, 224), (255, 255)],
                red: None,
                green: None,
                blue: None,
                black: None,
            }))
        );

        let all = encode_curves(
            &[(0, 0), (255, 255)],
            Some(&[(0, 0), (128, 255), (255, 255)]),
            Some(&[(0, 0), (64, 16), (255, 255)]),
            Some(&[(0, 255), (255, 0)]),
        );
        assert_eq!(all.data[5..7], [0, 15], "all four bits are set");
        assert_eq!(
            decode_curves(&all.data),
            Some(Adjustment::Curves(CurvesParams {
                points: vec![(0, 0), (255, 255)],
                red: Some(vec![(0, 0), (128, 255), (255, 255)]),
                green: Some(vec![(0, 0), (64, 16), (255, 255)]),
                blue: Some(vec![(0, 255), (255, 0)]),
                black: None,
            }))
        );
    }
}
