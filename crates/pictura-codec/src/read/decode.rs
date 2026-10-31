//! Channel and composite plane decoding: raw, PackBits RLE, and ZIP, at every
//! sample depth. Split from `read.rs` for the file-size cap.

use flate2::read::{DeflateDecoder, ZlibDecoder};
use pictura_core::Samples;
use std::io::Read;

use crate::budget::DecodeBudget;
use crate::common::*;
use crate::depth::{depth_bits, planar_len, row_bytes, undo_prediction};
use crate::error::PsdError;

/// Inflate a ZIP channel payload. The reference writes a zlib-framed stream; some
/// third-party writers emit raw deflate, so fall back to that if zlib framing
/// is absent. The decode is bounded to `expected` bytes so a crafted stream
/// cannot expand without limit; output shorter or longer than `expected` is
/// rejected with a typed error. This is a deliberate divergence from psd-tools,
/// which catches an over-long stream and substitutes a black channel with a
/// warning.
pub(crate) fn inflate(payload: &[u8], expected: usize) -> Result<Vec<u8>, PsdError> {
    // `+1` so an over-long stream is detected rather than silently truncated.
    let limit = expected.saturating_add(1) as u64;
    let mut out = Vec::new();
    if ZlibDecoder::new(payload)
        .take(limit)
        .read_to_end(&mut out)
        .is_err()
    {
        out.clear();
        if DeflateDecoder::new(payload)
            .take(limit)
            .read_to_end(&mut out)
            .is_err()
        {
            return Err(PsdError::Unsupported("ZIP channel data".into()));
        }
    }
    match out.len().cmp(&expected) {
        std::cmp::Ordering::Less => Err(PsdError::Invalid("ZIP payload too short".into())),
        std::cmp::Ordering::Greater => Err(PsdError::Invalid("ZIP payload too long".into())),
        std::cmp::Ordering::Equal => Ok(out),
    }
}

pub(super) fn read_rle(
    r: &mut Reader,
    channels: usize,
    row_bytes: usize,
    height: usize,
    is_psb: bool,
) -> Result<Vec<u8>, PsdError> {
    use rayon::prelude::*;

    let rows = channels
        .checked_mul(height)
        .ok_or_else(|| PsdError::Invalid("image dimensions overflow".into()))?;
    // Scanline byte-count table: 2-byte entries in PSD, 4-byte in PSB. Each
    // entry is read from the file, so the table never outgrows what remains.
    let mut counts = Vec::with_capacity(rows.min(r.remaining() / count_bytes(is_psb)));
    for _ in 0..rows {
        counts.push(read_rle_count(r, is_psb)?);
    }

    let mut out = vec![0u8; planar_len(channels, row_bytes, height)?];
    // The image-data section is last, so the payload is the rest of the file.
    // It is not consumed up front: a later row that runs off the end must not
    // outrank an earlier row's decode error.
    let payload = &r.data[r.pos..];
    let mut offsets = Vec::with_capacity(rows);
    let mut total = 0usize;
    for &count in &counts {
        offsets.push(total);
        total = total.saturating_add(count);
    }
    let results: Vec<Result<(), PsdError>> = out
        .par_chunks_mut(row_bytes)
        .zip(counts.par_iter().zip(offsets.par_iter()))
        .map(|(dst, (&count, &offset))| {
            let packed = payload
                .get(offset..)
                .and_then(|tail| tail.get(..count))
                .ok_or(PsdError::Truncated)?;
            decode_packbits(packed, dst)
        })
        .collect();
    for result in results {
        result?;
    }
    r.pos = r.pos.saturating_add(total);
    Ok(out)
}

/// One channel's decode work: located by the serial planning phase, decoded by
/// the parallel phase, and reassembled in layer-record order.
pub(super) enum ChannelTask {
    /// A depth-8 unmodeled channel, preserved verbatim with its header.
    Raw { offset: usize, len: usize },
    /// A modeled channel decoded at the document depth.
    Decode {
        offset: usize,
        len: usize,
        width: usize,
        height: usize,
    },
}

impl ChannelTask {
    pub(super) fn offset(&self) -> usize {
        match self {
            ChannelTask::Raw { offset, .. } | ChannelTask::Decode { offset, .. } => *offset,
        }
    }
}

/// One channel as the layer records describe it, before its task is planned.
pub(super) struct PlannedChannel {
    pub id: i16,
    pub len: usize,
    pub width: usize,
    pub height: usize,
}

