//! The Paint Bucket's engine (`docs/03-tools/gradient-and-paint-bucket.md`).
//!
//! The Paint Bucket is the Magic Wand with paint instead of a selection:
//! [`flood`] answers "which pixels belong with the one clicked?" as a coverage
//! mask, and [`fill`] lays the foreground colour or a pattern through it.
//!
//! Unlike the Magic Wand's flood, this one counts **alpha** as a channel, so a
//! fill inside a drawn outline on a transparent layer stops at the outline
//! instead of treating empty pixels as black.
//!
//! Ported from photorust's `core/src/bucket.rs` and `core/src/wand.rs`
//! (<https://github.com/perfecto25/photorust>), extended with the pattern
//! fill. Behavioural parity only.

use crate::eraser::antialias_mask;
use crate::fill::{fill_layer_within, Alpha};
use crate::healing::RgbaImage;
use crate::stroke::{layer_at, layer_planes, parse_layer_path};
use crate::{PaintMode, Rgba};
use pictura_core::mask_ops::{self, Border};
use pictura_core::{Document, Layer, PixelBuffer, PsdRect};
use rayon::prelude::*;

/// What the Paint Bucket fills with: the options bar's Fill menu.
#[derive(Clone, Copy, Debug)]
pub enum BucketPaint<'a> {
    Foreground(Rgba),
    /// A tile repeated from the document origin.
    Pattern(&'a RgbaImage),
}

