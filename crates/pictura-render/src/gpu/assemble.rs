//! Host-side assembly of the planes a GPU composite uploads: a pixel layer's
//! source planes and its mask coverage over the composite's region.

use pictura_core::{ColorMode, Document, Layer, PsdRect};
use wgpu::WriteOnly;

use super::backend::SrcLayout;
use crate::{channel, mask_alpha, sample};

/// The region origin and dimensions a composite runs over. Pure assembly (no
/// device) so the row-wise and per-pixel paths are unit-testable without a GPU.
#[derive(Clone, Copy)]
pub(super) struct Region {
    pub(super) x0: u32,
    pub(super) y0: u32,
    pub(super) w: u32,
    pub(super) h: u32,
}

impl Region {
    /// The region in document coordinates, the `clip` for mask sampling.
    pub(super) fn rect(self) -> PsdRect {
        let at = |v: u32| v.min(i32::MAX as u32) as i32;
        PsdRect {
            left: at(self.x0),
            top: at(self.y0),
            right: at(self.x0.saturating_add(self.w)),
            bottom: at(self.y0.saturating_add(self.h)),
        }
    }
}

/// The clamped layer-rect ∩ region intersection plus the plane layout, the
/// shared product of both source-assembly paths.
pub(super) struct SourceGeom {
    pub(super) x0: i32,
    pub(super) y0: i32,
    pub(super) x1: i32,
    pub(super) y1: i32,
    pub(super) cw: usize,
    pub(super) ch: usize,
    pub(super) n: usize,
    pub(super) planes: usize,
    pub(super) gray: bool,
}

impl SourceGeom {
    pub(super) fn layout(&self) -> SrcLayout {
        SrcLayout {
            x0: self.x0 as u32,
            y0: self.y0 as u32,
            w: self.cw as u32,
            h: self.ch as u32,
            gray: self.gray,
            packed: false,
        }
    }
}

pub(super) fn source_geom(region: Region, layer: &Layer, doc: &Document) -> Option<SourceGeom> {
    if layer.rect.width() <= 0 || layer.rect.height() <= 0 {
        return None;
    }
    let region_right = (region.x0 + region.w) as i32;
    let region_bottom = (region.y0 + region.h) as i32;
    let x0 = layer.rect.left.max(region.x0 as i32);
    let y0 = layer.rect.top.max(region.y0 as i32);
    let x1 = layer.rect.right.min(region_right);
    let y1 = layer.rect.bottom.min(region_bottom);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let gray = matches!(
        doc.mode,
        ColorMode::Grayscale | ColorMode::Bitmap | ColorMode::Duotone
    );
    let cw = (x1 - x0) as usize;
    let ch = (y1 - y0) as usize;
    Some(SourceGeom {
        x0,
        y0,
        x1,
        y1,
        cw,
        ch,
        n: cw * ch,
        planes: if gray { 2 } else { 4 },
        gray,
    })
}

/// The retained per-pixel reference assembly; the fast path must match it byte
/// for byte. Kept reachable for the equivalence tests and as the short/absent
/// plane fallback.
pub(super) fn assemble_source_per_pixel(layer: &Layer, g: &SourceGeom) -> Vec<u8> {
    let lw = layer.rect.width() as usize;
    let ch0 = channel(layer, 0);
    let ch1 = channel(layer, 1).or(ch0);
    let ch2 = channel(layer, 2).or(ch0);
    let alpha = channel(layer, -1);
    let n = g.n;
    let mut data = vec![0u8; g.planes * n];
    for y in g.y0..g.y1 {
        let row = (y - g.y0) as usize;
        for (col, x) in (g.x0..g.x1).enumerate() {
            let li = (y - layer.rect.top) as usize * lw + (x - layer.rect.left) as usize;
            let d = row * g.cw + col;
            let a = sample(alpha, li).unwrap_or(255);
            if g.gray {
                data[d] = sample(ch0, li).unwrap_or(0);
                data[n + d] = a;
            } else {
                data[d] = sample(ch0, li).unwrap_or(0);
                data[n + d] = sample(ch1, li).unwrap_or(0);
                data[2 * n + d] = sample(ch2, li).unwrap_or(0);
                data[3 * n + d] = a;
            }
        }
    }
    pad_to_4(&mut data);
    data
}

/// The rows a row-wise source copies from: one slice per uploaded plane
/// (`None` is a constant-255 alpha), the first row's offset into them, and the
/// layer's row stride.
pub(super) struct RowSource<'l> {
    planes: Vec<Option<&'l [u8]>>,
    base: usize,
    lw: usize,
}