/// Phase A: locate every channel and charge the decode budget in record order.
/// Returns the tasks and the total declared channel length.
pub(super) fn plan_channel_tasks(
    channels: &[PlannedChannel],
    color_channels: usize,
    depth: u16,
    budget: &mut DecodeBudget,
) -> Result<(Vec<ChannelTask>, usize), PsdError> {
    let mut tasks = Vec::with_capacity(channels.len());
    let mut offset = 0usize;
    for channel in channels {
        let is_color = channel.id >= 0 && (channel.id as usize) < color_channels;
        let modeled = channel.id == -2 || channel.id == -1 || is_color;
        let task = if modeled || depth != 8 {
            super::charge_channel(budget, channel.len, channel.width, channel.height, depth)?;
            ChannelTask::Decode {
                offset,
                len: channel.len,
                width: channel.width,
                height: channel.height,
            }
        } else {
            ChannelTask::Raw {
                offset,
                len: channel.len,
            }
        };
        tasks.push(task);
        offset = offset.saturating_add(channel.len);
    }
    Ok((tasks, offset))
}

/// Phase B: decode every channel from its own slice, preserving task order.
/// Each channel reads from `data[base + offset..]` on the whole file, never a
/// cursor another task advances and never clipped to the layer-info end, so a
/// truncated or over-long channel fails with the error the serial read gives.
pub(super) fn decode_channel_tasks(
    data: &[u8],
    base: usize,
    tasks: &[ChannelTask],
    is_psb: bool,
    depth: u16,
) -> Vec<Result<ChannelRead, PsdError>> {
    use rayon::prelude::*;

    tasks
        .par_iter()
        .map(|task| {
            let mut r = Reader::new(
                data.get(base.saturating_add(task.offset())..)
                    .unwrap_or(&[]),
            );
            match task {
                ChannelTask::Raw { len, .. } => Ok((r.take(*len)?.to_vec(), COMPRESSION_RAW, None)),
                ChannelTask::Decode {
                    len, width, height, ..
                } => read_channel_data(&mut r, *len, *width, *height, is_psb, depth),
            }
        })
        .collect()
}

/// Decode one PackBits scanline into `dst` (exactly `dst.len()` bytes).
fn decode_packbits(src: &[u8], dst: &mut [u8]) -> Result<(), PsdError> {
    let mut si = 0usize;
    let mut di = 0usize;
    while di < dst.len() {
        let control = *src
            .get(si)
            .ok_or_else(|| PsdError::Invalid("RLE underrun".into()))? as i8;
        si += 1;
        if control >= 0 {
            let count = control as usize + 1;
            if si + count > src.len() || di + count > dst.len() {
                return Err(PsdError::Invalid("RLE literal overrun".into()));
            }
            dst[di..di + count].copy_from_slice(&src[si..si + count]);
            si += count;
            di += count;
        } else if control != -128 {
            let count = (1 - control as i32) as usize; // -1 => 2, -127 => 128
            let value = *src
                .get(si)
                .ok_or_else(|| PsdError::Invalid("RLE underrun".into()))?;
            si += 1;
            if di + count > dst.len() {
                return Err(PsdError::Invalid("RLE repeat overrun".into()));
            }
            dst[di..di + count].fill(value);
            di += count;
        }
        // control == -128 is a no-op.
    }
    Ok(())
}

/// One scanline byte-count entry's width: `u32` in a PSB, `u16` otherwise.
fn count_bytes(is_psb: bool) -> usize {
    if is_psb {
        4
    } else {
        2
    }
}

fn read_rle_count(r: &mut Reader, is_psb: bool) -> Result<usize, PsdError> {
    Ok(if is_psb {
        r.u32()? as usize
    } else {
        r.u16()? as usize
    })
}

/// A decoded layer channel: 8-bit plane, compression word, optional native samples.
pub(super) type ChannelRead = (Vec<u8>, u16, Option<Samples>);

/// Read one layer channel's image data. `declared_len` includes the 2-byte
/// compression header. A depth-1 channel is bit-packed and expanded to an 8-bit
/// plane; a 16/32-bit channel is decoded at the document depth and narrowed, so
/// the layer path matches the composite. Returns the 8-bit plane, the
/// compression word, and at 16/32 the pre-narrow native plane for retention.
pub(super) fn read_channel_data(
    r: &mut Reader,
    declared_len: usize,
    width: usize,
    height: usize,
    is_psb: bool,
    depth: u16,
) -> Result<ChannelRead, PsdError> {
    if declared_len == 0 {
        return Ok((Vec::new(), COMPRESSION_RAW, None));
    }
    if declared_len < 2 {
        return Err(PsdError::Invalid(format!(
            "layer channel length {declared_len}"
        )));
    }
    let compression = r.u16()?;
    let payload = r.take(declared_len - 2)?;
    match depth {
        1 => Ok((
            decode_bitmap_channel(compression, payload, width, height, is_psb)?,
            compression,
            None,
        )),
        8 => Ok((
            decode_channel_data(compression, payload, width, height, is_psb)?,
            compression,
            None,
        )),
        _ => {
            let (data, native) =
                decode_depth_channel(compression, payload, width, height, is_psb, depth)?;
            Ok((data, compression, Some(native)))
        }
    }
}

