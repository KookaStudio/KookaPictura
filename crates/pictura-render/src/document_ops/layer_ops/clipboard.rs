//! The Edit clipboard: Copy, Copy Merged, Clear, and Paste / Paste Into / Paste
//! Outside. A copy keeps only the selection's bounding box — straight RGBA plus
//! the selection coverage over it — so a small copy stays small. The selection
//! arrives as a document-sized coverage plane (`width * height` bytes, 255 =
//! fully selected), read by rows so a tiled selection is never materialized
//! (#1268); `None` means "no selection", which selects everything.
//!
//! Ported from photorust's `Document::{copy_selection, paste_into,
//! clear_selection_pixels}` (<https://github.com/perfecto25/photorust>).

use pictura_core::{
    layer_pixel_locked, layer_transparency_locked, Coverage, Document, Layer, LayerMask,
    PixelBuffer, Plane, PsdRect,
};
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use super::create::{insert_node, next_layer_name, transparent_layer};
use super::paths::{resolve_path, resolve_path_mut};

/// A copied region in document space: `rgba` is the source pixels over `rect`
/// (packed, row-major), `mask` the selection coverage over the same box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clip {
    pub rect: PsdRect,
    pub rgba: Vec<u8>,
    pub mask: Vec<u8>,
}

impl Clip {
    pub fn width(&self) -> u32 {
        self.rect.width().max(0) as u32
    }

    pub fn height(&self) -> u32 {
        self.rect.height().max(0) as u32
    }

    /// A clip from another application's image: packed straight RGBA placed
    /// at the canvas origin with full coverage, since its source position is
    /// unknown. `None` for a zero dimension or a buffer of the wrong length.
    pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Option<Clip> {
        let count = width as usize * height as usize;
        if count == 0 || rgba.len() != count * 4 {
            return None;
        }
        Some(Clip {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: height as i32,
                right: width as i32,
            },
            rgba,
            mask: vec![255; count],
        })
    }

    /// Packed straight RGBA with the selection coverage folded into alpha:
    /// the pixels a paste shows, and what the system clipboard carries.
    pub fn masked_rgba(&self) -> Vec<u8> {
        let mut out = self.rgba.clone();
        for (px, &coverage) in out.as_chunks_mut::<4>().0.iter_mut().zip(&self.mask) {
            px[3] = scale(px[3], coverage);
        }
        out
    }
}

/// How a paste uses the selection in place when it happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteMode {
    /// A new layer of its own; the selection is ignored.
    Plain,
    /// Masked to the selection (Paste Into).
    Into,
    /// Masked to everything outside the selection (Paste Outside).
    Outside,
}

/// Copy the selected pixels of the layer at `path`. Refuses (`None`) a group,
/// an adjustment or other channel-less layer, a region that misses the layer or
/// the canvas, and a selection that covers only transparent pixels.
pub fn copy_layer(doc: &Document, path: &str, selection: Option<&Coverage>) -> Option<Clip> {
    let layer = resolve_path(doc, path)?;
    if !has_pixels(layer) {
        return None;
    }
    let region = layer.rect.intersect(canvas(doc))?;
    let region = match selection {
        Some(coverage) => region.intersect(coverage.covered_bounds(canvas(doc))?)?,
        None => region,
    };
    let src_w = layer.rect.width() as usize;
    let plane = |id: i16| layer.channels().iter().find(|c| c.id == id).map(|c| &c.data);
    let red = plane(0)?;
    // A grayscale layer has only channel 0; replicate it into G and B.
    let green = plane(1).unwrap_or(red);
    let blue = plane(2).unwrap_or(red);
    let alpha = plane(-1);
    build(region, doc.width, selection, |x, y| {
        let i = (y - layer.rect.top) as usize * src_w + (x - layer.rect.left) as usize;
        let at = |data: &Plane<u8>| data.sample(i).unwrap_or(0);
        [at(red), at(green), at(blue), alpha.map_or(255, at)]
    })
}

