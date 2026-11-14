//! The composite of one rectangle at one view-pyramid level (#1216).
//!
//! A level-`k` pixel `x` stands for the level-0 columns
//! `[x·2^k, (x+1)·2^k)` clipped to the document, folded one level at a time
//! by [`crate::ViewPyramid`]'s rule: premultiply, then the rounded average of
//! each 2×2 block that lies inside the level. So a level-`k` rectangle is the
//! level-0 rectangle `rect·2^k` composited and halved `k` times, with the
//! block test made against the whole level's size. The level-0 footprint's
//! origin is a multiple of `2^k` and its far edge is either one too or the
//! document's edge, so every block the halving sees is the block a whole
//! rebuild sees.

use crate::composite_region::{composite_rgba_region, stream_region};
use crate::view_pyramid::PyramidLevel;
use pictura_core::{Document, PixelBuffer, Plane, PsdRect, TILE};

/// The most level-0 pixels one streamed chunk composites.
const CHUNK_PIXELS: u64 = 8 << 20;

/// Converts a composited level-0 chunk (planar straight RGBA) to the space the
/// levels are built in; the identity keeps the working space.
pub type ChunkConvert<'a> = &'a (dyn Fn(PixelBuffer) -> PixelBuffer + Sync);

/// The size of `level` of a `width`×`height` document: each level halves,
/// rounding up.
pub fn level_size(width: u32, height: u32, level: usize) -> (u32, u32) {
    let shift = level.min(31) as u32;
    (width.div_ceil(1 << shift), height.div_ceil(1 << shift))
}

/// The composite of `rect` (level-0 pixels): straight-alpha working-space
/// RGBA, planar. Byte-identical to the same crop of [`crate::composite_rgba`];
/// a layerless document answers from its embedded composite. Anything outside
/// the document is transparent.
pub fn composite_region(doc: &Document, rect: PsdRect) -> PixelBuffer {
    let (rw, rh) = (rect.width().max(0) as u32, rect.height().max(0) as u32);
    let inner = clip(rect, doc.width, doc.height);
    if inner == rect {
        return level0(doc, inner);
    }
    let mut out = PixelBuffer::new(rw, rh, 4);
    if inner.width() > 0 && inner.height() > 0 {
        let part = level0(doc, inner);
        crate::patch_buffer_region(
            &mut out,
            &part,
            inner.left - rect.left,
            inner.top - rect.top,
        );
    }
    out
}

/// Stream the composite of `rect` (level-0 pixels, inside the document) in
/// chunks of `rows` rows, each straight-alpha planar RGBA with its row offset.
pub fn stream_composite(doc: &Document, rect: PsdRect, rows: u32, f: impl FnMut(u32, PixelBuffer)) {
    let rect = clip(rect, doc.width, doc.height);
    let span = (
        rect.left as u32,
        rect.top as u32,
        rect.width().max(0) as u32,
        rect.height().max(0) as u32,
    );
    if doc.layers.is_empty() {
        stream_layerless(doc, rect, rows, f);
    } else {
        stream_region(doc, span, rows, f);
    }
}

/// The composite of `rect` (inside the document) as one tiled plane per
/// channel, red, green, blue, alpha, each `rect`-sized. Composited in bands of
/// whole tile rows: a tile that comes out one value holds no allocation, so
/// the peak is the dense tiles and one band, never a canvas-sized buffer.
pub fn composite_channels(doc: &Document, rect: PsdRect) -> [Plane<u8>; 4] {
    let rect = clip(rect, doc.width, doc.height);
    let (w, h) = (rect.width().max(0) as usize, rect.height().max(0) as usize);
    let mut planes: [Plane<u8>; 4] = std::array::from_fn(|_| Plane::filled(w, h, 1, 0));
    if w == 0 || h == 0 {
        return planes;
    }
    let rows = band_rows(w);
    // A merge run as a job (#1278) counts composited rows.
    let job = pictura_core::job::current();
    if let Some(job) = &job {
        job.begin_stage(h as u64);
    }
    stream_composite(doc, rect, rows, |offset, band| {
        let n = band.width as usize * band.height as usize;
        let ys = offset as usize..offset as usize + band.height as usize;
        let mut samples = vec![0u8; n];
        for (c, plane) in planes.iter_mut().enumerate() {
            band.data.read_into(c * n, &mut samples);
            plane.write_rect_from(0..w, ys.clone(), w, &samples);
            plane.settle();
        }
        if let Some(job) = &job {
            job.advance(u64::from(band.height));
        }
    });
    planes
}

