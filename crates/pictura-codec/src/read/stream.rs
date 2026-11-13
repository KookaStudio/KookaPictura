//! Layer channels decoded a scanline at a time into tiled planes (#1273), so an
//! open holds one strip of 256 rows per channel task instead of every channel
//! whole. The samples and the errors match the dense decoders in `decode.rs`,
//! which stay as the oracle and as the pattern and composite path.

use std::io::Read;

use flate2::read::{DeflateDecoder, ZlibDecoder};
use pictura_core::{Plane, Samples, TileSample, TILE};

use super::decode::{count_bytes, decode_packbits, read_rle_count};
use crate::common::*;
use crate::depth::{row_bytes, undo_prediction};
use crate::error::PsdError;

/// Rows pushed one at a time into a tiled plane, written a strip at a time
/// and settled, so uniform tiles hold no allocation.
struct TiledSink<T: TileSample> {
    plane: Plane<T>,
    width: usize,
    strip: Vec<T>,
    rows: usize,
    y0: usize,
}

impl<T: TileSample> TiledSink<T> {
    fn new(width: usize, height: usize) -> Self {
        Self {
            plane: Plane::filled(width, height, 1, T::default()),
            width,
            strip: vec![T::default(); width * TILE.min(height)],
            rows: 0,
            y0: 0,
        }
    }

    /// The next row's samples, to be filled by the caller.
    fn row(&mut self) -> &mut [T] {
        if self.rows * self.width == self.strip.len() {
            self.flush();
        }
        let at = self.rows * self.width;
        self.rows += 1;
        &mut self.strip[at..at + self.width]
    }

    fn flush(&mut self) {
        if self.rows == 0 {
            return;
        }
        let ys = self.y0..self.y0 + self.rows;
        self.plane.write_rect_from(
            0..self.width,
            ys,
            self.width,
            &self.strip[..self.width * self.rows],
        );
        self.plane.settle();
        self.y0 += self.rows;
        self.rows = 0;
    }

    fn finish(mut self) -> Plane<T> {
        self.flush();
        self.plane
    }
}

/// The bytes one streamed channel holds contiguously while it decodes: a
/// strip of each plane it builds and one encoded scanline. `None` on overflow.
pub(super) fn strip_bytes(
    width: usize,
    height: usize,
    depth: u16,
    keep_native: bool,
) -> Option<usize> {
    let per_pixel = if keep_native && depth >= 16 {
        1 + depth as usize / 8
    } else {
        1
    };
    width
        .checked_mul(TILE.min(height))?
        .checked_mul(per_pixel)?
        .checked_add(row_bytes(width, depth))
}

/// Decode one 8-, 16- or 32-bit layer channel's payload (after its
/// compression word) into a tiled 8-bit plane and, at 16/32 when
/// `keep_native`, its tiled native samples.
pub(super) fn stream_channel(
    compression: u16,
    payload: &[u8],
    width: usize,
    height: usize,
    is_psb: bool,
    depth: u16,
    keep_native: bool,
) -> Result<(Plane<u8>, Option<Samples>), PsdError> {
    let stride = row_bytes(width, depth);
    let build = |rows: &mut dyn FnMut(RowSink) -> Result<(), RowError>| {
        let mut sinks = Sinks::new(width, height, depth, keep_native);
        rows(&mut |row| sinks.push(row))?;
        Ok::<_, RowError>(sinks.finish())
    };
    let result = match compression {
        COMPRESSION_RAW => build(&mut |f| raw_rows(payload, stride, height, f)),
        COMPRESSION_RLE => build(&mut |f| rle_rows(payload, stride, height, is_psb, f)),
        COMPRESSION_ZIP | COMPRESSION_ZIP_PREDICTION => {
            stride
                .checked_mul(height)
                .ok_or_else(|| PsdError::Invalid("channel size overflow".into()))?;
            let predict = (compression == COMPRESSION_ZIP_PREDICTION).then_some((width, depth));
            // A zlib stream that fails anywhere is retried as raw deflate, as
            // the dense `inflate` does.
            match build(&mut |f| zip_rows(ZlibDecoder::new(payload), stride, height, predict, f)) {
                Err(RowError::Stream) => build(&mut |f| {
                    zip_rows(DeflateDecoder::new(payload), stride, height, predict, f)
                }),
                other => other,
            }
        }
        c => return Err(PsdError::Unsupported(format!("channel compression {c}"))),
    };
    result.map_err(|e| match e {
        RowError::Psd(e) => e,
        RowError::Stream => PsdError::Unsupported("ZIP channel data".into()),
    })
}

enum RowError {
    /// The deflate stream itself failed.
    Stream,
    Psd(PsdError),
}

impl From<PsdError> for RowError {
    fn from(e: PsdError) -> Self {
        RowError::Psd(e)
    }
}