/// Copy the selected pixels of the document composite: `composite` is the
/// 4-plane planar RGBA frame of the visible image, document-sized. Refuses on
/// the same empty-region rules as [`copy_layer`].
pub fn copy_merged(
    doc: &Document,
    composite: &PixelBuffer,
    selection: Option<&Coverage>,
) -> Option<Clip> {
    if composite.channels != 4 || composite.width != doc.width || composite.height != doc.height {
        return None;
    }
    let region = match selection {
        Some(coverage) => canvas(doc).intersect(coverage.covered_bounds(canvas(doc))?)?,
        None => canvas(doc),
    };
    let plane = doc.width as usize * doc.height as usize;
    build(region, doc.width, selection, |x, y| {
        let i = y as usize * doc.width as usize + x as usize;
        let at = |c: usize| composite.data.sample(c * plane + i).unwrap_or(0);
        [at(0), at(1), at(2), at(3)]
    })
}

/// [`copy_merged`] reading only the copied rectangle: `read(region)` returns
/// that rectangle of the composite as 4-plane planar RGBA, so a small copy of a
/// large document composites no more than its bounds (#1274).
pub fn copy_merged_region(
    doc: &Document,
    selection: Option<&Coverage>,
    read: impl FnOnce(PsdRect) -> PixelBuffer,
) -> Option<Clip> {
    let region = match selection {
        Some(coverage) => canvas(doc).intersect(coverage.covered_bounds(canvas(doc))?)?,
        None => canvas(doc),
    };
    let pixels = read(region);
    let (rw, rh) = (region.width().max(0) as u32, region.height().max(0) as u32);
    if pixels.channels != 4 || pixels.width != rw || pixels.height != rh {
        return None;
    }
    let plane = rw as usize * rh as usize;
    let data = pixels.data.as_slice();
    build(region, doc.width, selection, |x, y| {
        let i = (y - region.top) as usize * rw as usize + (x - region.left) as usize;
        [
            data[i],
            data[plane + i],
            data[2 * plane + i],
            data[3 * plane + i],
        ]
    })
}

/// Erase the selected pixels of the layer at `path`: alpha scales by
/// `(255 - coverage) / 255`. A Background (no alpha channel) is filled toward
/// white instead. Refuses a group, a channel-less layer, a pixel lock, and a
/// transparency lock on a layer with alpha. Returns true only when a pixel
/// changed.
pub fn clear_layer(doc: &mut Document, path: &str, selection: Option<&Coverage>) -> bool {
    let (doc_w, doc_h) = (doc.width, doc.height);
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    if !has_pixels(layer) || layer_pixel_locked(layer) {
        return false;
    }
    let has_alpha = layer.channels().iter().any(|c| c.id == -1);
    if has_alpha && layer_transparency_locked(layer) {
        return false;
    }
    let rect = layer.rect;
    let src_w = rect.width().max(0) as usize;
    let src_h = rect.height().max(0) as usize;
    if src_w == 0 || src_h == 0 {
        return false;
    }
    // Columns of the layer that the selection can reach; the clear is bounded
    // to them so a small selection on a canvas-sized layer stays cheap.
    let (col0, col1) = match selection {
        None => (0, src_w),
        Some(coverage) => {
            let Some(bounds) = coverage.covered_bounds(PsdRect {
                left: 0,
                top: 0,
                right: doc_w as i32,
                bottom: doc_h as i32,
            }) else {
                return false;
            };
            let x0 = rect.left.max(bounds.left).max(0);
            let x1 = rect.right.min(bounds.right).min(doc_w as i32);
            if x1 <= x0 {
                return false;
            }
            ((x0 - rect.left) as usize, (x1 - rect.left) as usize)
        }
    };
    let mut changed = false;
    for channel in layer.channels_mut().iter_mut() {
        // ponytail: a Background clears to white; there is no background
        // swatch to read yet.
        let erase = if has_alpha {
            channel.id == -1
        } else {
            channel.id >= 0
        };
        if !erase {
            continue;
        }
        for row in 0..src_h {
            let y = rect.top + row as i32;
            if selection.is_some() && (y < 0 || y >= doc_h as i32) {
                continue;
            }
            let mut pixels = vec![0u8; col1 - col0];
            channel
                .data
                .read_rect_into(col0..col1, row..row + 1, src_w, &mut pixels);
            let mut row_changed = false;
            let x0 = rect.left + col0 as i32;
            let coverage = selection.map(|coverage| coverage.row(x0, y, col1 - col0));
            let clear = |value: u8, cov: u8| {
                if has_alpha {
                    scale(value, 255 - cov)
                } else {
                    value + scale(255 - value, cov)
                }
            };
            match coverage {
                None => {
                    for value in pixels.iter_mut() {
                        let next = clear(*value, 255);
                        row_changed |= next != *value;
                        *value = next;
                    }
                }
                Some(coverage) => {
                    for (value, &cov) in pixels.iter_mut().zip(coverage.iter()) {
                        if cov == 0 {
                            continue;
                        }
                        let next = clear(*value, cov);
                        row_changed |= next != *value;
                        *value = next;
                    }
                }
            }
            if row_changed {
                changed = true;
                channel
                    .data
                    .write_rect_from(col0..col1, row..row + 1, src_w, &pixels);
            }
        }
        channel.data.settle();
    }
    changed
}

