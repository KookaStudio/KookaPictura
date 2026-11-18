//! The row-based Save As writers, streamed a band at a time from
//! [`Pixels`] into the save's temp file (#1320). Each writes the bytes its
//! whole-image form wrote: lengths and checksums that precede their data are
//! patched once it is written.

use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use pictura_core::ColorMode;

use super::rows::{Pixels, Planes, BAND_ROWS};
use super::{adam7, bmp_check, bmp_header, bmp_pixels, crc32_update, over_white, SaveOptions};
use crate::error::PsdError;
use crate::write_stream::Sink;

/// Bytes of PNG scanlines handed to the deflater at once.
const PNG_FEED: usize = 1 << 20;

fn is_gray(mode: ColorMode) -> bool {
    matches!(
        mode,
        ColorMode::Grayscale | ColorMode::Bitmap | ColorMode::Duotone
    )
}

/// PNG: the scanlines deflate into one IDAT whose length is patched and whose
/// CRC runs alongside. Adam7 reads each pass's rows from the composite.
pub(super) fn write_png(
    sink: &mut Sink<'_>,
    rows: &dyn Pixels,
    options: &SaveOptions,
) -> Result<(), PsdError> {
    let (w, h) = rows.size();
    let gray = is_gray(rows.mode());
    let alpha = rows.alpha();
    let color_type = match (gray, alpha) {
        (true, false) => 0,
        (true, true) => 4,
        (false, false) => 2,
        (false, true) => 6,
    };
    let mut head = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut ihdr = [0u8; 13];
    ihdr[..4].copy_from_slice(&w.to_be_bytes());
    ihdr[4..8].copy_from_slice(&h.to_be_bytes());
    ihdr[8] = 8;
    ihdr[9] = color_type;
    ihdr[12] = u8::from(options.png_interlace);
    super::write_chunk(&mut head, b"IHDR", &ihdr);
    sink.put(&head)?;

    let len_at = sink.pos();
    sink.zeros(4)?;
    sink.put(b"IDAT")?;
    let mut crc = crc32_update(0xFFFF_FFFF, b"IDAT");
    let mut written = 0u64;
    let mut encoder = ZlibEncoder::new(
        Vec::new(),
        Compression::new(u32::from(options.png_compression.min(9))),
    );
    let mut flush = |encoder: &mut ZlibEncoder<Vec<u8>>, sink: &mut Sink<'_>| {
        let out = encoder.get_mut();
        crc = crc32_update(crc, out);
        written += out.len() as u64;
        let put = sink.put(out);
        out.clear();
        put
    };
    let mut raw = Vec::new();
    let mut rgba = Vec::new();
    let mut feed = |raw: &mut Vec<u8>,
                    encoder: &mut ZlibEncoder<Vec<u8>>,
                    sink: &mut Sink<'_>,
                    last: bool|
     -> Result<(), PsdError> {
        if raw.len() >= PNG_FEED || last {
            encoder
                .write_all(raw)
                .map_err(|_| PsdError::Invalid("PNG deflate".into()))?;
            raw.clear();
            flush(encoder, sink)?;
        }
        Ok(())
    };
    let pixel = |rgba: &[u8], x: usize, raw: &mut Vec<u8>| {
        let p = &rgba[x * 4..x * 4 + 4];
        if gray {
            raw.push(pictura_core::rec601_luma(p[0], p[1], p[2]));
        } else {
            raw.extend_from_slice(&p[..3]);
        }
        if alpha {
            raw.push(p[3]);
        }
    };
    if options.png_interlace {
        for (x0, y0, dx, dy) in adam7() {
            let pw = w.saturating_sub(x0).div_ceil(dx);
            let ph = h.saturating_sub(y0).div_ceil(dy);
            if pw == 0 || ph == 0 {
                continue;
            }
            for py in 0..ph {
                rows.rgba((y0 + py * dy) as usize, 1, &mut rgba);
                raw.push(0);
                for px in 0..pw {
                    pixel(&rgba, (x0 + px * dx) as usize, &mut raw);
                }
                feed(&mut raw, &mut encoder, sink, false)?;
            }
        }
    } else {
        let w = w as usize;
        for y0 in (0..h as usize).step_by(BAND_ROWS) {
            let band = BAND_ROWS.min(h as usize - y0);
            rows.rgba(y0, band, &mut rgba);
            for row in rgba.chunks_exact(w * 4) {
                raw.push(0);
                for x in 0..w {
                    pixel(row, x, &mut raw);
                }
            }
            feed(&mut raw, &mut encoder, sink, false)?;
        }
    }
    feed(&mut raw, &mut encoder, sink, true)?;
    encoder
        .try_finish()
        .map_err(|_| PsdError::Invalid("PNG deflate".into()))?;
    flush(&mut encoder, sink)?;
    let len = u32::try_from(written)
        .map_err(|_| PsdError::Unsupported("PNG data larger than 4 GB".into()))?;
    sink.patch(len_at, &len.to_be_bytes())?;
    sink.put(&(!crc).to_be_bytes())?;
    let mut end = Vec::new();
    super::write_chunk(&mut end, b"IEND", &[]);
    sink.put(&end)
}

