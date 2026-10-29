//! Curves (`curv`) decode and encode.
//!
//! Kept out of `composite.rs` so the compositor stays within its file-size
//! budget, mirroring `channel_mixer.rs`. The layout is ag-psd's version-1 block:
//! a `u8` ignored byte, a `u16` version (1), a `u16` ignored word, and a `u16`
//! channel bitmask (`1` rgb, `2` red, `4` green, `8` blue); for each set bit, in
//! the order rgb, red, green, blue, a `u16` node count followed by that many
//! `(i16 output, i16 input)` pairs. The version-4 duplicate `Crv ` section and
//! any trailing bytes are ignored.
//!
//! ponytail: the legacy `is_map` 256-byte bitmap form is unsupported and decodes
//! to `None` (its leading byte is nonzero); add it if a real `is_map` fixture
//! appears.

use pictura_adjust::{Adjustment, CurvesParams};
use pictura_core::AdjustmentData;

use crate::composite::{be_i16, be_u16};

/// Bitmask bit per channel, in the on-disk order.
const CHANNEL_BITS: [u16; 4] = [1, 2, 4, 8];

/// Read one channel's `u16` node count and that many `(output, input)` pairs,
/// returning the model `(input, output)` points. Enforces the shared curve
/// contract; a violation is `None`.
fn read_curve(d: &[u8], at: &mut usize) -> Option<Vec<(u8, u8)>> {
    let count = be_u16(d, *at)?;
    if !(2..=14).contains(&count) {
        return None;
    }
    *at += 2;
    let mut points = Vec::with_capacity(count as usize);
    let mut previous: Option<i16> = None;
    for _ in 0..count {
        let output = be_i16(d, *at)?;
        let input = be_i16(d, *at + 2)?;
        *at += 4;
        if !(0..=255).contains(&output) || !(0..=255).contains(&input) {
            return None;
        }
        if previous.is_some_and(|p| input <= p) {
            return None;
        }
        previous = Some(input);
        points.push((input as u8, output as u8));
    }
    Some(points)
}

/// `curv`: version must be `1`, the bitmask nonzero with no bit outside
/// `0b1111`, and every present channel must satisfy the curve contract;
/// otherwise `None`, never a panic. Trailing bytes are ignored.
pub(crate) fn decode_curves(d: &[u8]) -> Option<Adjustment> {
    // A nonzero lead byte is the legacy `is_map` table form, not the version-1
    // node-list block this decoder models.
    if d.first().copied()? != 0 {
        return None;
    }
    if be_u16(d, 1)? != 1 {
        return None;
    }
    let bitmask = be_u16(d, 5)?;
    if bitmask == 0 || bitmask & !0b1111 != 0 {
        return None;
    }
    let mut at = 7;
    let mut curves: [Option<Vec<(u8, u8)>>; 4] = [None, None, None, None];
    for (slot, bit) in curves.iter_mut().zip(CHANNEL_BITS) {
        if bitmask & bit != 0 {
            *slot = Some(read_curve(d, &mut at)?);
        }
    }
    let [rgb, red, green, blue] = curves;
    Some(Adjustment::Curves(CurvesParams {
        points: rgb.unwrap_or_else(|| vec![(0, 0), (255, 255)]),
        red,
        green,
        blue,
    }))
}

fn write_curve(data: &mut Vec<u8>, points: &[(u8, u8)]) {
    data.extend_from_slice(&(points.len() as u16).to_be_bytes());
    for &(input, output) in points {
        data.extend_from_slice(&(output as i16).to_be_bytes());
        data.extend_from_slice(&(input as i16).to_be_bytes());
    }
}

/// `curv`: write the version-1 bitmask block `decode_curves` reads, with a bit
/// for each present non-empty channel, in the order rgb, red, green, blue. The
/// duplicate `Crv ` version-4 section is not written.
///
/// Expects the op-valid form the kernel accepts: `2..=14` strictly-increasing
/// `(input, output)` points with coordinates in `0..=255`. It does not validate.
pub fn encode_curves(
    points: &[(u8, u8)],
    red: Option<&[(u8, u8)]>,
    green: Option<&[(u8, u8)]>,
    blue: Option<&[(u8, u8)]>,
) -> AdjustmentData {
    let channels: [&[(u8, u8)]; 4] = [
        points,
        red.unwrap_or(&[]),
        green.unwrap_or(&[]),
        blue.unwrap_or(&[]),
    ];
    let bitmask = channels
        .iter()
        .zip(CHANNEL_BITS)
        .filter(|(curve, _)| !curve.is_empty())
        .fold(0u16, |mask, (_, bit)| mask | bit);
    let mut data = vec![0u8];
    data.extend_from_slice(&1u16.to_be_bytes());
    data.extend_from_slice(&0u16.to_be_bytes());
    data.extend_from_slice(&bitmask.to_be_bytes());
    for curve in channels {
        if !curve.is_empty() {
            write_curve(&mut data, curve);
        }
    }
    AdjustmentData {
        key: *b"curv",
        data: data.into(),
    }
}

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
            }))
        );
    }
}