/// Where the Paint Bucket reads the colours it compares, in document space.
#[derive(Clone, Copy)]
pub enum BucketSource<'a> {
    /// A pixel layer: transparent outside its rectangle, opaque where it has
    /// no transparency plane.
    Layer(&'a Layer),
    /// A planar RGBA composite the size of the document.
    Composite(&'a PixelBuffer),
}

impl<'a> BucketSource<'a> {
    /// The pixel layer at `path`; `None` for a group, an adjustment, or a
    /// missing layer.
    pub fn layer(doc: &'a Document, path: &str) -> Option<Self> {
        let layer = layer_at(doc, &parse_layer_path(path)?)?;
        (!layer.is_group && layer.adjustment.is_none()).then_some(Self::Layer(layer))
    }

    /// Document row `y` from column `left` on, as straight RGBA, into `out`:
    /// read from the planes, so no copy of the whole source is made.
    fn row(&self, y: i32, left: i32, out: &mut [[u8; 4]]) {
        out.fill([0; 4]);
        match *self {
            Self::Layer(layer) => {
                let rect = layer.rect;
                let (lw, lh) = (rect.width().max(0), rect.height().max(0));
                let ly = y - rect.top;
                if lw == 0 || ly < 0 || ly >= lh {
                    return;
                }
                let planes = layer_planes(layer);
                let at = |k: usize| {
                    planes[k]
                        .and_then(|p| p.row(ly as usize, lw as usize))
                        .unwrap_or(&[])
                };
                let (r, g, b, a) = (at(0), at(1), at(2), at(3));
                for (i, px) in out.iter_mut().enumerate() {
                    let lx = left + i as i32 - rect.left;
                    if lx < 0 || lx >= lw {
                        continue;
                    }
                    let lx = lx as usize;
                    *px = [
                        r.get(lx).copied().unwrap_or(0),
                        g.get(lx).copied().unwrap_or(0),
                        b.get(lx).copied().unwrap_or(0),
                        a.get(lx).copied().unwrap_or(255),
                    ];
                }
            }
            Self::Composite(buf) => {
                let (bw, bh) = (buf.width as usize, buf.height as usize);
                if y < 0 || y as usize >= bh {
                    return;
                }
                let y = y as usize;
                let at = |k: usize| buf.data.row(k * bh + y, bw).unwrap_or(&[]);
                let (r, g, b, a) = (at(0), at(1), at(2), at(3));
                for (i, px) in out.iter_mut().enumerate() {
                    let x = left + i as i32;
                    if x < 0 {
                        continue;
                    }
                    let x = x as usize;
                    *px = [
                        r.get(x).copied().unwrap_or(0),
                        g.get(x).copied().unwrap_or(0),
                        b.get(x).copied().unwrap_or(0),
                        a.get(x).copied().unwrap_or(0),
                    ];
                }
            }
        }
    }
}

/// The pixels of `source` inside `bounds` (document pixels) that match the
/// one at document point `seed` within `tolerance` (0–255 per channel, alpha
/// included, inclusive): those 4-connected to it when `contiguous`, otherwise
/// every one. `selection`, a document-sized coverage `canvas_width` wide, is a
/// wall: an unselected pixel never matches, so the contiguous flood does not
/// pass through it. A row-major 0 / 255 mask the size of `bounds`; `None` for
/// a seed outside `bounds` or the selection.
///
/// The flood works only inside `bounds`, so a click inside a small selection
/// costs that selection's box, not the document (#1207).
pub fn flood(
    source: BucketSource<'_>,
    canvas_width: u32,
    bounds: PsdRect,
    selection: Option<&[u8]>,
    seed: (i32, i32),
    tolerance: u8,
    contiguous: bool,
) -> Option<Vec<u8>> {
    let (bw, bh) = (
        bounds.width().max(0) as usize,
        bounds.height().max(0) as usize,
    );
    let inside = seed.0 >= bounds.left
        && seed.1 >= bounds.top
        && seed.0 < bounds.right
        && seed.1 < bounds.bottom;
    if !inside || bounds.left < 0 || bounds.top < 0 {
        return None;
    }
    let cw = canvas_width as usize;
    let selected = |y: i32, x0: i32, len: usize| {
        selection.map(|s| {
            let start = y as usize * cw + x0 as usize;
            s.get(start..start + len).unwrap_or(&[])
        })
    };
    if selected(seed.1, seed.0, 1).is_some_and(|s| s.first().is_none_or(|&v| v == 0)) {
        return None;
    }
    let mut target = [[0u8; 4]];
    source.row(seed.1, seed.0, &mut target);
    let target = target[0];
    // The largest difference on any one channel: one badly-off channel is
    // enough to reject a pixel.
    let matches = |p: &[u8; 4]| {
        p.iter()
            .zip(&target)
            .all(|(a, b)| a.abs_diff(*b) <= tolerance)
    };
    // Every pixel is tested once, in parallel; the contiguous flood then only
    // walks the answers. When every pixel matches (an empty or uniform layer)
    // they are all connected, so the flood is the whole box (#1149). The match
    // plane is canvas-scale, so a refused or failed allocation floods nothing
    // and leaves the governor's refusal for the caller (#1211).
    let mut matched = pictura_core::memory::try_plane(bw * bh, 0, "a Paint Bucket flood").ok()?;
    matched
        .par_chunks_mut(bw)
        .enumerate()
        .for_each(|(row, out)| {
            let y = bounds.top + row as i32;
            let mut pixels = vec![[0u8; 4]; bw];
            source.row(y, bounds.left, &mut pixels);
            let wall = selected(y, bounds.left, bw);
            for (x, (m, p)) in out.iter_mut().zip(&pixels).enumerate() {
                let open = wall.is_none_or(|w| w.get(x).is_some_and(|&v| v > 0));
                *m = (open && matches(p)) as u8 * 255;
            }
        });
    if !contiguous || !matched.contains(&0) {
        return Some(matched);
    }
    let at = (seed.1 - bounds.top) as usize * bw + (seed.0 - bounds.left) as usize;
    mask_ops::flood_from(bw, bh, at, |_, to| matched[to] != 0)
}

/// The anti-aliased edge of a [`flood`] mask over `bounds`. An unselected
/// pixel is a wall for the flood, not an edge of the colour region, so it
/// does not soften its neighbours: it is treated as filled, and the fill's
/// selection keeps it unpainted.
pub fn antialias(
    mut flood: Vec<u8>,
    canvas_width: u32,
    bounds: PsdRect,
    selection: Option<&[u8]>,
) -> Vec<u8> {
    let bw = bounds.width().max(0) as usize;
    if let Some(selection) = selection {
        let cw = canvas_width as usize;
        flood
            .par_chunks_mut(bw.max(1))
            .enumerate()
            .for_each(|(row, out)| {
                let start = (bounds.top as usize + row) * cw + bounds.left as usize;
                let wall = selection.get(start..start + bw).unwrap_or(&[]);
                for (m, &s) in out.iter_mut().zip(wall) {
                    if s == 0 {
                        *m = 255;
                    }
                }
            });
    }
    antialias_mask(&flood, bw)
}

/// Fill the pixel layer at `path` with `paint` through a document-sized
/// coverage `mask` (0–255, row-major over `doc.width`) at `opacity` (0.0–1.0)
/// in `mode`, inside `selection` (another such mask) when there is one. The
/// `alpha` policy says what the fill does with the pixels' transparency; a
/// transparency lock overrides it and keeps every pixel's alpha. Returns the
/// document rectangle changed; `None` when nothing changed or the layer cannot
/// be filled.
#[allow(clippy::too_many_arguments)]
pub fn fill(
    doc: &mut Document,
    path: &str,
    mask: &[u8],
    paint: BucketPaint<'_>,
    mode: PaintMode,
    opacity: f32,
    selection: Option<&[u8]>,
    alpha: Alpha,
) -> Option<PsdRect> {
    let opacity = opacity.clamp(0.0, 1.0);
    let width = doc.width as i32;
    let empty_tile = matches!(paint, BucketPaint::Pattern(t) if t.width <= 0 || t.height <= 0);
    if opacity <= 0.0 || empty_tile || mask.len() < (width * doc.height as i32) as usize {
        return None;
    }
    let Some(damage) = coverage_bounds(mask, width, doc.height as i32) else {
        return None;
    };
    fill_layer_within(doc, path, mode, selection, alpha, Some(damage), |x, y| {
        let cover = mask[(y * width + x) as usize];
        if cover == 0 {
            return None;
        }
        let color = match paint {
            BucketPaint::Foreground(color) => color,
            BucketPaint::Pattern(tile) => {
                let [r, g, b, a] = tile.get(x.rem_euclid(tile.width), y.rem_euclid(tile.height));
                Rgba { r, g, b, a }
            }
        };
        Some((color, cover as f32 / 255.0 * opacity))
    })
}

/// The bounding box of the nonzero bytes of a `width`-wide coverage plane, in
/// plane coordinates; `None` when it covers nothing.
fn coverage_bounds(mask: &[u8], width: i32, height: i32) -> Option<PsdRect> {
    let w = width.max(0) as usize;
    if w == 0 {
        return None;
    }
    let mut rect: Option<PsdRect> = None;
    for (row, pixels) in mask.chunks(w).take(height.max(0) as usize).enumerate() {
        let Some(first) = pixels.iter().position(|&v| v > 0) else {
            continue;
        };
        let last = pixels.iter().rposition(|&v| v > 0).unwrap_or(first);
        let y = row as i32;
        rect = Some(match rect {
            None => PsdRect {
                top: y,
                left: first as i32,
                bottom: y + 1,
                right: last as i32 + 1,
            },
            Some(r) => PsdRect {
                top: r.top,
                left: r.left.min(first as i32),
                bottom: y + 1,
                right: r.right.max(last as i32 + 1),
            },
        });
    }
    rect
}

/// [`fill`] through a coverage `mask` that covers only `bounds` (document
/// pixels, row-major over `bounds.width()`), or the whole of `bounds` without
/// one: the layer is visited and written only where the box reaches, so a
/// shape or a selection filled on a large document costs its own box.
#[allow(clippy::too_many_arguments)]
pub fn fill_within(
    doc: &mut Document,
    path: &str,
    bounds: PsdRect,
    mask: Option<&[u8]>,
    paint: BucketPaint<'_>,
    mode: PaintMode,
    opacity: f32,
    selection: Option<&[u8]>,
    alpha: Alpha,
) -> Option<PsdRect> {
    let opacity = opacity.clamp(0.0, 1.0);
    let empty_tile = matches!(paint, BucketPaint::Pattern(t) if t.width <= 0 || t.height <= 0);
    let bw = bounds.width().max(0);
    let short = mask.is_some_and(|m| m.len() < (bw * bounds.height().max(0)) as usize);
    if opacity <= 0.0 || empty_tile || short {
        return None;
    }
    let damage = match mask {
        Some(mask) => {
            let Some(hit) = coverage_bounds(mask, bw, bounds.height().max(0)) else {
                return None;
            };
            PsdRect {
                top: hit.top + bounds.top,
                left: hit.left + bounds.left,
                bottom: hit.bottom + bounds.top,
                right: hit.right + bounds.left,
            }
        }
        None => bounds,
    };
    fill_layer_within(doc, path, mode, selection, alpha, Some(damage), |x, y| {
        if x < bounds.left || y < bounds.top || x >= bounds.right || y >= bounds.bottom {
            return None;
        }
        let cover = mask.map_or(255, |m| {
            m[((y - bounds.top) * bw + x - bounds.left) as usize]
        });
        if cover == 0 {
            return None;
        }
        let color = match paint {
            BucketPaint::Foreground(color) => color,
            BucketPaint::Pattern(tile) => {
                let [r, g, b, a] = tile.get(x.rem_euclid(tile.width), y.rem_euclid(tile.height));
                Rgba { r, g, b, a }
            }
        };
        Some((color, cover as f32 / 255.0 * opacity))
    })
}

/// The stroke band around a selection: a solid `color` laid over the pixels
/// within `width` px of the selection edge, `Inside` / `Outside` / `Center`
/// choosing which side. `selection` is a document-sized 0–255 coverage mask
/// and `bounds` the box of its covered pixels. Returns the document rectangle
/// changed; `None` when nothing changed, the selection is empty, or the layer
/// cannot be filled.
///
/// The band is built in `bounds` grown by the stroke's reach and one pixel,
/// clipped to the canvas: that ring holds no selected pixel, so the clamped
/// border at the window's edge sees what the document's would, and a stroke
/// around a small selection costs its own box (#1207).
///
/// ponytail: the band is built from integer max/min (dilate/erode) filters, so
/// it is a square-capped approximation of CS6's round stroke; the brush-mode
/// blend rides through `fill_layer` and `opacity` is `0.0`–`1.0`.
#[allow(clippy::too_many_arguments)]
pub fn stroke_selection(
    doc: &mut Document,
    path: &str,
    selection: &[u8],
    bounds: PsdRect,
    color: Rgba,
    width: u32,
    position: StrokeAlign,
    mode: PaintMode,
    opacity: f32,
) -> Option<PsdRect> {
    let (w, h) = (doc.width as i32, doc.height as i32);
    if width == 0 || opacity <= 0.0 || selection.len() < (w * h) as usize {
        return None;
    }
    let radius = width.min(u32::from(u16::MAX)) as i32;
    let (out_r, in_r) = match position {
        StrokeAlign::Outside => (radius, 0),
        StrokeAlign::Inside => (0, radius),
        StrokeAlign::Center => ((radius + 1) / 2, radius / 2),
    };
    let window = stroke_window(bounds, out_r.max(in_r) + 1, (w, h))?;
    let (ww, wh) = (window.width() as usize, window.height() as usize);
    let crop: Vec<u8> = (window.top..window.bottom)
        .flat_map(|y| {
            let start = (y * w + window.left) as usize;
            selection[start..start + ww].iter().copied()
        })
        .collect();
    if !crop.iter().any(|&v| v > 0) {
        return None;
    }
    let dilated = mask_ops::dilate(&crop, ww, wh, out_r.max(0) as usize, Border::Clamp);
    let eroded = mask_ops::erode(&crop, ww, wh, in_r.max(0) as usize, Border::Clamp);
    let band: Vec<u8> = crop
        .iter()
        .zip(&dilated)
        .zip(&eroded)
        .map(|((&m, &d), &e)| match position {
            StrokeAlign::Outside => d.saturating_sub(m),
            StrokeAlign::Inside => m.saturating_sub(e),
            StrokeAlign::Center => d.saturating_sub(e),
        })
        .collect();
    drop((crop, dilated, eroded));
    fill_within(
        doc,
        path,
        window,
        Some(&band),
        BucketPaint::Foreground(color),
        mode,
        opacity,
        None,
        Alpha::Blend,
    )
}

/// `bounds` grown by `reach` on every side and clipped to a `canvas`-sized
/// document; `None` when nothing of it is left.
pub fn stroke_window(bounds: PsdRect, reach: i32, canvas: (i32, i32)) -> Option<PsdRect> {
    PsdRect {
        top: bounds.top.saturating_sub(reach),
        left: bounds.left.saturating_sub(reach),
        bottom: bounds.bottom.saturating_add(reach),
        right: bounds.right.saturating_add(reach),
    }
    .intersect(PsdRect {
        top: 0,
        left: 0,
        bottom: canvas.1,
        right: canvas.0,
    })
}

/// Which side of the selection edge the stroke band covers.
#[derive(Clone, Copy, Debug)]
pub enum StrokeAlign {
    Inside,
    Outside,
    Center,
}

#[cfg(test)]
mod tests;
