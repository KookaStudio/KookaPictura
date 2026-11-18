//! Classic TIFF Save As writer.

use std::collections::HashMap;
use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use pictura_core::{BitDepth, ColorMode, Compression as PsdCompression, Document};

use super::{jpeg_quality_percent, JpegEncode, Raster, SaveOptions};
use crate::error::PsdError;

type Strips = Vec<(u32, Vec<u8>)>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Order {
    Little,
    Big,
}

impl Order {
    fn u16(self, v: u16) -> [u8; 2] {
        match self {
            Order::Little => v.to_le_bytes(),
            Order::Big => v.to_be_bytes(),
        }
    }
    fn u32(self, v: u32) -> [u8; 4] {
        match self {
            Order::Little => v.to_le_bytes(),
            Order::Big => v.to_be_bytes(),
        }
    }
}

struct Entry {
    tag: u16,
    typ: u16,
    count: u32,
    data: Vec<u8>,
}

fn entry(tag: u16, typ: u16, count: u32, data: Vec<u8>) -> Entry {
    Entry {
        tag,
        typ,
        count,
        data,
    }
}

fn short_entry(tag: u16, order: Order, values: &[u16]) -> Entry {
    let data = values.iter().flat_map(|v| order.u16(*v)).collect();
    entry(tag, 3, values.len() as u32, data)
}

fn long_entry(tag: u16, order: Order, values: &[u32]) -> Entry {
    let data = values.iter().flat_map(|v| order.u32(*v)).collect();
    entry(tag, 4, values.len() as u32, data)
}

fn build_ifd(entries: &[Entry], ifd_offset: u32, order: Order) -> Vec<u8> {
    let table_len = 2 + entries.len() * 12 + 4;
    let extra_base = ifd_offset as usize + table_len;
    let mut table = Vec::with_capacity(table_len);
    let mut extra = Vec::new();
    table.extend_from_slice(&order.u16(entries.len() as u16));
    for e in entries {
        table.extend_from_slice(&order.u16(e.tag));
        table.extend_from_slice(&order.u16(e.typ));
        table.extend_from_slice(&order.u32(e.count));
        if e.data.len() <= 4 {
            let mut inline = [0u8; 4];
            inline[..e.data.len()].copy_from_slice(&e.data);
            table.extend_from_slice(&inline);
        } else {
            let offset = extra_base + extra.len();
            table.extend_from_slice(&order.u32(offset as u32));
            extra.extend_from_slice(&e.data);
            if extra.len() % 2 != 0 {
                extra.push(0);
            }
        }
    }
    table.extend_from_slice(&order.u32(0));
    table.extend_from_slice(&extra);
    table
}

fn patch_ifd_value(ifd: &mut [u8], order: Order, tag: u16, value: u32) {
    let count = match order {
        Order::Little => u16::from_le_bytes([ifd[0], ifd[1]]),
        Order::Big => u16::from_be_bytes([ifd[0], ifd[1]]),
    } as usize;
    for i in 0..count {
        let at = 2 + i * 12;
        let t = match order {
            Order::Little => u16::from_le_bytes([ifd[at], ifd[at + 1]]),
            Order::Big => u16::from_be_bytes([ifd[at], ifd[at + 1]]),
        };
        if t == tag {
            let bytes = order.u32(value);
            ifd[at + 8..at + 12].copy_from_slice(&bytes);
            return;
        }
    }
}

fn packbits(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let mut run = 1;
        while i + run < data.len() && data[i + run] == data[i] && run < 128 {
            run += 1;
        }
        if run >= 3 {
            out.push((257 - run as u16) as u8);
            out.push(data[i]);
            i += run;
        } else {
            let start = i;
            i += 1;
            while i < data.len() && i - start < 128 {
                let mut next = 1;
                while i + next < data.len() && data[i + next] == data[i] && next < 128 {
                    next += 1;
                }
                if next >= 3 {
                    break;
                }
                i += 1;
            }
            let n = i - start;
            out.push((n - 1) as u8);
            out.extend_from_slice(&data[start..i]);
        }
    }
    out
}

