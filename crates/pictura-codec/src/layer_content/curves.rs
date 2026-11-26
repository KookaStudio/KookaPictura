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
const FOOTER_MAGIC: &[u8; 4] = b"PkCv";
const FOOTER_SIZE: usize = 7;

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
    let mut channels = [None, None, None, None, None];
    channels[..4].swap_with_slice(&mut curves);
    if let Some(tables) = d.get(at..).and_then(trailer).and_then(|(_, version, body)| {
        (version == 1).then(|| read_trailer(body, alloc)).flatten()
    }) {
        for (channel, table) in channels.iter_mut().zip(tables) {
            if table.is_some() {
                *channel = table;
            }
        }
    }
    let [rgb, red, green, blue, black] = channels;
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
        black,
    }))
}

type CurveSlots = [Option<Vec<(u8, u8)>>; 5];

fn trailer(data: &[u8]) -> Option<(usize, u8, &[u8])> {
    let end = data.len().checked_sub(FOOTER_SIZE)?;
    let footer = &data[end..];
    if &footer[3..] != FOOTER_MAGIC {
        return None;
    }
    let size = usize::from(u16::from_be_bytes([footer[1], footer[2]]));
    let start = end.checked_sub(size)?;
    Some((start, footer[0], &data[start..end]))
}

fn read_trailer(mut body: &[u8], alloc: &mut ContentAlloc) -> Option<CurveSlots> {
    let mut tables: CurveSlots = Default::default();
    while !body.is_empty() {
        let channel = usize::from(body[0]);
        let slot = tables.get_mut(channel)?;
        if slot.is_some() {
            return None;
        }
        match *body.get(1)? {
            0 if channel == 4 => {
                let mut at = 2;
                *slot = Some(read_curve(body, &mut at, alloc)?);
                body = body.get(at..)?;
            }
            1 => {
                let values = body.get(2..258)?;
                let mut table = alloc.vec(256, "dense curve points")?;
                table.extend(values.iter().enumerate().map(|(x, &y)| (x as u8, y)));
                *slot = Some(table);
                body = &body[258..];
            }
            _ => return None,
        }
    }
    Some(tables)
}

fn legacy_suffix(data: &[u8]) -> Option<&[u8]> {
    let bits = be_u16(data, 5)?;
    let mut at = 7;
    for bit in CHANNEL_BITS {
        if bits & bit != 0 {
            at += 2 + usize::from(be_u16(data, at)?) * 4;
        }
    }
    let suffix = data.get(at..)?;
    let suffix = &suffix[duplicate_end(suffix).unwrap_or(0)..];
    if let Some((start, 1, body)) = trailer(suffix) {
        if read_trailer(body, &mut ContentAlloc::new(None)).is_some() {
            return suffix.get(..start);
        }
    }
    Some(suffix)
}

// Only a complete v4 record at the standard boundary is safe to discard.
fn duplicate_end(data: &[u8]) -> Option<usize> {
    if data.get(..4)? != b"Crv " || be_u16(data, 4)? != 4 {
        return None;
    }
    let count = u32::from_be_bytes(data.get(6..10)?.try_into().ok()?);
    if !(1..=32).contains(&count) {
        return None;
    }
    let mut at = 10;
    let mut alloc = ContentAlloc::new(None);
    for _ in 0..count {
        read_curve(data, &mut at, &mut alloc)?;
    }
    Some(at)
}

/// Encode typed curves, preserving unrelated bytes following the source's legacy records.
pub fn encode_curves_params(p: &CurvesParams, source: Option<&AdjustmentData>) -> AdjustmentData {
    let channels = [
        Some(p.points.as_slice()),
        p.red.as_deref(),
        p.green.as_deref(),
        p.blue.as_deref(),
        p.black.as_deref(),
    ];
    let fallback = channels.map(|curve| curve.map(|points| {
        if points.len() == 256 {
            (0..14).map(|i| points[(i * 255 + 6) / 13]).collect()
        } else {
            points.to_vec()
        }
    }));
    let base = encode_curves(
        fallback[0].as_deref().unwrap_or_default(),
        fallback[1].as_deref(),
        fallback[2].as_deref(),
        fallback[3].as_deref(),
    );
    let mut data = base.data.to_vec();
    if let Some(suffix) = source.and_then(|source| legacy_suffix(&source.data)) {
        data.extend_from_slice(suffix);
    }
    let start = data.len();
    for (channel, points) in channels.into_iter().enumerate() {
        let Some(points) = points else {
            continue;
        };
        if points.len() == 256 {
            data.extend_from_slice(&[channel as u8, 1]);
            data.extend(points.iter().map(|&(_, y)| y));
        } else if channel == 4 {
            data.extend_from_slice(&[4, 0]);
            write_curve(&mut data, points);
        }
    }
    let length = data.len() - start;
    if length != 0 {
        data.push(1);
        data.extend_from_slice(&(length as u16).to_be_bytes());
        data.extend_from_slice(FOOTER_MAGIC);
    }
    AdjustmentData { key: base.key, data: data.into() }
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