/// Insert `clip` as a new raster layer named `Layer N` with its top-left at
/// `(left, top)`, by the New Layer placement rule relative to `selection_path`
/// (top of the stack when empty). The selection coverage the clip was copied
/// with becomes its alpha. [`PasteMode::Into`] / [`PasteMode::Outside`] add a
/// document-sized layer mask from the selection (inverted for Outside) and
/// refuse without one. Returns the new path, or empty on a refusal.
pub fn paste_clip(
    doc: &mut Document,
    selection_path: &str,
    clip: &Clip,
    (left, top): (i32, i32),
    mode: PasteMode,
    selection: Option<&Coverage>,
) -> String {
    let (w, h) = (clip.width(), clip.height());
    let count = w as usize * h as usize;
    if count == 0 || clip.rgba.len() != count * 4 || clip.mask.len() != count {
        return String::new();
    }
    let mask = match (mode, selection) {
        (PasteMode::Plain, _) => None,
        (_, None) => return String::new(),
        (_, Some(coverage)) => Some(selection_mask(doc, coverage, mode == PasteMode::Outside)),
    };
    let mut layer = transparent_layer(w, h, &next_layer_name(doc, "Layer"));
    layer.rect = PsdRect {
        top,
        left,
        bottom: top + h as i32,
        right: left + w as i32,
    };
    let pixels = clip.masked_rgba();
    for channel in layer.channels_mut().iter_mut() {
        let offset = match channel.id {
            0 => 0,
            1 => 1,
            2 => 2,
            -1 => 3,
            _ => continue,
        };
        for (i, value) in channel.data.iter_mut().enumerate() {
            *value = pixels[i * 4 + offset];
        }
    }
    layer.mask = mask;
    insert_node(doc, selection_path, layer)
}

/// The bounding box of nonzero `coverage` over a `width * height` plane, or
/// `None` when nothing is selected. A tiled plane is read tile by tile, its
/// uniform tiles whole, so it is never materialized.
pub fn coverage_bounds(coverage: &Plane<u8>, width: u32, height: u32) -> Option<PsdRect> {
    pictura_core::mask_ops::coverage_bounds(coverage, width as usize, height as usize).map(
        |(x0, y0, x1, y1)| PsdRect {
            top: y0 as i32,
            left: x0 as i32,
            bottom: y1 as i32,
            right: x1 as i32,
        },
    )
}

/// How many planes [`plane_has_coverage`] remembers before it starts over.
const COVERAGE_MEMO_ENTRIES: usize = 256;

