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

use std::ops::Range;

use crate::eraser::antialias_mask;
use crate::fill::{fill_layer_within, Alpha, Through};
use crate::healing::RgbaImage;
use crate::stroke::{layer_at, layer_planes, parse_layer_path};
use crate::{PaintMode, Rgba};
use pictura_core::mask_ops::{
    self, coverage_bounds, empty_mask, flood_tiled, map_bands, match_mask, zip_tiles, Border, Grid,
    MatchTiles, Seeds, TileMatch, TiledMask,
};
use pictura_core::{Document, Layer, PixelBuffer, PsdRect, TILE};

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
                let lx0 = (left - rect.left).max(0);
                let lx1 = (left + out.len() as i32 - rect.left).min(lw);
                if lx1 <= lx0 {
                    return;
                }
                let (n, ly) = ((lx1 - lx0) as usize, ly as usize);
                let at = |k: usize, missing: u8| {
                    let mut span = vec![missing; n];
                    if let Some(p) = planes[k] {
                        p.read_rect_into(
                            lx0 as usize..lx1 as usize,
                            ly..ly + 1,
                            lw as usize,
                            &mut span,
                        );
                    }
                    span
                };
                let (r, g, b, a) = (at(0, 0), at(1, 0), at(2, 0), at(3, 255));
                let skip = (lx0 - (left - rect.left)) as usize;
                for (i, px) in out[skip..skip + n].iter_mut().enumerate() {
                    *px = [r[i], g[i], b[i], a[i]];
                }
            }
            Self::Composite(buf) => {
                let (bw, bh) = (buf.width as usize, buf.height as usize);
                if y < 0 || y as usize >= bh {
                    return;
                }
                let y = y as usize;
                let x0 = left.max(0);
                let x1 = (left + out.len() as i32).min(bw as i32);
                if x1 <= x0 {
                    return;
                }
                let n = (x1 - x0) as usize;
                let at = |k: usize| {
                    let mut span = vec![0u8; n];
                    let row = k * bh + y;
                    buf.data
                        .read_rect_into(x0 as usize..x1 as usize, row..row + 1, bw, &mut span);
                    span
                };
                let (r, g, b, a) = (at(0), at(1), at(2), at(3));
                let skip = (x0 - left) as usize;
                for (i, px) in out[skip..skip + n].iter_mut().enumerate() {
                    *px = [r[i], g[i], b[i], a[i]];
                }
            }
        }
    }

    /// The one RGBA value every pixel of document columns `xs` × rows `ys`
    /// holds, read from the tile states alone; `None` when it is not known
    /// to be one value.
    fn uniform(&self, xs: Range<usize>, ys: Range<usize>) -> Option<[u8; 4]> {
        match *self {
            Self::Layer(layer) => {
                let r = layer.rect;
                let (x0, x1, y0, y1) = (
                    xs.start as i32,
                    xs.end as i32,
                    ys.start as i32,
                    ys.end as i32,
                );
                if r.is_empty() || x1 <= r.left || x0 >= r.right || y1 <= r.top || y0 >= r.bottom {
                    return Some([0; 4]);
                }
                if x0 < r.left || x1 > r.right || y0 < r.top || y1 > r.bottom {
                    return None;
                }
                let lx = (x0 - r.left) as usize..(x1 - r.left) as usize;
                let ly = (y0 - r.top) as usize..(y1 - r.top) as usize;
                let planes = layer_planes(layer);
                let mut px = [0, 0, 0, 255];
                for (k, value) in px.iter_mut().enumerate() {
                    if let Some(p) = planes[k] {
                        *value = p.uniform_over(lx.clone(), ly.clone())?;
                    }
                }
                Some(px)
            }
            Self::Composite(buf) => {
                let h = buf.height as usize;
                let mut px = [0; 4];
                for (k, value) in px.iter_mut().enumerate() {
                    *value = buf
                        .data
                        .uniform_over(xs.clone(), k * h + ys.start..k * h + ys.end)?;
                }
                Some(px)
            }
        }
    }
}

/// The pixels of `source` inside `bounds` (document pixels) that match the
/// one at document point `seed` within `tolerance` (0–255 per channel, alpha
/// included, inclusive): those 4-connected to it when `contiguous`, otherwise
/// every one. `selection`, a document-sized coverage, is a wall: an
/// unselected pixel never matches, so the contiguous flood does not pass
/// through it. A 0 / 255 tiled mask of the document's `canvas` size, `Empty`
/// outside `bounds`; `None` for a seed outside `bounds` or the selection.
///
/// The flood works only inside `bounds`, so a click inside a small selection
/// costs that selection's box, not the document (#1207). It reads the source
/// one tile at a time as it reaches it, and a tile of one matching colour is
/// reached whole and comes out `Uniform` (#1214).
pub fn flood(
    source: BucketSource<'_>,
    canvas: (u32, u32),
    bounds: PsdRect,
    selection: Option<&TiledMask>,
    seed: (i32, i32),
    tolerance: u8,
    contiguous: bool,
) -> Option<TiledMask> {
    let query = Query {
        seed,
        tolerance,
        contiguous,
        channels: 4,
    };
    flood_in(source, canvas, bounds, selection, query)
}