/// Decode a 16/32-bit layer channel: it is decoded at the document depth (raw,
/// RLE, or ZIP/ZIP-with-prediction) and returned both narrowed to an 8-bit
/// `width * height` plane and as its native pre-narrow samples, mirroring the
/// composite path so layer and composite agree.
fn decode_depth_channel(
    compression: u16,
    payload: &[u8],
    width: usize,
    height: usize,
    is_psb: bool,
    depth: u16,
) -> Result<(Vec<u8>, Samples), PsdError> {
    let stride = row_bytes(width, depth);
    let plane = stride
        .checked_mul(height)
        .ok_or_else(|| PsdError::Invalid("channel size overflow".into()))?;
    let raw = match compression {
        COMPRESSION_RAW => {
            if payload.len() < plane {
                return Err(PsdError::Invalid("raw channel too short".into()));
            }
            payload[..plane].to_vec()
        }
        COMPRESSION_RLE => decode_rle_channel(payload, stride, height, is_psb)?,
        COMPRESSION_ZIP | COMPRESSION_ZIP_PREDICTION => {
            let mut data = inflate(payload, plane)?;
            if compression == COMPRESSION_ZIP_PREDICTION {
                undo_prediction(&mut data, width, height, depth);
            }
            data
        }
        c => return Err(PsdError::Unsupported(format!("channel compression {c}"))),
    };
    let samples = Samples::from_bytes(&raw, depth_bits(depth).expect("16/32-bit channel"));
    let narrowed = samples.narrow_to_u8();
    Ok((narrowed, samples))
}

/// Decode a depth-1 layer channel to an 8-bit `width * height` plane. Raw and RLE
/// rows are `ceil(width / 8)` bytes, MSB-first; ZIP is unsupported (as for the
/// composite at depth 1). The bit is expanded through the same
/// [`crate::color_mode::bitmap_rows_to_rgb`] helper the composite uses.
fn decode_bitmap_channel(
    compression: u16,
    payload: &[u8],
    width: usize,
    height: usize,
    is_psb: bool,
) -> Result<Vec<u8>, PsdError> {
    let row_bytes = row_bytes(width, 1);
    let plane = row_bytes
        .checked_mul(height)
        .ok_or_else(|| PsdError::Invalid("channel size overflow".into()))?;
    let packed = match compression {
        COMPRESSION_RAW => {
            if payload.len() < plane {
                return Err(PsdError::Invalid("raw channel too short".into()));
            }
            payload[..plane].to_vec()
        }
        COMPRESSION_RLE => decode_rle_channel(payload, row_bytes, height, is_psb)?,
        COMPRESSION_ZIP | COMPRESSION_ZIP_PREDICTION => {
            return Err(PsdError::Unsupported("ZIP compression at depth 1".into()))
        }
        c => return Err(PsdError::Unsupported(format!("channel compression {c}"))),
    };
    let rgb = crate::color_mode::bitmap_rows_to_rgb(&packed, width, height);
    Ok(rgb[..width * height].to_vec())
}

pub(crate) fn decode_rle_channel(
    payload: &[u8],
    width: usize,
    height: usize,
    is_psb: bool,
) -> Result<Vec<u8>, PsdError> {
    let mut pr = Reader::new(payload);
    let mut counts = Vec::with_capacity(height.min(payload.len() / count_bytes(is_psb)));
    for _ in 0..height {
        counts.push(read_rle_count(&mut pr, is_psb)?);
    }
    let plane = width
        .checked_mul(height)
        .ok_or_else(|| PsdError::Invalid("channel size overflow".into()))?;
    let mut out = vec![0u8; plane];
    for (row, &count) in counts.iter().enumerate() {
        let packed = pr.take(count)?;
        decode_packbits(packed, &mut out[row * width..(row + 1) * width])?;
    }
    Ok(out)
}

/// Decode one 8-bit channel plane from its compression byte and payload. Shared
/// by layer channel data (via [`read_channel_data`]) and the document Patterns
/// resource, whose `VirtualMemoryArray` channels carry the same encodings.
/// `is_psb` selects the RLE scanline-count width (u32 in a PSB, u16 otherwise);
/// pattern channel counts are always u16 (version 1).
pub(crate) fn decode_channel_data(
    compression: u16,
    payload: &[u8],
    width: usize,
    height: usize,
    is_psb: bool,
) -> Result<Vec<u8>, PsdError> {
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| PsdError::Invalid("channel size overflow".into()))?;
    match compression {
        COMPRESSION_RAW => {
            if payload.len() < pixels {
                return Err(PsdError::Invalid("raw channel too short".into()));
            }
            Ok(payload[..pixels].to_vec())
        }
        COMPRESSION_RLE => decode_rle_channel(payload, width, height, is_psb),
        COMPRESSION_ZIP | COMPRESSION_ZIP_PREDICTION => {
            let mut data = inflate(payload, pixels)?;
            if compression == COMPRESSION_ZIP_PREDICTION {
                undo_prediction(&mut data, width, height, 8);
            }
            Ok(data)
        }
        c => Err(PsdError::Unsupported(format!("channel compression {c}"))),
    }
}