/// BMP at 16, 24 or 32 bits: rows bottom-up unless Flip, read a band at a time
/// from the bottom band up; the file size is patched.
pub(super) fn write_bmp(
    sink: &mut Sink<'_>,
    rows: &dyn Pixels,
    options: &SaveOptions,
) -> Result<(), PsdError> {
    bmp_check(options)?;
    let depth = options.bmp_depth;
    let (w, h) = rows.size();
    sink.put(&bmp_header(w, h, options, &[]))?;
    let (w, h) = (w as usize, h as usize);
    let mut rgba = Vec::new();
    let mut row = Vec::new();
    let mut put_band = |sink: &mut Sink<'_>, y0: usize, band: usize, reverse: bool| {
        rows.rgba(y0, band, &mut rgba);
        let lines: Vec<&[u8]> = rgba.chunks_exact(w * 4).collect();
        let order: Box<dyn Iterator<Item = &&[u8]>> = if reverse {
            Box::new(lines.iter().rev())
        } else {
            Box::new(lines.iter())
        };
        for line in order {
            row.clear();
            bmp_pixels(line, depth, options.bmp_advanced, &mut row);
            while row.len() % 4 != 0 {
                row.push(0);
            }
            sink.put(&row)?;
        }
        Ok::<(), PsdError>(())
    };
    let starts: Vec<usize> = (0..h).step_by(BAND_ROWS).collect();
    if options.bmp_flip {
        for &y0 in &starts {
            put_band(sink, y0, BAND_ROWS.min(h - y0), false)?;
        }
    } else {
        for &y0 in starts.iter().rev() {
            put_band(sink, y0, BAND_ROWS.min(h - y0), true)?;
        }
    }
    let size = u32::try_from(sink.pos())
        .map_err(|_| PsdError::Unsupported("BMP larger than 4 GB".into()))?;
    sink.patch(2, &size.to_le_bytes())
}

/// TGA: top-down pixels; with RLE, raw packets of 128 pixels that run on
/// across rows, the partial packet carried to the next band.
pub(super) fn write_tga(
    sink: &mut Sink<'_>,
    rows: &dyn Pixels,
    options: &SaveOptions,
) -> Result<(), PsdError> {
    let depth = options.tga_depth;
    if depth != 16 && depth != 24 && depth != 32 {
        return Err(PsdError::Unsupported("TGA depth".into()));
    }
    let (w, h) = rows.size();
    let mut head = Vec::with_capacity(18);
    let image_type: u8 = if options.tga_rle { 10 } else { 2 };
    head.extend_from_slice(&[0, 0, image_type]);
    head.extend_from_slice(&[0, 0, 0, 0, 0]);
    head.extend_from_slice(&[0, 0]); // x origin
    head.extend_from_slice(&[0, 0]); // y origin
    head.extend_from_slice(&(w as u16).to_le_bytes());
    head.extend_from_slice(&(h as u16).to_le_bytes());
    head.push(depth);
    // Bit 5: top-left origin. Bits 0-3: attribute (alpha) bits per pixel.
    let attribute_bits = match depth {
        16 => 1,
        32 => 8,
        _ => 0,
    };
    head.push(0x20 | attribute_bits);
    sink.put(&head)?;
    let size = usize::from(depth / 8);
    let packet = 128 * size;
    let mut rgba = Vec::new();
    let mut pixels = Vec::new();
    let mut out = Vec::new();
    for y0 in (0..h as usize).step_by(BAND_ROWS) {
        rows.rgba(y0, BAND_ROWS.min(h as usize - y0), &mut rgba);
        for p in rgba.as_chunks::<4>().0 {
            let (r, g, b, a) = (p[0], p[1], p[2], p[3]);
            match depth {
                16 => {
                    let v = ((r as u16 >> 3) << 10)
                        | ((g as u16 >> 3) << 5)
                        | (b as u16 >> 3)
                        | (((a as u16) >> 7) << 15);
                    pixels.extend_from_slice(&[v as u8, (v >> 8) as u8]);
                }
                24 => pixels.extend_from_slice(&[
                    over_white(b, a),
                    over_white(g, a),
                    over_white(r, a),
                ]),
                _ => pixels.extend_from_slice(&[b, g, r, a]),
            }
        }
        if options.tga_rle {
            let whole = pixels.len() / packet * packet;
            for chunk in pixels[..whole].chunks_exact(packet) {
                out.push(127);
                out.extend_from_slice(chunk);
            }
            pixels.drain(..whole);
        } else {
            out.append(&mut pixels);
        }
        sink.put(&out)?;
        out.clear();
    }
    if !pixels.is_empty() {
        out.push((pixels.len() / size - 1) as u8);
        out.extend_from_slice(&pixels);
        sink.put(&out)?;
    }
    Ok(())
}