type RowSink<'a> = &'a mut dyn FnMut(&[u8]);

fn raw_rows(payload: &[u8], stride: usize, height: usize, f: RowSink) -> Result<(), RowError> {
    let plane = stride
        .checked_mul(height)
        .ok_or_else(|| PsdError::Invalid("channel size overflow".into()))?;
    if payload.len() < plane {
        return Err(PsdError::Invalid("raw channel too short".into()).into());
    }
    payload[..plane].chunks_exact(stride).for_each(f);
    Ok(())
}

fn rle_rows(
    payload: &[u8],
    stride: usize,
    height: usize,
    is_psb: bool,
    f: RowSink,
) -> Result<(), RowError> {
    let mut pr = Reader::new(payload);
    let mut counts = Vec::with_capacity(height.min(payload.len() / count_bytes(is_psb)));
    for _ in 0..height {
        counts.push(read_rle_count(&mut pr, is_psb)?);
    }
    stride
        .checked_mul(height)
        .ok_or_else(|| PsdError::Invalid("channel size overflow".into()))?;
    let mut row = vec![0u8; stride];
    for &count in &counts {
        let packed = pr.take(count)?;
        decode_packbits(packed, &mut row)?;
        f(&row);
    }
    Ok(())
}

/// Inflate `height` rows of `stride` bytes; `predict` is `(width, depth)`
/// when each row's prediction is undone. A clean end before the last row is
/// "too short", a byte past it "too long", and any other failure is the
/// stream's.
fn zip_rows(
    mut stream: impl Read,
    stride: usize,
    height: usize,
    predict: Option<(usize, u16)>,
    f: RowSink,
) -> Result<(), RowError> {
    let mut row = vec![0u8; stride];
    for _ in 0..height {
        if let Err(e) = stream.read_exact(&mut row) {
            return Err(if e.kind() == std::io::ErrorKind::UnexpectedEof {
                PsdError::Invalid("ZIP payload too short".into()).into()
            } else {
                RowError::Stream
            });
        }
        if let Some((width, depth)) = predict {
            undo_prediction(&mut row, width, 1, depth);
        }
        f(&row);
    }
    let mut extra = [0u8; 1];
    loop {
        match stream.read(&mut extra) {
            Ok(0) => return Ok(()),
            Ok(_) => return Err(PsdError::Invalid("ZIP payload too long".into()).into()),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(RowError::Stream),
        }
    }
}

/// The planes one channel builds: the 8-bit plane, and at 16/32 the native
/// samples when they are kept. A row arrives as the file's big-endian bytes.
enum Sinks {
    Eight(TiledSink<u8>),
    Sixteen(TiledSink<u8>, Option<TiledSink<u16>>),
    ThirtyTwo(TiledSink<u8>, Option<TiledSink<f32>>),
}

impl Sinks {
    fn new(width: usize, height: usize, depth: u16, keep_native: bool) -> Self {
        let narrow = TiledSink::new(width, height);
        match depth {
            16 => Sinks::Sixteen(narrow, keep_native.then(|| TiledSink::new(width, height))),
            32 => Sinks::ThirtyTwo(narrow, keep_native.then(|| TiledSink::new(width, height))),
            _ => Sinks::Eight(narrow),
        }
    }

    // The narrowing formulas are `Samples::narrow_to_u8`'s.
    fn push(&mut self, row: &[u8]) {
        match self {
            Sinks::Eight(sink) => sink.row().copy_from_slice(row),
            Sinks::Sixteen(narrow, native) => {
                let values = row.as_chunks::<2>().0;
                for (out, b) in narrow.row().iter_mut().zip(values) {
                    *out = (u16::from_be_bytes(*b) >> 8) as u8;
                }
                if let Some(native) = native {
                    for (out, b) in native.row().iter_mut().zip(values) {
                        *out = u16::from_be_bytes(*b);
                    }
                }
            }
            Sinks::ThirtyTwo(narrow, native) => {
                let values = row.as_chunks::<4>().0;
                for (out, b) in narrow.row().iter_mut().zip(values) {
                    *out = (f32::from_be_bytes(*b) * 256.0).trunc().clamp(0.0, 255.0) as u8;
                }
                if let Some(native) = native {
                    for (out, b) in native.row().iter_mut().zip(values) {
                        *out = f32::from_be_bytes(*b);
                    }
                }
            }
        }
    }

    fn finish(self) -> (Plane<u8>, Option<Samples>) {
        match self {
            Sinks::Eight(sink) => (sink.finish(), None),
            Sinks::Sixteen(narrow, native) => {
                (narrow.finish(), native.map(|n| Samples::U16(n.finish())))
            }
            Sinks::ThirtyTwo(narrow, native) => {
                (narrow.finish(), native.map(|n| Samples::F32(n.finish())))
            }
        }
    }
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