fn lzw(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut bit_buffer = 0u32;
    let mut bit_count = 0u32;
    let mut emit = |code: u16, width: u32, out: &mut Vec<u8>| {
        bit_buffer |= u32::from(code) << (32 - width - bit_count);
        bit_count += width;
        while bit_count >= 8 {
            out.push((bit_buffer >> 24) as u8);
            bit_buffer <<= 8;
            bit_count -= 8;
        }
    };
    let mut width = 9u32;
    let clear = 256u16;
    let end = 257u16;
    emit(clear, width, &mut out);
    let mut table: HashMap<(u16, u8), u16> = HashMap::new();
    let mut next = 258u16;
    let Some((&first, rest)) = data.split_first() else {
        emit(end, width, &mut out);
        if bit_count > 0 {
            out.push((bit_buffer >> 24) as u8);
        }
        return out;
    };
    let mut prefix = u16::from(first);
    for &b in rest {
        if let Some(&code) = table.get(&(prefix, b)) {
            prefix = code;
            continue;
        }
        emit(prefix, width, &mut out);
        if next < 4096 {
            table.insert((prefix, b), next);
            next += 1;
            if next == (1u16 << width) && width < 12 {
                width += 1;
            }
        } else {
            emit(clear, width, &mut out);
            table.clear();
            next = 258;
            width = 9;
        }
        prefix = u16::from(b);
    }
    emit(prefix, width, &mut out);
    emit(end, width, &mut out);
    if bit_count > 0 {
        out.push((bit_buffer >> 24) as u8);
    }
    out
}

fn predictor(data: &mut [u8], width: usize, height: usize, step: usize) {
    for row in 0..height {
        let start = row * width;
        for x in (step..width).rev() {
            data[start + x] = data[start + x].wrapping_sub(data[start + x - step]);
        }
    }
}

fn layer_section(doc: &Document, zip: bool) -> Result<Option<Vec<u8>>, PsdError> {
    if doc.layers.is_empty() {
        return Ok(None);
    }
    let mut clone = doc.clone();
    clone.layer_compression = if zip {
        PsdCompression::Zip
    } else {
        PsdCompression::Rle
    };
    crate::write::layer_mask_tiff_tag(&clone, None)
}

fn pixel_planes(raster: &Raster, options: &SaveOptions) -> (Vec<u8>, u16, u16, bool, bool) {
    let n = raster.width as usize * raster.height as usize;
    let mut samples: Vec<u8>;
    let (photometric, channels, indexed, gray) = match raster.mode {
        ColorMode::Grayscale | ColorMode::Bitmap | ColorMode::Duotone => {
            samples = raster.gray.clone();
            (1u16, 1usize, false, true)
        }
        ColorMode::Cmyk => {
            samples = raster.cmyk.clone();
            (5u16, 4usize, false, false)
        }
        ColorMode::Lab => {
            samples = raster.lab.clone();
            (8u16, 3usize, false, false)
        }
        ColorMode::Indexed if raster.indices.is_some() => {
            samples = raster.indices.clone().unwrap_or_default();
            (3u16, 1usize, true, false)
        }
        _ => {
            samples = raster.rgb.clone();
            (2u16, 3usize, false, false)
        }
    };
    if !options.tiff_transparency && raster.alpha {
        // Save Transparency off flattens over the mode's white. The straight
        // samples are composited, not merely stripped of the alpha plane.
        let white: Option<[u8; 4]> = match raster.mode {
            ColorMode::Cmyk => Some([0, 0, 0, 0]),
            ColorMode::Lab => Some([255, 128, 128, 0]),
            ColorMode::Rgb | ColorMode::Grayscale | ColorMode::Bitmap | ColorMode::Duotone => {
                Some([255, 255, 255, 255])
            }
            _ => None,
        };
        if let Some(white) = white {
            for i in 0..n {
                let a = u32::from(raster.rgba[i * 4 + 3]);
                for c in 0..channels {
                    let at = c * n + i;
                    samples[at] = ((u32::from(samples[at]) * a
                        + u32::from(white[c.min(3)]) * (255 - a)
                        + 127)
                        / 255) as u8;
                }
            }
        }
    }
    let alpha = options.tiff_transparency && raster.alpha;
    if alpha {
        let mut out = vec![0u8; (channels + 1) * n];
        for c in 0..channels {
            out[c * n..(c + 1) * n].copy_from_slice(&samples[c * n..(c + 1) * n]);
        }
        for i in 0..n {
            out[channels * n + i] = raster.rgba[i * 4 + 3];
        }
        samples = out;
    }
    let _ = gray;
    (
        samples,
        photometric,
        channels as u16 + u16::from(alpha),
        indexed,
        alpha,
    )
}