/// Photoshop Raw: the mode's planes, interleaved a band at a time, or planar
/// one plane per pass.
pub(super) fn write_raw(
    sink: &mut Sink<'_>,
    rows: &dyn Pixels,
    options: &SaveOptions,
) -> Result<(), PsdError> {
    let (kind, count) = match rows.mode() {
        ColorMode::Grayscale | ColorMode::Bitmap | ColorMode::Duotone => (Planes::Gray, 1),
        ColorMode::Cmyk => (Planes::Cmyk, 4),
        ColorMode::Lab => (Planes::Lab, 3),
        // The reader normalizes Multichannel to RGB and keeps no plate store,
        // so Raw cannot truthfully write the mode's channels.
        ColorMode::Multichannel => {
            return Err(PsdError::Unsupported(
                "Raw needs Multichannel planes".into(),
            ))
        }
        _ => (Planes::Rgb, 3),
    };
    sink.zeros(options.raw_header as usize)?;
    let h = rows.size().1 as usize;
    let mut rgba = Vec::new();
    let mut planes = vec![Vec::new(); count];
    let mut out = Vec::new();
    let passes: Vec<Option<usize>> = if options.raw_interleaved || count == 1 {
        vec![None]
    } else {
        (0..count).map(Some).collect()
    };
    for pass in passes {
        for y0 in (0..h).step_by(BAND_ROWS) {
            let band = BAND_ROWS.min(h - y0);
            rows.rgba(y0, band, &mut rgba);
            rows.planes(kind, y0, band, &rgba, &mut planes);
            match pass {
                Some(c) => sink.put(&planes[c])?,
                None => {
                    out.clear();
                    for i in 0..planes[0].len() {
                        out.extend(planes.iter().map(|p| p[i]));
                    }
                    sink.put(&out)?;
                }
            }
        }
    }
    Ok(())
}

/// Cineon: a 1024-byte header, then 10-bit RGB packed into big-endian words.
pub(super) fn write_cineon(sink: &mut Sink<'_>, rows: &dyn Pixels) -> Result<(), PsdError> {
    let (w, h) = rows.size();
    let n = w as usize * h as usize;
    sink.put(&super::cineon_header(w, h, n))?;
    let mut rgba = Vec::new();
    let mut packed = Vec::new();
    for y0 in (0..h as usize).step_by(BAND_ROWS) {
        rows.rgba(y0, BAND_ROWS.min(h as usize - y0), &mut rgba);
        packed.clear();
        for p in rgba.as_chunks::<4>().0 {
            let r = (u32::from(p[0]) * 1023) / 255;
            let g = (u32::from(p[1]) * 1023) / 255;
            let b = (u32::from(p[2]) * 1023) / 255;
            let word = (r << 22) | (g << 12) | (b << 2);
            packed.extend_from_slice(&word.to_be_bytes());
        }
        sink.put(&packed)?;
    }
    Ok(())
}