/// The whole composite as a tiled four-channel planar buffer, built as
/// [`composite_channels`] does: a PSD writer's merged image without a
/// canvas-sized allocation for its uniform areas.
pub fn composite_tiled(doc: &Document) -> PixelBuffer {
    let (w, h) = (doc.width as usize, doc.height as usize);
    let mut data = Plane::filled(w, h, 4, 0u8);
    if w > 0 && h > 0 {
        let whole = PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        };
        stream_composite(doc, whole, band_rows(w), |offset, band| {
            let n = band.width as usize * band.height as usize;
            let mut samples = vec![0u8; n];
            for c in 0..4 {
                band.data.read_into(c * n, &mut samples);
                let top = c * h + offset as usize;
                data.write_rect_from(0..w, top..top + band.height as usize, w, &samples);
            }
            data.settle();
        });
    }
    PixelBuffer {
        width: doc.width,
        height: doc.height,
        channels: 4,
        data,
    }
}

/// Rows per streamed band: whole tile rows holding about [`CHUNK_PIXELS`].
fn band_rows(width: usize) -> u32 {
    let tiles = (CHUNK_PIXELS as usize / (width.max(1) * TILE)).max(1);
    (tiles * TILE) as u32
}

/// Supplies straight-alpha planar RGBA for a level-0 rectangle inside the
/// document: a composite, a converted composite, or a mask's display plane.
pub type Level0Source<'a> = &'a (dyn Fn(PsdRect) -> PixelBuffer + Sync);

/// `rect` of `level` (in that level's pixels) as premultiplied interleaved
/// RGBA, byte-identical to the same crop of a [`crate::ViewPyramid`] rebuilt
/// from the whole composite after `convert`. Level 0 is the premultiplied
/// crop. Anything outside the level is transparent. The level-0 footprint is
/// composited in chunks, so the working memory is one chunk and the output.
pub fn composite_region_level(
    doc: &Document,
    rect: PsdRect,
    level: usize,
    convert: ChunkConvert<'_>,
) -> PyramidLevel {
    let source = |r: PsdRect| convert(composite_region(doc, r));
    region_level(&source, (doc.width, doc.height), rect, level)
}

/// As [`composite_region_level`] over any level-0 `source` of a
/// `size.0`×`size.1` image.
pub fn region_level(
    source: Level0Source<'_>,
    size: (u32, u32),
    rect: PsdRect,
    level: usize,
) -> PyramidLevel {
    region_level_chunked(source, size, rect, level, CHUNK_PIXELS)
}

fn region_level_chunked(
    source: Level0Source<'_>,
    size: (u32, u32),
    rect: PsdRect,
    level: usize,
    chunk_pixels: u64,
) -> PyramidLevel {
    let (rw, rh) = (rect.width().max(0) as u32, rect.height().max(0) as u32);
    let mut out = PyramidLevel::new(rw, rh);
    let (lw, lh) = level_size(size.0, size.1, level);
    let inner = clip(rect, lw, lh);
    if inner.width() <= 0 || inner.height() <= 0 {
        return out;
    }
    let footprint = PsdRect {
        top: inner.top << level,
        left: inner.left << level,
        bottom: (inner.bottom << level).min(size.1 as i32),
        right: (inner.right << level).min(size.0 as i32),
    };
    let rows = chunk_rows(footprint.width() as u64, level, chunk_pixels) as i32;
    let mut top = footprint.top;
    while top < footprint.bottom {
        let bottom = (top + rows).min(footprint.bottom);
        let chunk = source(PsdRect {
            top,
            bottom,
            ..footprint
        });
        let mut region = Region::premultiplied(&chunk, footprint.left, top);
        for step in 0..level {
            let (sw, sh) = level_size(size.0, size.1, step);
            region = region.halved(sw, sh);
        }
        region.paste_into(&mut out, rect);
        top = bottom;
    }
    out
}