/// A plane's `(ident, width, height)`. An ident names one set of bytes, so an
/// answer stored under it never goes stale.
type CoverageKey = (u64, u32, u32);

/// [`plane_has_coverage`] answers by [`CoverageKey`].
static COVERAGE_MEMO: LazyLock<Mutex<HashMap<CoverageKey, bool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(test)]
thread_local! {
    /// Scans [`plane_has_coverage`] started on this thread, for the tests that
    /// prove a remembered plane is not read again.
    pub(super) static COVERAGE_SCANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Whether `coverage` holds a nonzero sample: exactly when
/// [`coverage_bounds`] is `Some`. Rows are read in place and the scan
/// stops at the first hit. The answer is remembered by the plane's ident, so
/// the menu checks one refresh runs read an unchanged plane once (#1145).
pub fn plane_has_coverage(coverage: &Plane<u8>, width: u32, height: u32) -> bool {
    let (w, h) = (width as usize, height as usize);
    if w == 0 || coverage.len() < w * h {
        return false;
    }
    let key = (coverage.ident(), width, height);
    let memo = || {
        COVERAGE_MEMO
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    };
    if let Some(&known) = memo().get(&key) {
        return known;
    }
    #[cfg(test)]
    COVERAGE_SCANS.with(|scans| scans.set(scans.get() + 1));
    let covered = (0..h).into_par_iter().any(|y| {
        coverage
            .row(y, w)
            .is_some_and(|row| row.iter().any(|&v| v > 0))
    });
    let mut memo = memo();
    if memo.len() >= COVERAGE_MEMO_ENTRIES {
        memo.clear();
    }
    memo.insert(key, covered);
    covered
}

/// Sample `region` through `pixel` into a clip, with the selection coverage (or
/// full coverage) as its mask. `None` for an empty region or when every sampled
/// pixel is transparent after masking.
fn build(
    region: PsdRect,
    _doc_w: u32,
    selection: Option<&Coverage>,
    pixel: impl Fn(i32, i32) -> [u8; 4],
) -> Option<Clip> {
    if region.width() <= 0 || region.height() <= 0 {
        return None;
    }
    let count = region.width() as usize * region.height() as usize;
    let mut rgba = Vec::with_capacity(count * 4);
    let mut mask = Vec::with_capacity(count);
    let mut visible = false;
    for y in region.top..region.bottom {
        let row = selection.map(|coverage| coverage.row(region.left, y, region.width() as usize));
        for x in region.left..region.right {
            let px = pixel(x, y);
            let cov = row
                .as_ref()
                .map_or(255, |row| row[(x - region.left) as usize]);
            visible |= scale(px[3], cov) > 0;
            rgba.extend_from_slice(&px);
            mask.push(cov);
        }
    }
    visible.then_some(Clip {
        rect: region,
        rgba,
        mask,
    })
}

/// The selection as a document-sized layer mask. Outside the canvas a Paste Into
/// hides and a Paste Outside shows.
fn selection_mask(doc: &Document, coverage: &Coverage, invert: bool) -> LayerMask {
    use pictura_core::mask_ops::{tiled_view, zip_tiles};
    let (w, h) = (doc.width as usize, doc.height as usize);
    let plane = coverage
        .plane_within(canvas(doc))
        .expect("valid canvas geometry");
    let data = if invert {
        let view = tiled_view(&plane, w, h);
        zip_tiles(&view, &view, w, h, |v, _| 255 - v)
    } else {
        plane
    };
    LayerMask {
        rect: canvas(doc),
        default_color: if invert { 255 } else { 0 },
        data: Some(data),
        ..Default::default()
    }
}

fn has_pixels(layer: &Layer) -> bool {
    !layer.is_group() && !layer.has_adjustment_content() && layer.channels().iter().any(|c| c.id == 0)
}

pub(super) fn canvas(doc: &Document) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    }
}

/// `round(value * coverage / 255)`.
fn scale(value: u8, coverage: u8) -> u8 {
    ((value as u32 * coverage as u32 + 127) / 255) as u8
}
