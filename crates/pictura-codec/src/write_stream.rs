//! The seekable sink the PSD and raster writers stream into, and the band
//! encoders that fill it (#1320).
//!
//! A writer emits lengths, offsets and count tables before the data they
//! describe as placeholders and patches them once that data is written, so it
//! holds one band of rows at a time instead of the encoded file. The in-memory
//! encoders run the same code over a `Cursor<Vec<u8>>`, so they produce the
//! same bytes.

use std::io::{Cursor, Seek, SeekFrom, Write};

use rayon::prelude::*;

use crate::depth::apply_prediction;
use crate::error::PsdError;
use crate::write::{encode_packbits_row, ScanRows};
use pictura_core::Compression;

/// Bytes of rows a band encoder reads at once, and its row cap.
const BAND_BYTES: usize = 8 << 20;
pub(crate) const ENCODE_ROWS: usize = 256;

/// Rows per band for `row_bytes`-wide rows.
pub(crate) fn band_rows(row_bytes: usize) -> usize {
    (BAND_BYTES / row_bytes.max(1)).clamp(1, ENCODE_ROWS)
}

pub(crate) fn io_error(e: std::io::Error) -> PsdError {
    PsdError::Invalid(format!("write failed: {e}"))
}

/// A byte sink with a position, back-patching, and an optional capture of the
/// bytes written since a mark (for the encode cache).
pub(crate) struct Sink<'w> {
    inner: &'w mut dyn WriteSeek,
    start: u64,
    pos: u64,
    capture: Option<Capture>,
}

pub(crate) trait WriteSeek: Write + Seek {}
impl<T: Write + Seek + ?Sized> WriteSeek for T {}

struct Capture {
    at: u64,
    limit: usize,
    bytes: Option<Vec<u8>>,
}

impl<'w> Sink<'w> {
    pub(crate) fn new(inner: &'w mut dyn WriteSeek) -> Result<Self, PsdError> {
        let start = inner.stream_position().map_err(io_error)?;
        Ok(Self {
            inner,
            start,
            pos: start,
            capture: None,
        })
    }

    /// Bytes written since the sink was opened.
    pub(crate) fn pos(&self) -> u64 {
        self.pos - self.start
    }

