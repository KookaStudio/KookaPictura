//! Classic TIFF Save As writer, streamed a band of rows at a time (#1320).

use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use pictura_core::{BitDepth, ColorMode, Compression as PsdCompression, Document};

use super::rows::{Pixels, Planes, BAND_ROWS};
use super::tiff_codec::{Lzw, PackBits};
use super::{jpeg_quality_percent, JpegEncode, Raster, SaveOptions};
use crate::error::PsdError;
use crate::write_stream::Sink;

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

/// The one-shot PackBits the streaming encoder must match.
#[cfg(test)]
pub(super) fn packbits(data: &[u8]) -> Vec<u8> {
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

/// The one-shot LZW the streaming encoder must match.
#[cfg(test)]
pub(super) fn lzw(data: &[u8]) -> Vec<u8> {
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
    let mut table: std::collections::HashMap<(u16, u8), u16> = std::collections::HashMap::new();
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

/// The TIFF photometric interpretation, sample planes and Indexed flag for
/// `px`'s mode.
fn layout(px: &dyn Pixels) -> (u16, Planes, usize, bool) {
    match px.mode() {
        ColorMode::Grayscale | ColorMode::Bitmap | ColorMode::Duotone => {
            (1, Planes::Gray, 1, false)
        }
        ColorMode::Cmyk => (5, Planes::Cmyk, 4, false),
        ColorMode::Lab => (8, Planes::Lab, 3, false),
        ColorMode::Indexed if px.has_indices() => (3, Planes::Indices, 1, true),
        _ => (2, Planes::Rgb, 3, false),
    }
}

/// The output planes of one band: the mode's planes, flattened over the mode's
/// white when Save Transparency is off, plus the alpha plane when it is on.
struct BandSampler<'p> {
    px: &'p dyn Pixels,
    kind: Planes,
    color: usize,
    white: Option<[u8; 4]>,
    alpha: bool,
}

impl BandSampler<'_> {
    fn sample(&self, y0: usize, rows: usize, out: &mut [Vec<u8>]) {
        let mut rgba = Vec::new();
        self.px.rgba(y0, rows, &mut rgba);
        self.px.planes(self.kind, y0, rows, &rgba, out);
        if let Some(white) = self.white {
            for (c, plane) in out.iter_mut().take(self.color).enumerate() {
                let w = u32::from(white[c.min(3)]);
                for (v, p) in plane.iter_mut().zip(rgba.as_chunks::<4>().0) {
                    let a = u32::from(p[3]);
                    *v = ((u32::from(*v) * a + w * (255 - a) + 127) / 255) as u8;
                }
            }
        }
        if self.alpha {
            let plane = &mut out[self.color];
            plane.clear();
            plane.extend(rgba.as_chunks::<4>().0.iter().map(|p| p[3]));
        }
    }
}

/// Fills one band's output planes: rows `y0 .. y0 + rows`.
type BandFill<'a> = dyn FnMut(usize, usize, &mut [Vec<u8>]) + 'a;

/// One strip's compressor, fed a band at a time.
enum StripEncoder {
    None,
    PackBits(PackBits),
    Lzw(Lzw),
    Zip(ZlibEncoder<Vec<u8>>),
    /// ponytail: Qt's JPEG encoder takes the whole strip at once.
    Jpeg(Vec<u8>),
}

impl StripEncoder {
    fn new(compression: u16, out: &mut Vec<u8>) -> Self {
        match compression {
            32773 => Self::PackBits(PackBits::default()),
            5 => Self::Lzw(Lzw::new(out)),
            8 => Self::Zip(ZlibEncoder::new(Vec::new(), Compression::default())),
            7 => Self::Jpeg(Vec::new()),
            _ => Self::None,
        }
    }