/// Halve a premultiplied level-`level` region of a `width`×`height` document
/// once, for a caller that holds the four children of a tile.
pub fn halve_level(
    data: &[u8],
    origin: (i32, i32),
    size: (u32, u32),
    doc: (u32, u32),
    level: usize,
) -> (PsdRect, Vec<u8>) {
    let region = Region {
        left: origin.0,
        top: origin.1,
        width: size.0,
        height: size.1,
        data: data.to_vec(),
    };
    let (sw, sh) = level_size(doc.0, doc.1, level);
    let halved = region.halved(sw, sh);
    (halved.rect(), halved.data)
}

/// Premultiply a straight planar RGBA buffer into interleaved RGBA.
pub fn premultiply_planar(buffer: &PixelBuffer) -> Vec<u8> {
    Region::premultiplied(buffer, 0, 0).data
}

/// Level-0 rows per chunk: a multiple of `2^level` holding about
/// [`CHUNK_PIXELS`], so every chunk halves completely.
fn chunk_rows(width: u64, level: usize, chunk_pixels: u64) -> u32 {
    let step = 1u64 << level.min(31);
    let rows = (chunk_pixels / width.max(1)) / step * step;
    rows.max(step).min(i32::MAX as u64) as u32
}

fn level0(doc: &Document, rect: PsdRect) -> PixelBuffer {
    if doc.layers.is_empty() {
        let mut out = None;
        stream_layerless(doc, rect, rect.height().max(0) as u32, |_, chunk| {
            out = Some(chunk)
        });
        return out.unwrap_or_else(|| PixelBuffer::new(0, 0, 4));
    }
    composite_rgba_region(
        doc,
        rect.left as u32,
        rect.top as u32,
        rect.width().max(0) as u32,
        rect.height().max(0) as u32,
    )
}

/// A layerless document's embedded composite, cropped chunk by chunk to
/// four-channel RGBA (a missing alpha plane is opaque).
fn stream_layerless(doc: &Document, rect: PsdRect, rows: u32, mut f: impl FnMut(u32, PixelBuffer)) {
    let source = &doc.composite;
    let (w, h) = (source.width as usize, source.height as usize);
    let channels = usize::from(source.channels);
    let rw = rect.width().max(0) as usize;
    let rh = rect.height().max(0) as u32;
    let xs = rect.left as usize..rect.right as usize;
    let mut offset = 0;
    while offset < rh {
        let n = rows.max(1).min(rh - offset);
        let plane = rw * n as usize;
        let mut out = PixelBuffer::new(rw as u32, n, 4);
        let top = rect.top as usize + offset as usize;
        let data = out.data.as_mut_slice();
        for c in 0..4 {
            let dst = &mut data[c * plane..(c + 1) * plane];
            let from = match c {
                3 if channels < 4 => None,
                _ if channels == 1 => Some(0),
                _ => Some(c),
            };
            match from {
                Some(c) if source.width as usize >= xs.end && h >= top + n as usize => source
                    .data
                    .read_rect_into(xs.clone(), c * h + top..c * h + top + n as usize, w, dst),
                Some(_) => {}
                None => dst.fill(255),
            }
        }
        f(offset, out);
        offset += n;
    }
}

fn clip(rect: PsdRect, width: u32, height: u32) -> PsdRect {
    let top = rect.top.max(0);
    let left = rect.left.max(0);
    let bottom = rect.bottom.min(height as i32);
    let right = rect.right.min(width as i32);
    if right <= left || bottom <= top {
        PsdRect::default()
    } else {
        PsdRect {
            top,
            left,
            bottom,
            right,
        }
    }
}