/// The Magic Eraser's flood over a layer: the Magic Wand's test, which
/// compares colour and not alpha, run on the layer's planes over the whole
/// canvas with no selection.
pub fn magic_flood(
    source: BucketSource<'_>,
    canvas: (u32, u32),
    seed: (i32, i32),
    tolerance: u8,
    contiguous: bool,
) -> Option<TiledMask> {
    let whole = PsdRect {
        top: 0,
        left: 0,
        bottom: canvas.1 as i32,
        right: canvas.0 as i32,
    };
    let query = Query {
        seed,
        tolerance,
        contiguous,
        channels: 3,
    };
    flood_in(source, canvas, whole, None, query)
}

/// What a flood looks for: the pixels within `tolerance` of the seed's on its
/// first `channels` channels.
#[derive(Clone, Copy)]
struct Query {
    seed: (i32, i32),
    tolerance: u8,
    contiguous: bool,
    channels: usize,
}

fn flood_in(
    source: BucketSource<'_>,
    canvas: (u32, u32),
    bounds: PsdRect,
    selection: Option<&TiledMask>,
    query: Query,
) -> Option<TiledMask> {
    let Query {
        seed,
        tolerance,
        contiguous,
        channels,
    } = query;
    let (cw, ch) = (canvas.0 as usize, canvas.1 as usize);
    let inside = seed.0 >= bounds.left
        && seed.1 >= bounds.top
        && seed.0 < bounds.right
        && seed.1 < bounds.bottom;
    if !inside
        || bounds.left < 0
        || bounds.top < 0
        || seed.0 as usize >= cw
        || seed.1 as usize >= ch
    {
        return None;
    }
    let (sx, sy) = (seed.0 as usize, seed.1 as usize);
    if selection.is_some_and(|s| s.sample(sy * cw + sx).is_none_or(|v| v == 0)) {
        return None;
    }
    let mut target = [[0u8; 4]];
    source.row(seed.1, seed.0, &mut target);
    let target = target[0];
    // The largest difference on any one channel: one badly-off channel is
    // enough to reject a pixel.
    let matches = |p: &[u8; 4]| {
        p[..channels]
            .iter()
            .zip(&target)
            .all(|(a, b)| a.abs_diff(*b) <= tolerance)
    };
    let (bx0, by0) = (bounds.left as usize, bounds.top as usize);
    let (bx1, by1) = (
        (bounds.right.max(0) as usize).min(cw),
        (bounds.bottom.max(0) as usize).min(ch),
    );
    let eval = |_: usize, (x0, y0, tw, th): (usize, usize, usize, usize)| {
        let (ix0, iy0) = (x0.max(bx0), y0.max(by0));
        let (ix1, iy1) = ((x0 + tw).min(bx1), (y0 + th).min(by1));
        if ix0 >= ix1 || iy0 >= iy1 {
            return TileMatch::None;
        }
        let (xs, ys) = (x0..x0 + tw, y0..y0 + th);
        let wall = selection.map_or(Some(255), |s| s.uniform_over(xs.clone(), ys.clone()));
        if wall == Some(0) {
            return TileMatch::None;
        }
        let whole = (ix0, iy0, ix1, iy1) == (x0, y0, x0 + tw, y0 + th);
        if whole && wall.is_some() {
            if let Some(px) = source.uniform(xs.clone(), ys) {
                return if matches(&px) {
                    TileMatch::All
                } else {
                    TileMatch::None
                };
            }
        }
        let mut pixels = vec![[0u8; 4]; tw];
        let mut open = vec![255u8; tw];
        TileMatch::from_rows(tw, th, |dy, out| {
            let y = y0 + dy;
            if y < iy0 || y >= iy1 {
                return;
            }
            source.row(y as i32, x0 as i32, &mut pixels);
            if let Some(s) = selection {
                open.fill(0);
                s.read_rect_into(xs.clone(), y..y + 1, cw, &mut open);
            }
            for dx in ix0 - x0..ix1 - x0 {
                out[dx] = open[dx] > 0 && matches(&pixels[dx]);
            }
        })
    };
    if !contiguous {
        return Some(match_mask(cw, ch, eval));
    }
    let mut tiles = MatchTiles::new(cw, ch, eval);
    flood_tiled(cw, ch, Seeds::Pixel(sx, sy), &mut tiles)
}