/// The row-wise source when every required channel covers the clamped row
/// intersection; `None` (the per-pixel fallback) when a present plane is short
/// or channel 0 is absent.
pub(super) fn row_source<'l>(layer: &'l Layer, g: &SourceGeom) -> Option<RowSource<'l>> {
    let lw = layer.rect.width() as usize;
    let base = (g.y0 - layer.rect.top) as usize * lw + (g.x0 - layer.rect.left) as usize;
    let last_end = base + (g.ch - 1) * lw + g.cw;
    let ch0 = channel(layer, 0)?;
    let ch1 = channel(layer, 1).unwrap_or(ch0);
    let ch2 = channel(layer, 2).unwrap_or(ch0);
    let alpha = channel(layer, -1);
    if ch0.len() < last_end || ch1.len() < last_end || ch2.len() < last_end {
        return None;
    }
    if alpha.is_some_and(|a| a.len() < last_end) {
        return None;
    }
    let planes = if g.gray {
        vec![Some(ch0), alpha]
    } else {
        vec![Some(ch0), Some(ch1), Some(ch2), alpha]
    };
    Some(RowSource { planes, base, lw })
}

/// Bytes per parallel piece of an upload: a multiple of the 4-byte copy
/// alignment, so every piece is a valid staging write of its own.
pub(super) const RUN: usize = 4 << 20;

/// The upload's byte size: `len` padded to a whole word.
pub(super) fn padded(len: usize) -> usize {
    len.next_multiple_of(4)
}