    fn push(&mut self, data: &[u8], out: &mut Vec<u8>) -> Result<(), PsdError> {
        match self {
            Self::None => out.extend_from_slice(data),
            Self::PackBits(p) => p.push(data, out),
            Self::Lzw(l) => l.push(data, out),
            Self::Zip(z) => {
                z.write_all(data)
                    .map_err(|_| PsdError::Invalid("TIFF deflate".into()))?;
                out.append(z.get_mut());
            }
            Self::Jpeg(all) => all.extend_from_slice(data),
        }
        Ok(())
    }

    fn finish(
        self,
        out: &mut Vec<u8>,
        size: (u32, u32, u8),
        quality: u8,
        jpeg: Option<&JpegEncode<'_>>,
    ) -> Result<(), PsdError> {
        match self {
            Self::None => {}
            Self::PackBits(p) => p.finish(out),
            Self::Lzw(l) => l.finish(out),
            Self::Zip(z) => out.extend(
                z.finish()
                    .map_err(|_| PsdError::Invalid("TIFF deflate".into()))?,
            ),
            Self::Jpeg(all) => {
                let encode = jpeg.ok_or_else(|| {
                    PsdError::Unsupported("TIFF JPEG needs the app encoder".into())
                })?;
                out.extend(
                    encode(&all, size.0, size.1, size.2, jpeg_quality_percent(quality))
                        .ok_or_else(|| PsdError::Invalid("TIFF JPEG encode".into()))?,
                );
            }
        }
        Ok(())
    }
}

/// The parameters every strip of one image shares.
struct StripPlan<'a> {
    width: usize,
    height: usize,
    channels: usize,
    planar: bool,
    compression: u16,
    quality: u8,
    jpeg: Option<&'a JpegEncode<'a>>,
}