    pub(crate) fn put(&mut self, bytes: &[u8]) -> Result<(), PsdError> {
        self.inner.write_all(bytes).map_err(io_error)?;
        self.pos += bytes.len() as u64;
        if let Some(capture) = &mut self.capture {
            if let Some(buf) = &mut capture.bytes {
                if buf.len() + bytes.len() > capture.limit {
                    capture.bytes = None;
                } else {
                    buf.extend_from_slice(bytes);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn zeros(&mut self, n: usize) -> Result<(), PsdError> {
        const ZEROS: [u8; 4096] = [0; 4096];
        let mut left = n;
        while left > 0 {
            let k = left.min(ZEROS.len());
            self.put(&ZEROS[..k])?;
            left -= k;
        }
        Ok(())
    }

    /// Overwrite the bytes at `at` (a [`Self::pos`] value) and return to the end.
    pub(crate) fn patch(&mut self, at: u64, bytes: &[u8]) -> Result<(), PsdError> {
        let end = self.pos;
        self.inner
            .seek(SeekFrom::Start(self.start + at))
            .map_err(io_error)?;
        self.inner.write_all(bytes).map_err(io_error)?;
        self.inner.seek(SeekFrom::Start(end)).map_err(io_error)?;
        if let Some(capture) = &mut self.capture {
            let at = self.start + at;
            if let Some(buf) = &mut capture.bytes {
                if at >= capture.at {
                    let i = (at - capture.at) as usize;
                    buf[i..i + bytes.len()].copy_from_slice(bytes);
                }
            }
        }
        Ok(())
    }

    /// Patch a big-endian length: `u64` when `wide`, else `u32`.
    pub(crate) fn patch_len(&mut self, at: u64, value: u64, wide: bool) -> Result<(), PsdError> {
        if wide {
            self.patch(at, &value.to_be_bytes())
        } else {
            let value = u32::try_from(value)
                .map_err(|_| PsdError::Invalid("section exceeds u32 length".into()))?;
            self.patch(at, &value.to_be_bytes())
        }
    }

    /// Reserve a big-endian length field, returning its offset.
    pub(crate) fn len_placeholder(&mut self, wide: bool) -> Result<u64, PsdError> {
        let at = self.pos();
        self.zeros(if wide { 8 } else { 4 })?;
        Ok(at)
    }

    /// Start keeping a copy of what is written, up to `limit` bytes.
    pub(crate) fn begin_capture(&mut self, limit: usize) {
        self.capture = Some(Capture {
            at: self.pos,
            limit,
            bytes: Some(Vec::new()),
        });
    }

    /// The bytes written since [`Self::begin_capture`], or `None` when they
    /// went over its limit.
    pub(crate) fn end_capture(&mut self) -> Option<Vec<u8>> {
        self.capture.take().and_then(|c| c.bytes)
    }
}

impl Write for Sink<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.put(buf)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Run `f` over a sink backed by a fresh buffer and return the buffer.
pub(crate) fn to_vec(
    f: impl FnOnce(&mut Sink<'_>) -> Result<(), PsdError>,
) -> Result<Vec<u8>, PsdError> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut sink = Sink::new(&mut cursor)?;
        f(&mut sink)?;
    }
    Ok(cursor.into_inner())
}

/// Rows `y0 .. y0 + rows` of `plane`, `row_bytes` each, into `out`.
fn read_band<P: ScanRows>(plane: &P, y0: usize, rows: usize, row_bytes: usize, out: &mut Vec<u8>) {
    out.resize(rows * row_bytes, 0);
    plane.read_rows(y0, rows, row_bytes, out);
}

/// Write the payload of `planes` (each `row_bytes * height`, plane-major) as
/// `kind`: no compression word. RLE writes every plane's scanline counts
/// (2 bytes PSD, 4 PSB) before the packed rows; the counts are patched once
/// the rows are written. ZIP deflates the planes as one stream, predicted per
/// row for [`Compression::ZipPrediction`].
#[allow(clippy::too_many_arguments)]
pub(crate) fn write_payload<P: ScanRows + Sync>(
    sink: &mut Sink<'_>,
    kind: Compression,
    planes: &[P],
    width: usize,
    row_bytes: usize,
    height: usize,
    depth: u16,
    psb: bool,
) -> Result<(), PsdError> {
    let plane_len = row_bytes
        .checked_mul(height)
        .ok_or_else(|| PsdError::Invalid("plane size overflow".into()))?;
    if planes.iter().any(|p| p.byte_len() != plane_len) {
        return Err(PsdError::Invalid("plane length mismatch".into()));
    }
    let rows_per_band = band_rows(row_bytes);
    let mut band = Vec::new();
    match kind {
        Compression::Raw => {
            for plane in planes {
                for y0 in (0..height).step_by(rows_per_band) {
                    let rows = rows_per_band.min(height - y0);
                    read_band(plane, y0, rows, row_bytes, &mut band);
                    sink.put(&band)?;
                }
            }
        }
        Compression::Rle => {
            let count_width = if psb { 4 } else { 2 };
            let counts_at = sink.pos();
            let mut counts = Vec::with_capacity(planes.len() * height * count_width);
            sink.zeros(planes.len() * height * count_width)?;
            for plane in planes {
                for y0 in (0..height).step_by(rows_per_band) {
                    let rows = rows_per_band.min(height - y0);
                    read_band(plane, y0, rows, row_bytes, &mut band);
                    let packed: Vec<Vec<u8>> = band
                        .par_chunks(row_bytes.max(1))
                        .with_min_len(16)
                        .map(|row| {
                            let mut out = Vec::with_capacity(row.len() + row.len() / 128 + 1);
                            encode_packbits_row(row, &mut out);
                            out
                        })
                        .collect();
                    for row in &packed {
                        // The PSD maximum width (30 000) cannot reach the u16
                        // limit, so the guard only fires for a PSB row
                        // approaching u32; it errs rather than truncate.
                        if psb {
                            let n = u32::try_from(row.len()).map_err(|_| {
                                PsdError::Invalid("RLE scanline exceeds u32 byte count".into())
                            })?;
                            counts.extend_from_slice(&n.to_be_bytes());
                        } else {
                            let n = u16::try_from(row.len()).map_err(|_| {
                                PsdError::Invalid("RLE scanline exceeds u16 byte count".into())
                            })?;
                            counts.extend_from_slice(&n.to_be_bytes());
                        }
                        sink.put(row)?;
                    }
                }
            }
            sink.patch(counts_at, &counts)?;
        }
        Compression::Zip | Compression::ZipPrediction => {
            use flate2::write::ZlibEncoder;
            let predict = kind == Compression::ZipPrediction;
            let mut enc = ZlibEncoder::new(&mut *sink, flate2::Compression::default());
            for plane in planes {
                for y0 in (0..height).step_by(rows_per_band) {
                    let rows = rows_per_band.min(height - y0);
                    read_band(plane, y0, rows, row_bytes, &mut band);
                    if predict {
                        apply_prediction(&mut band, width, rows, depth);
                    }
                    enc.write_all(&band)
                        .map_err(|_| PsdError::Invalid("ZIP encode".into()))?;
                }
            }
            enc.finish()
                .map_err(|_| PsdError::Invalid("ZIP encode".into()))?;
        }
    }
    Ok(())
}