/// Fill `data` a [`RUN`] at a time in parallel, `write(start, piece)` writing
/// bytes `[start, start + piece.len())`.
#[cfg(test)]
fn fill_parallel(data: &mut [u8], write: impl Fn(usize, WriteOnly<'_, [u8]>) + Sync) {
    use rayon::prelude::*;
    data.par_chunks_mut(RUN)
        .enumerate()
        .for_each(|(i, chunk)| write(i * RUN, WriteOnly::from(chunk)));
}

/// Write bytes `[start, start + out.len())` of a source upload: the planes of
/// `src`, then zero padding.
fn write_source_range(
    mut out: WriteOnly<'_, [u8]>,
    start: usize,
    g: &SourceGeom,
    src: &RowSource<'_>,
) {
    let total = g.planes * g.n;
    let end = start + out.len();
    let mut at = start;
    while at < end {
        if at >= total {
            out.slice(at - start..).fill(0);
            return;
        }
        let (plane, within) = (at / g.n, at % g.n);
        let (row, col) = (within / g.cw, within % g.cw);
        let stop = end.min(plane * g.n + (row + 1) * g.cw);
        let mut dst = out.slice(at - start..stop - start);
        match src.planes[plane] {
            Some(p) => {
                let from = src.base + row * src.lw + col;
                dst.copy_from_slice(&p[from..from + (stop - at)]);
            }
            None => dst.fill(255),
        }
        at = stop;
    }
}

/// The writer for bytes of `layer`'s source upload over `g`: row copies when
/// the channels allow, else slices of the per-pixel reference.
pub(super) fn source_writer<'l>(
    layer: &'l Layer,
    g: &'l SourceGeom,
) -> impl Fn(usize, WriteOnly<'_, [u8]>) + Sync + 'l {
    let rows = row_source(layer, g);
    let reference = rows.is_none().then(|| assemble_source_per_pixel(layer, g));
    move |start, mut out| match (&rows, &reference) {
        (Some(src), _) => write_source_range(out, start, g, src),
        (None, Some(data)) => {
            let len = out.len();
            out.copy_from_slice(&data[start..start + len]);
        }
        (None, None) => unreachable!("the reference is built when rows are not"),
    }
}

/// The row-wise assembly as an owned, word-padded buffer; `None` as for
/// [`row_source`].
#[cfg(test)]
pub(super) fn assemble_source_rowwise(layer: &Layer, g: &SourceGeom) -> Option<Vec<u8>> {
    let src = row_source(layer, g)?;
    let mut data = vec![0u8; padded(g.planes * g.n)];
    fill_parallel(&mut data, |at, out| write_source_range(out, at, g, &src));
    Some(data)
}

#[cfg(test)]
pub(super) fn assemble_source(
    region: Region,
    layer: &Layer,
    doc: &Document,
) -> Option<(Vec<u8>, SrcLayout)> {
    let g = source_geom(region, layer, doc)?;
    let mut data = vec![0u8; padded(g.planes * g.n)];
    fill_parallel(&mut data, source_writer(layer, &g));
    Some((data, g.layout()))
}

/// Whether `mask_alpha` has data to sample: absent, disabled, or data-less
/// masks (including an empty/all-open vector mask) are the constant-255 case
/// the row fill covers.
pub(super) fn mask_has_data(layer: &Layer) -> bool {
    layer
        .mask
        .as_ref()
        .is_some_and(|m| !m.disabled && m.data.is_some())
        || layer.vector_mask.as_ref().is_some_and(|v| v.has_fill())
}

/// The coverage influence rectangle: the whole region for a group or an
/// adjustment layer, the clamped layer rect for a pixel layer.
pub(super) fn mask_influence_rect(region: Region, layer: &Layer) -> (i32, i32, i32, i32) {
    let region_right = (region.x0 + region.w) as i32;
    let region_bottom = (region.y0 + region.h) as i32;
    if layer.is_group || layer.adjustment.is_some() {
        (
            region.x0 as i32,
            region.y0 as i32,
            region_right,
            region_bottom,
        )
    } else {
        (
            layer.rect.left.max(region.x0 as i32),
            layer.rect.top.max(region.y0 as i32),
            layer.rect.right.min(region_right),
            layer.rect.bottom.min(region_bottom),
        )
    }
}

/// Constant-255 coverage over the influence rect, 0 elsewhere.
#[cfg(test)]
pub(super) fn assemble_mask_fill(region: Region, r: (i32, i32, i32, i32)) -> Vec<u8> {
    let (x0, y0, x1, y1) = r;
    let mut data = vec![0u8; region.w as usize * region.h as usize];
    if x1 > x0 && y1 > y0 {
        let stride = region.w as usize;
        let left = (x0 - region.x0 as i32) as usize;
        let right = (x1 - region.x0 as i32) as usize;
        for y in y0..y1 {
            let row = (y - region.y0 as i32) as usize * stride;
            data[row + left..row + right].fill(255);
        }
    }
    pad_to_4(&mut data);
    data
}

/// The retained per-pixel `mask_alpha` reference, kept for data-carrying masks
/// and the equivalence tests.
#[cfg(test)]
pub(super) fn assemble_mask_per_pixel(
    region: Region,
    layer: &Layer,
    r: (i32, i32, i32, i32),
) -> Vec<u8> {
    let (x0, y0, x1, y1) = r;
    let mut data = vec![0u8; region.w as usize * region.h as usize];
    if x1 > x0 && y1 > y0 {
        let stride = region.w as usize;
        for y in y0..y1 {
            let row = (y - region.y0 as i32) as usize * stride;
            for x in x0..x1 {
                data[row + (x - region.x0 as i32) as usize] =
                    mask_alpha(layer, region.rect(), x, y);
            }
        }
    }
    pad_to_4(&mut data);
    data
}

/// The writer for bytes of `layer`'s coverage upload over `region` (a `w × h`
/// plane, then zero padding): 255 over the influence rect for a data-less
/// mask, `mask_alpha` there for one with data, 0 elsewhere.
pub(super) fn mask_writer(
    region: Region,
    layer: &Layer,
) -> impl Fn(usize, WriteOnly<'_, [u8]>) + Sync + '_ {
    let (x0, y0, x1, y1) = mask_influence_rect(region, layer);
    let stride = region.w as usize;
    let total = stride * region.h as usize;
    let sampled = mask_has_data(layer);
    move |start, mut out| {
        let end = start + out.len();
        let mut row_bytes = vec![0u8; stride];
        let mut at = start;
        while at < end {
            if at >= total {
                out.slice(at - start..).fill(0);
                return;
            }
            let (row, col) = (at / stride, at % stride);
            let y = region.y0 as i32 + row as i32;
            row_bytes.fill(0);
            if y >= y0 && y < y1 && x1 > x0 {
                let left = (x0 - region.x0 as i32) as usize;
                let span = &mut row_bytes[left..left + (x1 - x0) as usize];
                if sampled {
                    for (x, v) in (x0..x1).zip(span.iter_mut()) {
                        *v = mask_alpha(layer, region.rect(), x, y);
                    }
                } else {
                    span.fill(255);
                }
            }
            let stop = end.min((row + 1) * stride);
            out.slice(at - start..stop - start)
                .copy_from_slice(&row_bytes[col..col + (stop - at)]);
            at = stop;
        }
    }
}

#[cfg(test)]
pub(super) fn assemble_mask(region: Region, layer: &Layer) -> Vec<u8> {
    let mut data = vec![0u8; padded(region.w as usize * region.h as usize)];
    fill_parallel(&mut data, mask_writer(region, layer));
    data
}

/// A byte-packed `array<u32>` binding's size and every upload must be a
/// multiple of 4; pad the tail rather than relying on the caller.
fn pad_to_4(data: &mut Vec<u8>) {
    while !data.len().is_multiple_of(4) {
        data.push(0);
    }
}