#[allow(clippy::too_many_arguments)]
fn encode_image(
    samples: &[u8],
    width: u32,
    height: u32,
    channels: usize,
    planar: bool,
    compression: u16,
    quality: u8,
    jpeg: Option<&JpegEncode<'_>>,
    start: u32,
) -> Result<(Strips, u32), PsdError> {
    let n = width as usize * height as usize;
    let mut strips = Vec::new();
    let mut at = start;
    if planar {
        for c in 0..channels {
            let mut plane = samples[c * n..(c + 1) * n].to_vec();
            if matches!(compression, 5 | 8) {
                let mut wide = Vec::with_capacity(plane.len());
                for row in plane.chunks_exact(width as usize) {
                    let mut row = row.to_vec();
                    predictor(&mut row, width as usize, 1, 1);
                    wide.extend_from_slice(&row);
                }
                plane = wide;
            }
            let encoded = compress(&plane, compression, width, height, 1, quality, jpeg)?;
            strips.push((at, encoded.clone()));
            at += encoded.len() as u32 + (encoded.len() as u32 & 1);
        }
    } else {
        let mut rows = Vec::with_capacity(samples.len());
        for y in 0..height as usize {
            for x in 0..width as usize {
                let i = y * width as usize + x;
                for c in 0..channels {
                    rows.push(samples[c * n + i]);
                }
            }
        }
        if matches!(compression, 5 | 8) {
            let stride = width as usize * channels;
            for row in rows.chunks_exact_mut(stride) {
                for x in (channels..stride).rev() {
                    row[x] = row[x].wrapping_sub(row[x - channels]);
                }
            }
        }
        let encoded = compress(
            &rows,
            compression,
            width,
            height,
            channels as u8,
            quality,
            jpeg,
        )?;
        strips.push((at, encoded.clone()));
        at += encoded.len() as u32 + (encoded.len() as u32 & 1);
    }
    Ok((strips, at))
}

#[allow(clippy::too_many_arguments)]
fn image_entries(
    width: u32,
    height: u32,
    channels: usize,
    photometric: u16,
    indexed: bool,
    alpha: bool,
    planar: bool,
    compression: u16,
    strips: &[(u32, Vec<u8>)],
    palette: Option<[u8; 768]>,
    order: Order,
) -> Vec<Entry> {
    // A full YCbCr JFIF stream needs photometric 6 and a subsampling tag;
    // tag 2 would tell readers the decoded samples are RGB.
    let (photometric, ycbcr) = if compression == 7 && photometric == 2 {
        (6u16, true)
    } else {
        (photometric, false)
    };
    let mut entries = Vec::new();
    entries.push(long_entry(256, order, &[width]));
    entries.push(long_entry(257, order, &[height]));
    entries.push(short_entry(258, order, &vec![8u16; channels]));
    entries.push(short_entry(259, order, &[compression]));
    entries.push(short_entry(262, order, &[photometric]));
    entries.push(long_entry(
        273,
        order,
        &strips.iter().map(|(o, _)| *o).collect::<Vec<_>>(),
    ));
    entries.push(short_entry(277, order, &[channels as u16]));
    entries.push(long_entry(278, order, &[height]));
    entries.push(long_entry(
        279,
        order,
        &strips
            .iter()
            .map(|(_, d)| d.len() as u32)
            .collect::<Vec<_>>(),
    ));
    entries.push(short_entry(284, order, &[if planar { 2 } else { 1 }]));
    if matches!(compression, 5 | 8) {
        entries.push(short_entry(317, order, &[2]));
    }
    if alpha {
        entries.push(short_entry(338, order, &[2]));
    }
    if ycbcr {
        entries.push(short_entry(530, order, &[2, 2]));
    }
    if indexed {
        if let Some(palette) = palette {
            let mut map = Vec::with_capacity(3 * 256);
            for &value in &palette[..256] {
                map.push(u16::from(value) * 257);
            }
            for &value in &palette[256..512] {
                map.push(u16::from(value) * 257);
            }
            for &value in &palette[512..] {
                map.push(u16::from(value) * 257);
            }
            entries.push(short_entry(320, order, &map));
        }
    }
    entries.sort_by_key(|entry| entry.tag);
    entries
}