/// The anti-aliased edge of a [`flood`] mask over `bounds`. An unselected
/// pixel is a wall for the flood, not an edge of the colour region, so it
/// does not soften its neighbours: it is treated as filled, and the fill's
/// selection keeps it unpainted. The 3×3 box is clamped to `bounds`, as it
/// was when the mask was the size of the box.
///
/// Tile by tile, each reading a one-pixel margin inside `bounds`: a tile
/// whose neighbourhood holds one value keeps it without a sample read.
pub fn antialias(
    flood: TiledMask,
    canvas: (u32, u32),
    bounds: PsdRect,
    selection: Option<&TiledMask>,
) -> TiledMask {
    let (cw, ch) = (canvas.0 as usize, canvas.1 as usize);
    let clip = bounds.intersect(PsdRect {
        top: 0,
        left: 0,
        bottom: ch as i32,
        right: cw as i32,
    });
    let Some(clip) = clip else {
        return flood;
    };
    let (bx0, by0, bx1, by1) = (
        clip.left as usize,
        clip.top as usize,
        clip.right as usize,
        clip.bottom as usize,
    );
    // What the edge pass sees: an unselected pixel as filled.
    let effective = |xs: Range<usize>, ys: Range<usize>| -> Option<u8> {
        let f = flood.uniform_over(xs.clone(), ys.clone())?;
        match selection {
            None => Some(f),
            Some(s) => Some(if s.uniform_over(xs, ys)? == 0 { 255 } else { f }),
        }
    };
    let grid = Grid::new(cw, ch);
    let mut out = empty_mask(cw, ch);
    for ty in by0 / TILE..by1.div_ceil(TILE) {
        for tx in bx0 / TILE..bx1.div_ceil(TILE) {
            let t = ty * grid.cols + tx;
            let (x0, y0, tw, th) = grid.rect(t);
            let (cx0, cy0) = (x0.max(bx0), y0.max(by0));
            let (cx1, cy1) = ((x0 + tw).min(bx1), (y0 + th).min(by1));
            let (wx0, wy0) = (
                cx0.saturating_sub(1).max(bx0),
                cy0.saturating_sub(1).max(by0),
            );
            let (wx1, wy1) = ((cx1 + 1).min(bx1), (cy1 + 1).min(by1));
            let centre = (cx1 - cx0) * (cy1 - cy0);
            if let Some(u) = effective(wx0..wx1, wy0..wy1) {
                if u == 0 {
                    continue;
                }
                if centre == tw * th {
                    out.set_tile_uniform(t, u);
                } else {
                    out.write_rect_from(cx0..cx1, cy0..cy1, cw, &vec![u; centre]);
                }
                continue;
            }
            let (ww, wh) = (wx1 - wx0, wy1 - wy0);
            let mut window = vec![0u8; ww * wh];
            flood.read_rect_into(wx0..wx1, wy0..wy1, cw, &mut window);
            if let Some(s) = selection {
                let mut wall = vec![0u8; ww * wh];
                s.read_rect_into(wx0..wx1, wy0..wy1, cw, &mut wall);
                for (m, &v) in window.iter_mut().zip(&wall) {
                    if v == 0 {
                        *m = 255;
                    }
                }
            }
            let soft = antialias_mask(&window, ww);
            let mut part = Vec::with_capacity(centre);
            for y in cy0..cy1 {
                let at = (y - wy0) * ww + (cx0 - wx0);
                part.extend_from_slice(&soft[at..at + cx1 - cx0]);
            }
            out.write_rect_from(cx0..cx1, cy0..cy1, cw, &part);
        }
    }
    out.settle();
    out
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
    selection: Option<&TiledMask>,
    alpha: Alpha,
) -> Option<PsdRect> {
    let opacity = opacity.clamp(0.0, 1.0);
    let width = doc.width as i32;
    let empty_tile = matches!(paint, BucketPaint::Pattern(t) if t.width <= 0 || t.height <= 0);
    if opacity <= 0.0 || empty_tile || mask.len() < (width * doc.height as i32) as usize {
        return None;
    }
    let damage = dense_bounds(mask, width, doc.height as i32)?;
    let through = Through {
        selection,
        ..Through::default()
    };
    fill_layer_within(doc, path, mode, through, alpha, Some(damage), |x, y, _| {
        let cover = mask[(y * width + x) as usize];
        if cover == 0 {
            return None;
        }
        Some((paint_at(paint, x, y), cover as f32 / 255.0 * opacity))
    })
}

