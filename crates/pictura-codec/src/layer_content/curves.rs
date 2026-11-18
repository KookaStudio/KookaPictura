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

use crate::layer_content_budget::ContentAlloc;
use pictura_core::AdjustmentData;
use pictura_core::{Adjustment, CurvesParams};

use super::basic::{be_i16, be_u16};

/// Bitmask bit per channel, in the on-disk order.
const CHANNEL_BITS: [u16; 4] = [1, 2, 4, 8];

/// Read one channel's `u16` node count and that many `(output, input)` pairs,
/// returning the model `(input, output)` points. Enforces the shared curve
/// contract; a violation is `None`.
fn read_curve(d: &[u8], at: &mut usize, alloc: &mut ContentAlloc) -> Option<Vec<(u8, u8)>> {
    let count = be_u16(d, *at)?;
    if !(2..=14).contains(&count) {
        return None;
    }
    *at += 2;
    let mut points = alloc.vec(count as usize, "curve points")?;
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
pub fn decode_curves(d: &[u8]) -> Option<Adjustment> {
    decode_curves_using(d, &mut ContentAlloc::new(None))
}

pub(crate) fn decode_curves_using(d: &[u8], alloc: &mut ContentAlloc) -> Option<Adjustment> {
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
            *slot = Some(read_curve(d, &mut at, alloc)?);
        }
    }
    let [rgb, red, green, blue] = curves;
    Some(Adjustment::Curves(CurvesParams {
        points: match rgb {
            Some(points) => points,
            None => {
                let mut points = alloc.vec(2, "identity curve points")?;
                points.extend_from_slice(&[(0, 0), (255, 255)]);
                points
            }
        },
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
