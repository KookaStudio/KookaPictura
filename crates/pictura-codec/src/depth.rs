//! Depth-aware row stride, 16/32-bit sample narrowing, and ZIP-with-prediction
//! inversion. Kept out of `read.rs` only to respect the file-size cap; the
//! encodings mirror `psd-tools`' `compression/__init__.py`.

#[cfg(test)]
use pictura_core::Samples;
use pictura_core::{BitDepth, Document};

use crate::error::PsdError;

/// The length in bytes of `channels` planar `row_bytes`-wide rows, or a typed
/// error when the product overflows `usize`.
pub(crate) fn planar_len(
    channels: usize,
    row_bytes: usize,
    height: usize,
) -> Result<usize, PsdError> {
    channels
        .checked_mul(row_bytes)
        .and_then(|n| n.checked_mul(height))
        .ok_or_else(|| PsdError::Invalid("image dimensions overflow".into()))
}

/// The [`BitDepth`] for a 16/32-bit sample width, else `None`.
pub(crate) fn depth_bits(depth: u16) -> Option<BitDepth> {
    match depth {
        16 => Some(BitDepth::Sixteen),
        32 => Some(BitDepth::ThirtyTwo),
        _ => None,
    }
}

/// The PSD sample width for a recorded source depth, defaulting to 8.
pub(crate) fn depth_of(source: Option<BitDepth>) -> u16 {
    match source {
        Some(BitDepth::One) => 1,
        Some(BitDepth::Sixteen) => 16,
        Some(BitDepth::ThirtyTwo) => 32,
        _ => 8,
    }
}

/// The PSD sample width to emit for `doc`: the recorded 16/32-bit source depth
/// when a read retained native samples, else 8. An 8-bit Lab read retains an
/// 8-bit store and a 16/32-bit converted mode retains nothing, so both save
/// 8-bit. A depth-1 Bitmap store is handled by the Bitmap write-back path; a
/// `One` reaching here is an inconsistent hand-built document (the fields are
/// `pub`), so it is a typed error rather than a sample width the narrowing code
/// cannot handle.
pub(crate) fn output_depth(doc: &Document) -> Result<u16, PsdError> {
    if !doc.retains_source_depth() {
        return Ok(8);
    }
    match doc.source_depth {
        Some(BitDepth::Sixteen) => Ok(16),
        Some(BitDepth::ThirtyTwo) => Ok(32),
        Some(BitDepth::One) => Err(PsdError::Invalid("depth-1 source store".into())),
        _ => Ok(8),
    }
}

/// One channel row's byte stride at `depth` bits per sample: `ceil(width *
/// depth / 8)`, so a depth-1 Bitmap row is `ceil(width / 8)`, a depth-16 row is
/// `2 * width`, and a depth-32 row is `4 * width`.
pub(crate) fn row_bytes(width: usize, depth: u16) -> usize {
    (width * depth as usize).div_ceil(8)
}

/// Test-only byte adapter over [`Samples`]: narrow one contiguous 16/32-bit
/// plane to `width * height` 8-bit samples.
#[cfg(test)]
pub(crate) fn narrow_channel(
    plane: &[u8],
    _width: usize,
    _height: usize,
    _row_bytes: usize,
    depth: u16,
) -> Vec<u8> {
    Samples::from_bytes(plane, depth_bits(depth).expect("16/32-bit plane")).narrow_to_u8()
}

/// Widen one 8-bit plane (`width * height` samples) to the native-depth
/// layout: depth 16 is `v * 257`, depth 32 scales the 8-bit value to `[0, 1]`.
/// Test-only byte adapter over [`Samples`].
#[cfg(test)]
pub(crate) fn widen_channel(plane: &[u8], _width: usize, _height: usize, depth: u16) -> Vec<u8> {
    Samples::widen_from_u8(plane, depth_bits(depth).expect("16/32-bit plane")).to_bytes()
}

/// Widen a planar 8-bit buffer of `channels` planes to the native-depth layout.
/// Used by tests as the whole-buffer counterpart of [`widen_channel`].
#[cfg(test)]
pub(crate) fn widen_planes(
    data: &[u8],
    channels: usize,
    width: usize,
    height: usize,
    depth: u16,
) -> Vec<u8> {
    let plane = width * height;
    let mut out = Vec::with_capacity(channels * row_bytes(width, depth) * height);
    for c in 0..channels {
        out.extend_from_slice(&widen_channel(
            &data[c * plane..(c + 1) * plane],
            width,
            height,
            depth,
        ));
    }
    out
}