fn downsample(samples: &[u8], width: u32, height: u32, channels: usize) -> (Vec<u8>, u32, u32) {
    let lw = (width / 2).max(1);
    let lh = (height / 2).max(1);
    let n = width as usize * height as usize;
    let mut out = vec![0u8; lw as usize * lh as usize * channels];
    for c in 0..channels {
        for y in 0..lh as usize {
            for x in 0..lw as usize {
                let sy = (y * 2).min(height as usize - 1);
                let sx = (x * 2).min(width as usize - 1);
                out[c * lw as usize * lh as usize + y * lw as usize + x] =
                    samples[c * n + sy * width as usize + sx];
            }
        }
    }
    (out, lw, lh)
}

/// Write a classic TIFF for `doc`, then inject the document metadata.
pub(super) fn write(
    doc: &Document,
    raster: &Raster,
    options: &SaveOptions,
    jpeg: Option<&JpegEncode<'_>>,
) -> Result<Vec<u8>, PsdError> {
    if options.tiff_compression == 4
        && ((options.tiff_transparency && raster.alpha)
            || !matches!(raster.mode, ColorMode::Rgb | ColorMode::Grayscale)
            || doc.depth != BitDepth::Eight
            || doc.source.depth().is_some()
            || raster.width > 30_000
            || raster.height > 30_000)
    {
        return Err(PsdError::Unsupported(
            "TIFF JPEG needs opaque 8-bpc RGB/gray".into(),
        ));
    }
    let layers = if options.tiff_layer_compression != 2 {
        layer_section(doc, options.tiff_layer_compression == 1)?
    } else {
        None
    };
    // PSD layer records are big-endian. A little-endian TIFF would need a
    // full layer-section byte swap; until that exists, a layered TIFF is MM
    // so its tag 37724 blocks and the container agree.
    let order = if layers.is_some() || options.tiff_byte_order_mac {
        Order::Big
    } else {
        Order::Little
    };
    let (samples, photometric, samples_per_pixel, indexed, alpha) = pixel_planes(raster, options);
    let n = raster.width as usize * raster.height as usize;
    let channels = samples_per_pixel as usize;
    if channels == 0 || samples.len() != n * channels {
        return Err(PsdError::Invalid("TIFF pixel buffer".into()));
    }
    let planar = options.tiff_pixel_order;
    let compression = match options.tiff_compression {
        0 => 1u16,
        1 => 32773,
        2 => 5,
        3 => 8,
        4 => 7,
        _ => 1,
    };
    if compression == 7 && planar {
        return Err(PsdError::Unsupported(
            "TIFF JPEG needs interleaved pixels".into(),
        ));
    }
    let (strips, mut data_at) = encode_image(
        &samples,
        raster.width,
        raster.height,
        channels,
        planar,
        compression,
        options.tiff_jpeg_quality,
        jpeg,
        8,
    )?;
    let mut level = None;
    if options.tiff_pyramid {
        let (small, lw, lh) = downsample(&samples, raster.width, raster.height, channels);
        let (level_strips, next) = encode_image(
            &small,
            lw,
            lh,
            channels,
            planar,
            compression,
            options.tiff_jpeg_quality,
            jpeg,
            data_at,
        )?;
        level = Some((small, lw, lh, level_strips));
        data_at = next;
    }
    let mut entries = image_entries(
        raster.width,
        raster.height,
        channels,
        photometric,
        indexed,
        alpha,
        planar,
        compression,
        &strips,
        raster.palette,
        order,
    );
    if let Some(layer) = layers {
        entries.push(entry(37724, 7, layer.len() as u32, layer));
    }
    if level.is_some() {
        entries.push(long_entry(330, order, &[0]));
    }
    let ifd_offset = data_at;
    let mut ifd = build_ifd(&entries, ifd_offset, order);
    let mut out = Vec::new();
    out.extend_from_slice(if order == Order::Little { b"II" } else { b"MM" });
    out.extend_from_slice(&order.u16(42));
    out.extend_from_slice(&order.u32(ifd_offset));
    for (_, d) in &strips {
        out.extend_from_slice(d);
        if d.len() % 2 != 0 {
            out.push(0);
        }
    }
    if let Some((_, _, _, level_strips)) = &level {
        for (_, d) in level_strips {
            out.extend_from_slice(d);
            if d.len() % 2 != 0 {
                out.push(0);
            }
        }
    }
    if let Some((_, lw, lh, level_strips)) = level {
        let sub_ifd_offset = ifd_offset + ifd.len() as u32;
        let sub_entries = image_entries(
            lw,
            lh,
            channels,
            photometric,
            indexed,
            alpha,
            planar,
            compression,
            &level_strips,
            raster.palette,
            order,
        );
        let sub_ifd = build_ifd(&sub_entries, sub_ifd_offset, order);
        patch_ifd_value(&mut ifd, order, 330, sub_ifd_offset);
        out.extend_from_slice(&ifd);
        out.extend_from_slice(&sub_ifd);
    } else {
        out.extend_from_slice(&ifd);
    }
    inject_metadata(doc, out, options)
}