/// The bounding box of the nonzero bytes of a `width`-wide coverage plane, in
/// plane coordinates; `None` when it covers nothing.
fn dense_bounds(mask: &[u8], width: i32, height: i32) -> Option<PsdRect> {
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
    selection: Option<&TiledMask>,
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
            let hit = dense_bounds(mask, bw, bounds.height().max(0))?;
            PsdRect {
                top: hit.top + bounds.top,
                left: hit.left + bounds.left,
                bottom: hit.bottom + bounds.top,
                right: hit.right + bounds.left,
            }
        }
        None => bounds,
    };
    // A foreground fill with no mask lays the same colour at every pixel, so
    // a uniform tile is filled as one value.
    let through = Through {
        selection,
        mask: None,
        solid: mask.is_none() && matches!(paint, BucketPaint::Foreground(_)),
    };
    fill_layer_within(doc, path, mode, through, alpha, Some(damage), |x, y, _| {
        if x < bounds.left || y < bounds.top || x >= bounds.right || y >= bounds.bottom {
            return None;
        }
        let cover = mask.map_or(255, |m| {
            m[((y - bounds.top) * bw + x - bounds.left) as usize]
        });
        if cover == 0 {
            return None;
        }
        Some((paint_at(paint, x, y), cover as f32 / 255.0 * opacity))
    })
}

/// The colour `paint` lays at document pixel `(x, y)`.
fn paint_at(paint: BucketPaint<'_>, x: i32, y: i32) -> Rgba {
    match paint {
        BucketPaint::Foreground(color) => color,
        BucketPaint::Pattern(tile) => {
            let [r, g, b, a] = tile.get(x.rem_euclid(tile.width), y.rem_euclid(tile.height));
            Rgba { r, g, b, a }
        }
    }
}

/// [`fill`] through a document-sized tiled coverage `mask` that is zero
/// outside `bounds` (a [`flood`] or a stroke band): the layer is visited only
/// in the tiles the mask's coverage crosses, reading the mask one tile row at
/// a time, and a tile the mask covers whole with a foreground colour on a
/// uniform layer tile is written as one value (#1214).
#[allow(clippy::too_many_arguments)]
pub fn fill_tiled(
    doc: &mut Document,
    path: &str,
    bounds: PsdRect,
    mask: &TiledMask,
    paint: BucketPaint<'_>,
    mode: PaintMode,
    opacity: f32,
    selection: Option<&TiledMask>,
    alpha: Alpha,
) -> Option<PsdRect> {
    let opacity = opacity.clamp(0.0, 1.0);
    let empty_tile = matches!(paint, BucketPaint::Pattern(t) if t.width <= 0 || t.height <= 0);
    if opacity <= 0.0 || empty_tile {
        return None;
    }
    let (cw, ch) = (doc.width as usize, doc.height as usize);
    let (x0, y0, x1, y1) = coverage_bounds(mask, cw, ch)?;
    let damage = PsdRect {
        top: y0 as i32,
        left: x0 as i32,
        bottom: y1 as i32,
        right: x1 as i32,
    }
    .intersect(bounds)?;
    let through = Through {
        selection,
        mask: Some(mask),
        solid: matches!(paint, BucketPaint::Foreground(_)),
    };
    fill_layer_within(
        doc,
        path,
        mode,
        through,
        alpha,
        Some(damage),
        |x, y, cover| {
            if x < bounds.left || y < bounds.top || x >= bounds.right || y >= bounds.bottom {
                return None;
            }
            Some((paint_at(paint, x, y), cover as f32 / 255.0 * opacity))
        },
    )
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
    selection: &TiledMask,
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
    let (wu, hu) = (w as usize, h as usize);
    let covered = coverage_bounds(selection, wu, hu).and_then(|(x0, y0, x1, y1)| {
        PsdRect {
            top: y0 as i32,
            left: x0 as i32,
            bottom: y1 as i32,
            right: x1 as i32,
        }
        .intersect(window)
    });
    covered?;
    // The window's ring past `bounds` holds no selected pixel, so the
    // window's clamped edge reads the zeros the document holds there: the
    // whole-document filters, clamped at the canvas edge, give the window's
    // band, and their tiles far from the edge are decided without a read.
    let (out_r, in_r) = (out_r.max(0) as usize, in_r.max(0) as usize);
    let dilated = map_bands(selection, wu, hu, out_r + 1, |s, w, h| {
        mask_ops::dilate(s, w, h, out_r, Border::Clamp)
    });
    let eroded = map_bands(selection, wu, hu, in_r + 1, |s, w, h| {
        mask_ops::erode(s, w, h, in_r, Border::Clamp)
    });
    let band = match position {
        StrokeAlign::Outside => zip_tiles(&dilated, selection, wu, hu, u8::saturating_sub),
        StrokeAlign::Inside => zip_tiles(selection, &eroded, wu, hu, u8::saturating_sub),
        StrokeAlign::Center => zip_tiles(&dilated, &eroded, wu, hu, u8::saturating_sub),
    };
    drop((dilated, eroded));
    fill_tiled(
        doc,
        path,
        window,
        &band,
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