/// Invert the delta applied by ZIP-with-prediction over `rows` scanlines of
/// `width` pixels. Depth 8 is byte-wise; depth 16 is a per-row running sum of
/// big-endian `u16` values mod 2^16; depth 32 first undoes the byte-wise delta
/// over each `4 * width`-byte row, then un-shuffles the four byte planes back
/// into big-endian floats. Depth 1 never reaches ZIP (rejected earlier).
pub(crate) fn undo_prediction(data: &mut [u8], width: usize, rows: usize, depth: u16) {
    use rayon::prelude::*;

    if width == 0 {
        return;
    }
    match depth {
        16 => {
            let row = 2 * width;
            data.par_chunks_mut(row).take(rows).for_each(|chunk| {
                // A short final row is skipped, as the serial loop breaks.
                if chunk.len() < row {
                    return;
                }
                let mut prev = 0u16;
                for x in 0..width {
                    let i = x * 2;
                    let v = u16::from_be_bytes([chunk[i], chunk[i + 1]]).wrapping_add(prev);
                    chunk[i..i + 2].copy_from_slice(&v.to_be_bytes());
                    prev = v;
                }
            });
        }
        32 => {
            let row = 4 * width;
            data.par_chunks_mut(row)
                .take(rows)
                .for_each_init(Vec::new, |scratch, chunk| {
                    for i in 1..chunk.len() {
                        chunk[i] = chunk[i].wrapping_add(chunk[i - 1]);
                    }
                    // The serial unshuffle breaks on a short row, so only a full
                    // row is unshuffled.
                    if chunk.len() == row {
                        unshuffle_row_32(chunk, width, scratch);
                    }
                });
        }
        _ => {
            // Depth 8 covers every remaining byte, short final row included.
            data.par_chunks_mut(width).for_each(|chunk| {
                for i in 1..chunk.len() {
                    chunk[i] = chunk[i].wrapping_add(chunk[i - 1]);
                }
            });
        }
    }
}

/// Undo the depth-32 four-byte-plane shuffle in place for one `4 * width`-byte
/// row: psd-tools packs each row's four byte planes together (`12341234` ->
/// `111222333444`) before the byte-wise delta, so this restores the big-endian
/// float order.
fn unshuffle_row_32(chunk: &mut [u8], width: usize, scratch: &mut Vec<u8>) {
    scratch.clear();
    scratch.extend_from_slice(chunk);
    let mut dst = 0;
    for offset in 0..width {
        let mut x = offset;
        while x < chunk.len() {
            chunk[dst] = scratch[x];
            dst += 1;
            x += width;
        }
    }
}

/// Apply the forward ZIP-with-prediction delta, the inverse of
/// [`undo_prediction`]: depth 8 is byte-wise, depth 16 a per-`u16` difference,
/// depth 32 the byte-wise difference after the forward four-byte-plane shuffle.
pub(crate) fn apply_prediction(data: &mut [u8], width: usize, rows: usize, depth: u16) {
    if width == 0 {
        return;
    }
    match depth {
        16 => {
            let row = 2 * width;
            for r in 0..rows {
                let start = r * row;
                if start + row > data.len() {
                    break;
                }
                for x in (1..width).rev() {
                    let i = start + x * 2;
                    let cur = u16::from_be_bytes([data[i], data[i + 1]]);
                    let prev = u16::from_be_bytes([data[i - 2], data[i - 1]]);
                    data[i..i + 2].copy_from_slice(&cur.wrapping_sub(prev).to_be_bytes());
                }
            }
        }
        32 => {
            shuffle_32(data, width, rows);
            let row = 4 * width;
            for r in 0..rows {
                let row_start = r * row;
                let row_end = (row_start + row).min(data.len());
                for i in (row_start + 1..row_end).rev() {
                    data[i] = data[i].wrapping_sub(data[i - 1]);
                }
            }
        }
        _ => {
            for row_start in (0..data.len()).step_by(width) {
                let row_end = (row_start + width).min(data.len());
                for i in (row_start + 1..row_end).rev() {
                    data[i] = data[i].wrapping_sub(data[i - 1]);
                }
            }
        }
    }
}

/// Apply the depth-32 four-byte-plane shuffle in place, the inverse of
/// [`unshuffle_row_32`].
fn shuffle_32(data: &mut [u8], width: usize, rows: usize) {
    let row = 4 * width;
    let src = data.to_vec();
    for r in 0..rows {
        let base = r * row;
        if base + row > data.len() {
            break;
        }
        let mut k = base;
        for offset in base..base + width {
            let mut x = offset;
            while x < base + row {
                data[x] = src[k];
                k += 1;
                x += width;
            }
        }
    }
}

/// Forward of the depth-16 per-`u16` running sum: each row stores its first
/// sample then big-endian differences, matching psd-tools `encode_prediction`.
#[cfg(test)]
pub(crate) fn predict16(samples: &[u16], width: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples.len() * 2);
    for (i, &v) in samples.iter().enumerate() {
        let encoded = if width == 0 || i % width == 0 {
            v
        } else {
            v.wrapping_sub(samples[i - 1])
        };
        out.extend_from_slice(&encoded.to_be_bytes());
    }
    out
}

/// Forward of the depth-32 codec: shuffle the four byte planes of each row
/// together, then byte-wise delta. Matches psd-tools `encode_prediction`.
#[cfg(test)]
pub(crate) fn predict32(be: &[u8], width: usize, rows: usize) -> Vec<u8> {
    let row = 4 * width;
    let mut out = be.to_vec();
    shuffle_32(&mut out, width, rows);
    for r in 0..rows {
        let base = r * row;
        if base + row > out.len() {
            break;
        }
        for i in (1..row).rev() {
            out[base + i] = out[base + i].wrapping_sub(out[base + i - 1]);
        }
    }
    out
}