/// A minimal 1-bit grayscale TIFF preview (uncompressed, one strip) for the
/// DOS EPS preview block. 1 is white under PhotometricInterpretation 1.
pub(super) fn preview_one_bit(width: u32, height: u32, gray: &[u8]) -> Vec<u8> {
    let row = width.div_ceil(8) as usize;
    let mut bits = vec![0u8; row * height as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            if gray.get(y * width as usize + x).copied().unwrap_or(255) >= 128 {
                bits[y * row + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    let entries = vec![
        long_entry(256, Order::Little, &[width]),
        long_entry(257, Order::Little, &[height]),
        short_entry(258, Order::Little, &[1]),
        short_entry(259, Order::Little, &[1]),
        short_entry(262, Order::Little, &[1]),
        long_entry(273, Order::Little, &[8]),
        short_entry(277, Order::Little, &[1]),
        long_entry(278, Order::Little, &[height]),
        long_entry(279, Order::Little, &[bits.len() as u32]),
    ];
    let ifd_offset = 8 + bits.len() as u32;
    let ifd = build_ifd(&entries, ifd_offset, Order::Little);
    let mut out = Vec::with_capacity(8 + bits.len() + ifd.len());
    out.extend_from_slice(b"II");
    out.extend_from_slice(&Order::Little.u16(42));
    out.extend_from_slice(&Order::Little.u32(ifd_offset));
    out.extend_from_slice(&bits);
    out.extend_from_slice(&ifd);
    out
}

fn inject_metadata(
    doc: &Document,
    bytes: Vec<u8>,
    options: &SaveOptions,
) -> Result<Vec<u8>, PsdError> {
    let mut metadata = crate::raster_metadata::raster_metadata(doc);
    if options.icc.is_some() {
        metadata.icc = options.icc.clone();
    }
    crate::raster_metadata::embed_raster_metadata(&bytes, "tif", &metadata)
}

fn compress(
    data: &[u8],
    compression: u16,
    width: u32,
    height: u32,
    components: u8,
    quality: u8,
    jpeg: Option<&JpegEncode<'_>>,
) -> Result<Vec<u8>, PsdError> {
    match compression {
        32773 => Ok(packbits(data)),
        5 => Ok(lzw(data)),
        7 => {
            let encode = jpeg
                .ok_or_else(|| PsdError::Unsupported("TIFF JPEG needs the app encoder".into()))?;
            encode(
                data,
                width,
                height,
                components,
                jpeg_quality_percent(quality),
            )
            .ok_or_else(|| PsdError::Invalid("TIFF JPEG encode".into()))
        }
        8 => {
            let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
            enc.write_all(data)
                .map_err(|_| PsdError::Invalid("TIFF deflate".into()))?;
            enc.finish()
                .map_err(|_| PsdError::Invalid("TIFF deflate".into()))
        }
        _ => Ok(data.to_vec()),
    }
}
