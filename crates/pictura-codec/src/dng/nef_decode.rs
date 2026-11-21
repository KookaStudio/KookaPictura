//! Nikon NEF sensor data (#1469): the Huffman-coded prediction scheme with
//! its linearization curve (maker-note tag `0x0096`), and samples packed
//! least-significant bit first.

use super::ljpeg::{Bits, Huffman};
use super::tiff::Order;
use crate::error::PsdError;

fn invalid(what: &str) -> PsdError {
    PsdError::Invalid(format!("NEF: {what}"))
}

/// Nikon's six code tables as JPEG `BITS` (codes per length 1–16) and
/// `HUFFVAL` symbols: 12-bit lossy, lossy after the split, lossless, then the
/// same three for 14 bits. A symbol's low four bits are the difference length,
/// its high four the low bits dropped.
// Provenance: these are interoperability constants of the NEF format, the
// same in every decoder; taken as format facts by owner decision (#1469) and
// checked against real files (D70, D700, D850, D3300, D7000, Z f).
// The 12-bit lossy table's fourteenth code also stands for symbol 0.
const TABLES: [([u8; 16], &[u8]); 6] = [
    (
        [0, 1, 5, 1, 1, 1, 1, 1, 1, 2, 0, 0, 0, 0, 0, 0],
        &[5, 4, 3, 6, 2, 7, 1, 0, 8, 9, 11, 10, 12, 0],
    ),
    (
        [0, 1, 5, 1, 1, 1, 1, 1, 1, 2, 0, 0, 0, 0, 0, 0],
        &[0x39, 0x5a, 0x38, 0x27, 0x16, 5, 4, 3, 2, 1, 0, 11, 12, 12],
    ),
    (
        [0, 1, 4, 2, 3, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        &[5, 4, 6, 3, 7, 2, 8, 1, 9, 0, 10, 11, 12],
    ),
    (
        [0, 1, 4, 3, 1, 1, 1, 1, 1, 2, 0, 0, 0, 0, 0, 0],
        &[5, 6, 4, 7, 8, 3, 9, 2, 1, 0, 10, 11, 12, 13, 14],
    ),
    (
        [0, 1, 5, 1, 1, 1, 1, 1, 1, 1, 2, 0, 0, 0, 0, 0],
        &[8, 0x5c, 0x4b, 0x3a, 0x29, 7, 6, 5, 4, 3, 2, 1, 0, 13, 14],
    ),
    (
        [0, 1, 4, 2, 2, 3, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0],
        &[7, 6, 8, 5, 9, 4, 10, 3, 11, 12, 2, 0, 1, 13, 14],
    ),
];

const LOSSY: usize = 0;
const AFTER_SPLIT: usize = 1;
const LOSSLESS: usize = 2;

fn table(index: usize, bits: u32) -> Result<Huffman, PsdError> {
    let (counts, symbols) = &TABLES[index + if bits == 14 { 3 } else { 0 }];
    Huffman::new(counts, symbols.to_vec())
}

/// The decoding parameters of a linearization table.
pub(super) struct Linearization {
    lossless: bool,
    /// Initial vertical predictors, by row parity then column.
    vpred: [[i32; 2]; 2],
    /// Output by predicted value.
    curve: Vec<u16>,
    /// The first row coded with the after-split table; 0 for none.
    split: usize,
}

/// Reads the linearization table (maker-note tag `0x0096`) of a `bits`-deep
/// image, its 16-bit fields in the maker note's byte `order`.
pub(super) fn linearization(
    table: &[u8],
    order: Order,
    bits: u32,
) -> Result<Linearization, PsdError> {
    let u16_at =
        |at: usize| -> Option<u16> { table.get(at..at + 2).map(|b| order.u16([b[0], b[1]])) };
    let header = (0..5)
        .map(|i| u16_at(2 + 2 * i))
        .collect::<Option<Vec<u16>>>()
        .ok_or_else(|| invalid("linearization table shorter than its header"))?;
    let (ver0, ver1) = (table[0], table[1]);
    let p = |i: usize| header[i] as i16 as i32;
    let vpred = [[p(0), p(1)], [p(2), p(3)]];
    let size = header[4] as usize;
    let identity = || (0..1u32 << bits).map(|v| v as u16).collect::<Vec<u16>>();
    if ver0 == 0x46 {
        return Ok(Linearization {
            lossless: true,
            vpred,
            curve: identity(),
            split: 0,
        });
    }
    let max = 1usize << bits;
    let listed = |n: usize| -> Result<Vec<u16>, PsdError> {
        (0..n)
            .map(|i| u16_at(12 + 2 * i))
            .collect::<Option<Vec<u16>>>()
            .ok_or(PsdError::Truncated)
    };
    let step = if size > 1 { max / (size - 1) } else { 0 };
    let (curve, split) = if ver0 == 0x44 && ver1 == 0x20 && step > 0 {
        let points = listed(size)?;
        let last = (size - 1) * step;
        let curve = (0..max.max(last + 1))
            .map(|i| {
                if i >= last {
                    return points[size - 1];
                }
                let (k, f) = (i / step, i % step);
                let (a, b) = (points[k] as usize, points[k + 1] as usize);
                ((a * (step - f) + b * f) / step) as u16
            })
            .collect();
        (curve, u16_at(562).map_or(0, |v| v as usize))
    } else if (2..=0x4001).contains(&size) {
        (listed(size)?, 0)
    } else {
        (identity(), 0)
    };
    Ok(Linearization {
        lossless: false,
        vpred,
        curve,
        split,
    })
}

/// The difference coded by symbol `s` and the bits after it.
fn difference(bits: &mut Bits, s: u8) -> Result<i32, PsdError> {
    let (len, shl) = ((s & 15) as u32, (s >> 4) as u32);
    if len == 0 {
        return Ok(0);
    }
    if shl > len {
        return Err(invalid("Huffman symbol"));
    }
    let v = bits.take(len - shl) as i32;
    let magnitude = if shl == 0 {
        v
    } else {
        (v << shl) | (1 << (shl - 1))
    };
    Ok(if magnitude & (1 << (len - 1)) == 0 {
        magnitude - (1 << len) + (shl == 0) as i32
    } else {
        magnitude
    })
}

/// Decodes `width × height` Huffman-coded samples of a `bits`-deep image.
pub(super) fn decode_huffman(
    data: &[u8],
    lin: &Linearization,
    bits: u32,
    width: usize,
    height: usize,
) -> Result<Vec<u16>, PsdError> {
    let mut huff = table(if lin.lossless { LOSSLESS } else { LOSSY }, bits)?;
    let mut reader = Bits::plain(data);
    let mut vpred = lin.vpred;
    let top = lin.curve.len() as i32 - 1;
    let mut plane = vec![0u16; width * height];
    for (y, row) in plane.chunks_exact_mut(width).enumerate() {
        if lin.split > 0 && y == lin.split {
            huff = table(AFTER_SPLIT, bits)?;
        }
        let mut hpred = [0i32; 2];
        for (x, out) in row.iter_mut().enumerate() {
            let s = reader.decode(&huff)?;
            let diff = difference(&mut reader, s)?;
            if x < 2 {
                vpred[y & 1][x] += diff;
                hpred[x] = vpred[y & 1][x];
            } else {
                hpred[x & 1] += diff;
            }
            *out = lin.curve[hpred[x & 1].clamp(0, top) as usize];
        }
        if reader.overran() {
            return Err(PsdError::Truncated);
        }
    }
    Ok(plane)
}

/// Unpacks `width × height` samples of `bits` bits packed least-significant
/// bit first, each row starting on a byte.
pub(super) fn unpack_lsb(
    data: &[u8],
    bits: u32,
    width: usize,
    height: usize,
) -> Result<Vec<u16>, PsdError> {
    let row_bytes = (width * bits as usize).div_ceil(8);
    if data.len() < row_bytes * height {
        return Err(PsdError::Truncated);
    }
    let mask = (1u64 << bits) - 1;
    let mut plane = Vec::with_capacity(width * height);
    for src in data.chunks_exact(row_bytes).take(height) {
        let (mut acc, mut have, mut bytes) = (0u64, 0u32, src.iter());
        for _ in 0..width {
            while have < bits {
                acc |= (*bytes.next().unwrap_or(&0) as u64) << have;
                have += 8;
            }
            plane.push((acc & mask) as u16);
            acc >>= bits;
            have -= bits;
        }
    }
    Ok(plane)
}