/// Encode an image's strips into the sink: one per channel when `planar`,
/// else one interleaved strip, each padded to an even length. Returns each
/// strip's offset and length. `sample` fills one band's output planes.
fn write_strips(
    sink: &mut Sink<'_>,
    plan: &StripPlan<'_>,
    sample: &mut BandFill<'_>,
) -> Result<Vec<(u32, u32)>, PsdError> {
    let (w, h, channels) = (plan.width, plan.height, plan.channels);
    let predict = matches!(plan.compression, 5 | 8);
    let mut planes = vec![Vec::new(); channels];
    let mut strips = Vec::new();
    let mut out = Vec::new();
    let strip_count = if plan.planar { channels } else { 1 };
    for c in 0..strip_count {
        let start = sink.pos();
        let mut encoder = StripEncoder::new(plan.compression, &mut out);
        let mut data = Vec::new();
        for y0 in (0..h).step_by(BAND_ROWS) {
            let rows = BAND_ROWS.min(h - y0);
            sample(y0, rows, &mut planes);
            if plan.planar {
                data.clear();
                data.extend_from_slice(&planes[c]);
                if predict {
                    for row in data.chunks_exact_mut(w) {
                        for x in (1..w).rev() {
                            row[x] = row[x].wrapping_sub(row[x - 1]);
                        }
                    }
                }
            } else {
                data.clear();
                data.reserve(w * rows * channels);
                for i in 0..w * rows {
                    data.extend(planes.iter().map(|p| p[i]));
                }
                if predict {
                    let stride = w * channels;
                    for row in data.chunks_exact_mut(stride) {
                        for x in (channels..stride).rev() {
                            row[x] = row[x].wrapping_sub(row[x - channels]);
                        }
                    }
                }
            }
            encoder.push(&data, &mut out)?;
            sink.put(&out)?;
            out.clear();
        }
        let components = if plan.planar { 1 } else { channels as u8 };
        encoder.finish(
            &mut out,
            (w as u32, h as u32, components),
            plan.quality,
            plan.jpeg,
        )?;
        sink.put(&out)?;
        out.clear();
        let len = sink.pos() - start;
        if !len.is_multiple_of(2) {
            sink.put(&[0])?;
        }
        let offset = u32::try_from(start)
            .map_err(|_| PsdError::Unsupported("TIFF larger than 4 GB".into()))?;
        let len = u32::try_from(len)
            .map_err(|_| PsdError::Unsupported("TIFF strip larger than 4 GB".into()))?;
        strips.push((offset, len));
    }
    Ok(strips)
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
    strips: &[(u32, u32)],
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
        &strips.iter().map(|(_, len)| *len).collect::<Vec<_>>(),
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

/// The layer section's tag, whose data streams after the rest of the IFD.
const TAG_LAYERS: u16 = 37724;
const TAG_SUB_IFDS: u16 = 330;

/// Write a classic TIFF for `doc`, as [`write_to`], into a buffer.
pub(super) fn write(
    doc: &Document,
    raster: &Raster,
    options: &SaveOptions,
    jpeg: Option<&JpegEncode<'_>>,
) -> Result<Vec<u8>, PsdError> {
    crate::write_stream::to_vec(|sink| write_to(sink, doc, raster, options, jpeg))
}

/// Write a classic TIFF for `doc` into the sink, its strips a band at a time,
/// then its layer section and metadata (#1320).
pub(super) fn write_to(
    sink: &mut Sink<'_>,
    doc: &Document,
    px: &dyn Pixels,
    options: &SaveOptions,
    jpeg: Option<&JpegEncode<'_>>,
) -> Result<(), PsdError> {
    let (width, height) = px.size();
    if options.tiff_compression == 4
        && ((options.tiff_transparency && px.alpha())
            || !matches!(px.mode(), ColorMode::Rgb | ColorMode::Grayscale)
            || doc.depth != BitDepth::Eight
            || doc.source.depth().is_some()
            || width > 30_000
            || height > 30_000)
    {
        return Err(PsdError::Unsupported(
            "TIFF JPEG needs opaque 8-bpc RGB/gray".into(),
        ));
    }
    let layered = options.tiff_layer_compression != 2 && !doc.layers.is_empty();
    // PSD layer records are big-endian. A little-endian TIFF would need a
    // full layer-section byte swap; until that exists, a layered TIFF is MM
    // so its tag 37724 blocks and the container agree.
    let order = if layered || options.tiff_byte_order_mac {
        Order::Big
    } else {
        Order::Little
    };
    let (photometric, kind, color, indexed) = layout(px);
    // Save Transparency off flattens over the mode's white. The straight
    // samples are composited, not merely stripped of the alpha plane.
    let white = (!options.tiff_transparency && px.alpha())
        .then(|| match px.mode() {
            ColorMode::Cmyk => Some([0, 0, 0, 0]),
            ColorMode::Lab => Some([255, 128, 128, 0]),
            ColorMode::Rgb | ColorMode::Grayscale | ColorMode::Bitmap | ColorMode::Duotone => {
                Some([255, 255, 255, 255])
            }
            _ => None,
        })
        .flatten();
    let alpha = options.tiff_transparency && px.alpha();
    let channels = color + usize::from(alpha);
    let sampler = BandSampler {
        px,
        kind,
        color,
        white,
        alpha,
    };
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
    sink.put(if order == Order::Little { b"II" } else { b"MM" })?;
    sink.put(&order.u16(42))?;
    let header_ifd = sink.pos();
    sink.zeros(4)?;
    let (w, h) = (width as usize, height as usize);
    let mut plan = StripPlan {
        width: w,
        height: h,
        channels,
        planar,
        compression,
        quality: options.tiff_jpeg_quality,
        jpeg,
    };
    let strips = write_strips(sink, &plan, &mut |y0, rows, out| {
        sampler.sample(y0, rows, out)
    })?;
    let mut level = None;
    if options.tiff_pyramid {
        let (lw, lh) = ((width / 2).max(1), (height / 2).max(1));
        plan.width = lw as usize;
        plan.height = lh as usize;
        let mut full = vec![Vec::new(); channels];
        let level_strips = write_strips(sink, &plan, &mut |y0, rows, out| {
            for plane in out.iter_mut() {
                plane.clear();
            }
            for y in y0..y0 + rows {
                sampler.sample((y * 2).min(h - 1), 1, &mut full);
                for (plane, row) in out.iter_mut().zip(&full) {
                    plane.extend((0..lw as usize).map(|x| row[(x * 2).min(w - 1)]));
                }
            }
        })?;
        level = Some((lw, lh, level_strips));
    }
    let mut entries = image_entries(
        width,
        height,
        channels,
        photometric,
        indexed,
        alpha,
        planar,
        compression,
        &strips,
        px.palette(),
        order,
    );
    if layered {
        // The data streams in after the table; its count is patched.
        entries.push(entry(TAG_LAYERS, 7, 0, vec![0; 5]));
    }
    if level.is_some() {
        entries.push(long_entry(TAG_SUB_IFDS, order, &[0]));
    }
    let ifd_offset = sink.pos();
    let ifd_at = u32::try_from(ifd_offset)
        .map_err(|_| PsdError::Unsupported("TIFF larger than 4 GB".into()))?;
    let mut ifd = build_ifd(&entries, ifd_at, order);
    if layered {
        // The layer entry sorts last, so its data ends the IFD block.
        ifd.truncate(ifd.len() - 6);
        sink.put(&ifd)?;
        let start = sink.pos();
        let mut clone = doc.clone();
        clone.layer_compression = if options.tiff_layer_compression == 1 {
            PsdCompression::Zip
        } else {
            PsdCompression::Rle
        };
        crate::write::write_layer_mask_tiff_tag(sink, &clone, None)?;
        let len = sink.pos() - start;
        if !len.is_multiple_of(2) {
            sink.put(&[0])?;
        }
        let count = u32::try_from(len)
            .map_err(|_| PsdError::Unsupported("TIFF layer section larger than 4 GB".into()))?;
        let at = ifd_entry_at(&ifd, order, TAG_LAYERS) + 4;
        ifd[at..at + 4].copy_from_slice(&order.u32(count));
        sink.patch(ifd_offset + at as u64, &order.u32(count))?;
    } else {
        sink.put(&ifd)?;
    }
    if let Some((lw, lh, level_strips)) = level {
        let sub_ifd_offset = u32::try_from(sink.pos())
            .map_err(|_| PsdError::Unsupported("TIFF larger than 4 GB".into()))?;
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
            px.palette(),
            order,
        );
        let sub_ifd = build_ifd(&sub_entries, sub_ifd_offset, order);
        let at = ifd_entry_at(&ifd, order, TAG_SUB_IFDS) + 8;
        ifd[at..at + 4].copy_from_slice(&order.u32(sub_ifd_offset));
        sink.patch(ifd_offset + at as u64, &order.u32(sub_ifd_offset))?;
        sink.put(&sub_ifd)?;
    }
    sink.patch(header_ifd, &order.u32(ifd_at))?;
    let mut metadata = crate::raster_metadata::raster_metadata(doc);
    if options.icc.is_some() {
        metadata.icc = options.icc.clone();
    }
    if let Some((tail, ifd0)) =
        crate::raster_metadata::embed_tiff_tail(sink.pos(), order == Order::Big, &ifd, &metadata)?
    {
        sink.put(&tail)?;
        sink.patch(header_ifd, &order.u32(ifd0))?;
    }
    Ok(())
}

/// The byte offset of `tag`'s entry inside an IFD table.
fn ifd_entry_at(ifd: &[u8], order: Order, tag: u16) -> usize {
    let read = |at: usize| match order {
        Order::Little => u16::from_le_bytes([ifd[at], ifd[at + 1]]),
        Order::Big => u16::from_be_bytes([ifd[at], ifd[at + 1]]),
    };
    let count = read(0) as usize;
    (0..count)
        .map(|i| 2 + i * 12)
        .find(|&at| read(at) == tag)
        .expect("the entry was written")
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