/// Premultiplied interleaved RGBA covering a rectangle of one level.
struct Region {
    left: i32,
    top: i32,
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Region {
    fn premultiplied(buffer: &PixelBuffer, left: i32, top: i32) -> Self {
        let (w, h) = (buffer.width as usize, buffer.height as usize);
        let plane = w * h;
        use rayon::prelude::*;
        let mut data = vec![0u8; plane * 4];
        let src = &buffer.data;
        if w > 0 {
            // Rows on every core (#1218): a fit-to-screen level premultiplies
            // the whole level 0.
            data.par_chunks_mut(w * 4).enumerate().for_each_init(
                || [vec![0u8; w], vec![0u8; w], vec![0u8; w], vec![0u8; w]],
                |rows, (y, out)| {
                    for (c, row) in rows.iter_mut().enumerate() {
                        src.read_into(c * plane + y * w, row);
                    }
                    for (x, px) in out.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                        let a = rows[3][x];
                        px[0] = premul(rows[0][x], a);
                        px[1] = premul(rows[1][x], a);
                        px[2] = premul(rows[2][x], a);
                        px[3] = a;
                    }
                },
            );
        }
        Self {
            left,
            top,
            width: w as u32,
            height: h as u32,
            data,
        }
    }

    fn rect(&self) -> PsdRect {
        PsdRect {
            top: self.top,
            left: self.left,
            bottom: self.top + self.height as i32,
            right: self.left + self.width as i32,
        }
    }

    /// One level down: each pixel the rounded average of the 2×2 block above
    /// it that lies inside the `lw`×`lh` level. The origin must be even.
    fn halved(&self, lw: u32, lh: u32) -> Self {
        use rayon::prelude::*;
        debug_assert!(self.left % 2 == 0 && self.top % 2 == 0, "an aligned region");
        let (left, top) = (self.left / 2, self.top / 2);
        let width = (self.left + self.width as i32 + 1) / 2 - left;
        let height = (self.top + self.height as i32 + 1) / 2 - top;
        let (width, height) = (width.max(0) as u32, height.max(0) as u32);
        let mut data = vec![0u8; width as usize * height as usize * 4];
        let sw = self.width as usize;
        let src = &self.data;
        if width > 0 {
            data.par_chunks_mut(width as usize * 4)
                .enumerate()
                .for_each(|(y, row)| {
                    let gy = top + y as i32;
                    for (x, px) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                        let gx = left + x as i32;
                        let mut sum = [0u32; 4];
                        let mut n = 0u32;
                        for dy in 0..2 {
                            let sy = 2 * gy + dy;
                            if sy >= lh as i32 || sy >= self.top + self.height as i32 {
                                continue;
                            }
                            for dx in 0..2 {
                                let sx = 2 * gx + dx;
                                if sx >= lw as i32 || sx >= self.left + sw as i32 {
                                    continue;
                                }
                                let o =
                                    ((sy - self.top) as usize * sw + (sx - self.left) as usize) * 4;
                                for (c, slot) in sum.iter_mut().enumerate() {
                                    *slot += u32::from(src[o + c]);
                                }
                                n += 1;
                            }
                        }
                        for (c, value) in sum.iter().enumerate() {
                            if let Some(mean) = (value + n / 2).checked_div(n) {
                                px[c] = mean as u8;
                            }
                        }
                    }
                });
        }
        Self {
            left,
            top,
            width,
            height,
            data,
        }
    }

    /// Copy the part of this region inside `rect` into `out`, which covers
    /// `rect` at the same level.
    fn paste_into(&self, out: &mut PyramidLevel, rect: PsdRect) {
        let src = self.rect();
        let top = src.top.max(rect.top);
        let bottom = src.bottom.min(rect.bottom);
        let left = src.left.max(rect.left);
        let right = src.right.min(rect.right);
        if right <= left || bottom <= top {
            return;
        }
        let ow = out.width() as usize;
        let len = (right - left) as usize * 4;
        let dst = out.data_mut();
        for y in top..bottom {
            let s = ((y - src.top) as usize * self.width as usize + (left - src.left) as usize) * 4;
            let d = ((y - rect.top) as usize * ow + (left - rect.left) as usize) * 4;
            dst[d..d + len].copy_from_slice(&self.data[s..s + len]);
        }
    }
}

/// round(c · a / 255), the pyramid's premultiplication.
fn premul(c: u8, a: u8) -> u8 {
    ((u32::from(c) * u32::from(a) + 127) / 255) as u8
}

#[cfg(test)]
#[path = "tests/region_level.rs"]
pub(crate) mod tests;
