//! The erasers (`docs/03-tools/eraser-tools.md`). The **Eraser** is a Brush
//! stroke that takes pixels away instead of laying colour down.
//!
//! * On an ordinary layer it erases **to transparency** — the stroke's
//!   coverage multiplies the pixel's alpha down, so going over a half-erased
//!   edge deepens it instead of punching through.
//! * On the Background, or a layer whose transparency is locked, there is no
//!   alpha to take, so it paints the **background colour**.
//! * **Erase To History** paints the History Brush's source state back instead.
//!
//! Mode picks the tip: `Brush` is the Brush's soft tip, `Pencil` its hard
//! aliased one, and `Block` a hard square of 16 *screen* pixels at full
//! strength.
//!
//! The other two erase *by colour*, and always to transparency:
//!
//! * The **Background Eraser** is a brush that samples a colour under its
//!   crosshair (continuously, once, or the background swatch) and erases what
//!   matches within the dab, so dragged along a subject's edge with the
//!   crosshair on the background it cuts the subject out. It overrides Lock
//!   Transparency.
//! * The **Magic Eraser** is one click: the Magic Wand's flood, erased through
//!   [`magic_erase`] instead of selected.
//!
//! Erasing multiplies alpha down, never subtracts: a pixel half erased twice is
//! three-quarters gone, so going over an edge again deepens it smoothly.
//!
//! The plain Eraser is ported from photorust's erase mode (the Brush painting
//! the background colour), extended with erasing to transparency, Block, and
//! Erase To History from the spec; the Background and Magic Erasers from
//! photorust's `core/src/erase.rs` (<https://github.com/perfecto25/photorust>).
//! Behavioural parity only.

use crate::healing::RgbaImage;
use crate::replace::{dab_bounds, grow, match_strength, reachable, Limits, Sampling};
use crate::stamp::StampSource;
use crate::stroke::{layer_at, layer_at_mut, parse_layer_path};
use crate::{tip_coverage, PaintError, PaintMode, Rgba, Stroke, StrokeConfig};
#[cfg(test)]
use pictura_core::mask_ops::TiledMask;
use pictura_core::{
    layer_transparency_locked, Channel, Document, Layer, Plane, PsdRect, TileState,
};
use rayon::prelude::*;

/// The Block eraser's side, in *screen* pixels. CS6's Block is a fixed
/// screen-space square, so its document side is this divided by the zoom.
pub const BLOCK_SCREEN_PX: f32 = 16.0;

/// The largest Block side in document pixels: the `StrokeConfig` diameter cap.
const MAX_BLOCK_SIDE: u32 = 5000;

/// The Block eraser's side in document pixels at `zoom` (screen pixels per
/// document pixel), so the block covers the same number of screen pixels at
/// every zoom. Never below one document pixel, the finest an erase can be. A
/// non-finite or non-positive zoom is taken as 1.0.
pub fn block_side_doc_px(zoom: f64) -> u32 {
    let zoom = if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    };
    let side = (BLOCK_SCREEN_PX as f64 / zoom).round();
    side.clamp(1.0, MAX_BLOCK_SIDE as f64) as u32
}

/// The Eraser's Mode menu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EraserMode {
    #[default]
    Brush,
    Pencil,
    Block,
}

impl EraserMode {
    /// 0 Brush, 1 Pencil, 2 Block; `None` otherwise.
    pub fn from_i32(v: i32) -> Option<EraserMode> {
        match v {
            0 => Some(EraserMode::Brush),
            1 => Some(EraserMode::Pencil),
            2 => Some(EraserMode::Block),
            _ => None,
        }
    }
}

/// True when erasing `layer` paints the background colour: the Background, a
/// layer with no alpha channel, or a transparency lock.
pub fn erases_to_background(layer: &Layer) -> bool {
    layer.background
        || layer_transparency_locked(layer)
        || !layer.channels.iter().any(|c| c.id == -1)
}

/// Begin an Eraser stroke on the layer at `path`. `cfg` carries the tip,
/// Opacity, Flow, and the background colour (`cfg.background`); its `color`
/// and `mode` are replaced. With `history`, the stroke paints that source back
/// (Erase To History) instead of erasing. `zoom` is the canvas zoom (screen
/// pixels per document pixel); the Block mode's side is a screen-space size,
/// so it needs it.
pub fn begin_erase(
    doc: &Document,
    path: &str,
    cfg: StrokeConfig,
    mode: EraserMode,
    history: Option<StampSource>,
    zoom: f64,
) -> Result<Stroke, PaintError> {
    let target = parse_layer_path(path)
        .and_then(|indices| layer_at(doc, &indices))
        .ok_or(PaintError::NoRasterLayer)?;
    let mut cfg = StrokeConfig {
        auto_erase: false,
        ..cfg
    };
    match mode {
        EraserMode::Brush => {}
        EraserMode::Pencil => cfg.aliased = true,
        EraserMode::Block => {
            cfg = StrokeConfig {
                diameter: block_side_doc_px(zoom),
                square: true,
                opacity: 100,
                flow: 100,
                // A brush preset's grain or Wet Edges would leave the square
                // partly opaque; Block always clears fully.
                grain: 0,
                wet_edges: false,
                // The shape dynamics would scatter the square, shrink it, or
                // stamp it several times; Block is one square of one size.
                scatter: 0,
                count: 1,
                size_jitter: 0,
                ..cfg
            }
        }
    }
    if let Some(source) = history {
        cfg.mode = PaintMode::Normal;
        return Stroke::begin_source(doc, path, cfg, source);
    }
    if erases_to_background(target) {
        cfg.mode = PaintMode::Normal;
        cfg.color = cfg.background;
    } else {
        cfg.mode = PaintMode::Clear;
    }
    Stroke::begin_at(doc, path, cfg)
}

/// The Background Eraser's options bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackgroundEraseOptions {
    pub sampling: Sampling,
    pub limits: Limits,
    /// 0–255: how far a pixel may differ per channel and still be erased.
    pub tolerance: u8,
    /// Never erase what matches the foreground colour.
    pub protect_foreground: bool,
}

impl Default for BackgroundEraseOptions {
    /// Continuous, Contiguous, Tolerance 50 %.
    fn default() -> Self {
        BackgroundEraseOptions {
            sampling: Sampling::Continuous,
            limits: Limits::Contiguous,
            tolerance: 128,
            protect_foreground: false,
        }
    }
}

/// One Background Eraser stroke's state.
pub(crate) struct BackgroundEraser {
    options: BackgroundEraseOptions,
    /// The colour being erased, once fixed (Once, Background Swatch).
    reference: Option<[u8; 4]>,
}

/// The Background Eraser always softens the edge of what it matches: CS6 has
/// no Anti-alias control here, and a hard match would leave a jagged fringe
/// exactly along the subject's edge.
const MATCH_ANTIALIAS: bool = true;

impl BackgroundEraser {
    /// `background` is the Background Swatch reference.
    pub(crate) fn new(options: BackgroundEraseOptions, background: Rgba) -> Self {
        let reference = (options.sampling == Sampling::BackgroundSwatch).then_some([
            background.r,
            background.g,
            background.b,
            background.a,
        ]);
        BackgroundEraser { options, reference }
    }

    /// Erase one dab centred at layer-local `(cx, cy)`; `foreground` is the
    /// colour Protect Foreground Color keeps. Returns the rectangle changed.
    pub(crate) fn dab(
        &mut self,
        img: &mut RgbaImage,
        cfg: &StrokeConfig,
        cx: f32,
        cy: f32,
        foreground: Rgba,
    ) -> Option<PsdRect> {
        let region = dab_bounds(img, cfg, cx, cy)?;
        let centre = (cx.floor() as i32, cy.floor() as i32);
        let inside = centre.0 >= region.left
            && centre.0 < region.right
            && centre.1 >= region.top
            && centre.1 < region.bottom;
        let reference = match self.reference {
            Some(fixed) => fixed,
            None => {
                if !inside {
                    return None;
                }
                let under = img.get(centre.0, centre.1);
                // Sampling ground this stroke already cleared would match
                // transparent black and go on to erase everything dark.
                if under[3] == 0 {
                    return None;
                }
                if self.options.sampling == Sampling::Once {
                    self.reference = Some(under);
                }
                under
            }
        };
        let tolerance = self.options.tolerance;
        let matches = |p: [u8; 4]| match_strength(p, reference, tolerance, MATCH_ANTIALIAS);
        let reach = match self.options.limits {
            Limits::Discontiguous => None,
            _ if !inside => return None,
            limits => Some(reachable(
                img,
                cfg,
                region,
                centre,
                (cx, cy),
                matches,
                limits,
            )),
        };
        let keep = [foreground.r, foreground.g, foreground.b, 255];
        let strength = cfg.opacity as f32 / 100.0 * cfg.flow as f32 / 100.0;
        let rw = region.width() as usize;
        let mut dirty = None;
        for y in region.top..region.bottom {
            for x in region.left..region.right {
                let pixel = img.get(x, y);
                if pixel[3] == 0 {
                    continue;
                }
                if let Some(mask) = &reach {
                    if !mask[(y - region.top) as usize * rw + (x - region.left) as usize] {
                        continue;
                    }
                }
                // Protect Foreground Color wins over the match.
                if self.options.protect_foreground
                    && match_strength(pixel, keep, tolerance, MATCH_ANTIALIAS) > 0.0
                {
                    continue;
                }
                let tip = tip_coverage(cfg, x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let weight = (tip * matches(pixel) * strength).clamp(0.0, 1.0);
                if weight <= 0.0 {
                    continue;
                }
                let alpha = (pixel[3] as f32 * (1.0 - weight)).round() as u8;
                if alpha != pixel[3] {
                    img.set(x, y, [pixel[0], pixel[1], pixel[2], alpha]);
                    dirty = Some(grow(dirty, x, y));
                }
            }
        }
        dirty
    }
}

/// Give `layer` an opaque alpha channel when it has none (a Background just
/// turned into a layer), so erasing can take it away.
pub fn ensure_alpha(layer: &mut Layer) {
    if layer.channels.iter().any(|c| c.id == -1) {
        return;
    }
    let (w, h) = (
        layer.rect.width().max(0) as usize,
        layer.rect.height().max(0) as usize,
    );
    layer.channels.push(Channel {
        id: -1,
        data: Plane::filled(w, h, 1, 255),
    });
}

/// Erase the layer at `path` through a document-sized coverage `mask`
/// (0–255, tiled on the document grid or flat) scaled by `opacity`
/// (0.0–1.0): the Magic Eraser's commit. A layer without alpha or with Lock
/// Transparency is blended toward `background` instead. Returns the document
/// rectangle changed, or `None` when nothing changed.
///
/// Only the layer tiles the mask covers are visited, each reading the mask
/// one row at a time (#1214).
pub fn magic_erase(
    doc: &mut Document,
    path: &str,
    mask: &pictura_core::Coverage,
    opacity: f32,
    background: Rgba,
) -> Option<PsdRect> {
    let opacity = opacity.clamp(0.0, 1.0);
    let canvas = (doc.width as i32, doc.height as i32);
    let (cw, ch) = (canvas.0.max(0) as usize, canvas.1.max(0) as usize);
    if opacity <= 0.0 {
        return None;
    }
    let bounds = mask.covered_bounds(PsdRect {
        left: 0,
        top: 0,
        right: canvas.0,
        bottom: canvas.1,
    })?;
    let (mx0, my0, mx1, my1) = (
        bounds.left as usize,
        bounds.top as usize,
        bounds.right as usize,
        bounds.bottom as usize,
    );
    let layer = layer_at_mut(doc, &parse_layer_path(path)?)?;
    if layer.is_group || layer.adjustment.is_some() {
        return None;
    }
    let fill = erases_to_background(layer).then_some([background.r, background.g, background.b]);
    let rect = layer.rect;
    let (w, h) = (rect.width().max(0) as usize, rect.height().max(0) as usize);
    let region = rect.intersect(PsdRect {
        top: my0 as i32,
        left: mx0 as i32,
        bottom: my1 as i32,
        right: mx1 as i32,
    })?;
    // Each written plane and the value it moves toward: the colour planes
    // toward the background, or the transparency toward empty.
    let targets: Vec<(i16, Option<u8>)> = match fill {
        Some(rgb) => (0..3).map(|c| (c as i16, Some(rgb[c]))).collect(),
        None => vec![(-1, None)],
    };
    let xs = (region.left - rect.left) as usize..(region.right - rect.left) as usize;
    let ys = (region.top - rect.top) as usize..(region.bottom - rect.top) as usize;
    let mut dirty: Option<PsdRect> = None;
    for (id, target) in targets {
        let Some(channel) = layer
            .channels
            .iter_mut()
            .find(|c| c.id == id && c.data.len() == w * h)
        else {
            continue;
        };
        if !channel.data.tile(w, h) {
            continue;
        }
        let mut dense: Vec<usize> = Vec::new();
        let mut changed: Option<PsdRect> = None;
        for t in channel.data.tiles_in(xs.clone(), ys.clone()) {
            let Some((lx, ly, tw, th)) = channel.data.tile_rect(t) else {
                continue;
            };
            let (left, top) = (rect.left + lx as i32, rect.top + ly as i32);
            let (right, bottom) = (left + tw as i32, top + th as i32);
            // A uniform tile may be written whole only when the mask covers
            // all of it and it lies on the canvas; anything else keeps the
            // per-pixel path so pixels outside the canvas are untouched.
            let cover = (left >= 0
                && top >= 0
                && (right as i64) <= cw as i64
                && (bottom as i64) <= ch as i64)
                .then(|| {
                    mask.uniform_over(left as usize..right as usize, top as usize..bottom as usize)
                })
                .flatten();
            let Some(cover) = cover else {
                dense.push(t);
                continue;
            };
            if cover == 0 {
                continue;
            }
            let cover = cover as f32 / 255.0 * opacity;
            let current = match channel.data.tile_state(t) {
                Some(TileState::Empty) => 0,
                Some(TileState::Uniform(v)) => v,
                _ => {
                    dense.push(t);
                    continue;
                }
            };
            let out = match target {
                Some(t) => (current as f32 + (t as f32 - current as f32) * cover).round() as u8,
                None => (current as f32 * (1.0 - cover)).round() as u8,
            };
            if out == current {
                continue;
            }
            if channel.data.set_tile_uniform(t, out) {
                let at = PsdRect {
                    top,
                    left,
                    bottom,
                    right,
                };
                changed = Some(changed.map_or(at, |c| c.union(at)));
            } else {
                dense.push(t);
            }
        }
        let dense_changed = channel
            .data
            .tiles_mut_at(&dense)
            .into_par_iter()
            .filter_map(|mut tile| {
                let mut dirty = None;
                let (left, top) = (rect.left + tile.x as i32, rect.top + tile.y as i32);
                let mut row = vec![0u8; tile.w];
                // The tile's columns on the canvas; the rest read as uncovered.
                let x0 = left.clamp(0, canvas.0);
                let x1 = (left + tile.w as i32).clamp(0, canvas.0);
                for dy in 0..tile.h {
                    let y = top + dy as i32;
                    row.fill(0);
                    if y < 0 || y >= canvas.1 || x0 >= x1 {
                        continue;
                    }
                    mask.read_row(x0, y, &mut row[(x0 - left) as usize..(x1 - left) as usize]);
                    for (dx, v) in tile.row_mut(dy).iter_mut().enumerate() {
                        let cover = row[dx] as f32 / 255.0 * opacity;
                        if cover <= 0.0 {
                            continue;
                        }
                        let out = match target {
                            Some(t) => (*v as f32 + (t as f32 - *v as f32) * cover).round() as u8,
                            None => (*v as f32 * (1.0 - cover)).round() as u8,
                        };
                        if out != *v {
                            *v = out;
                            dirty = Some(grow(dirty, left + dx as i32, y));
                        }
                    }
                }
                dirty
            })
            .reduce_with(PsdRect::union);
        channel.data.settle();
        if let Some(c) = dense_changed {
            changed = Some(changed.map_or(c, |d| d.union(c)));
        }
        if let Some(c) = changed {
            dirty = Some(dirty.map_or(c, |d| d.union(c)));
        }
    }
    dirty
}

/// Soften a binary document-sized `mask` (`w` wide) with a 3×3 box, so an
/// anti-aliased Magic Eraser leaves a smooth edge instead of a stepped one.
///
/// Rows run in parallel from column sums; a row whose 3×3 neighbourhood is
/// all 0 or all 255 has nothing to soften and is copied (#1149).
pub fn antialias_mask(mask: &[u8], w: usize) -> Vec<u8> {
    let mut out = mask.to_vec();
    if w == 0 {
        return out;
    }
    let h = mask.len() / w;
    if h == 0 {
        return out;
    }
    out[..w * h]
        .par_chunks_mut(w)
        .enumerate()
        .for_each(|(y, row)| {
            let (y0, y1) = (y.saturating_sub(1), (y + 1).min(h - 1));
            let near = &mask[y0 * w..(y1 + 1) * w];
            if near.iter().all(|&v| v == 255) || !near.iter().any(|&v| v != 0) {
                return;
            }
            let columns: Vec<u32> = (0..w)
                .map(|x| (y0..=y1).map(|yy| u32::from(mask[yy * w + x])).sum())
                .collect();
            let rows = (y1 - y0 + 1) as u32;
            let own = &mask[y * w..(y + 1) * w];
            for (x, px) in row.iter_mut().enumerate() {
                if own[x] == 0 {
                    continue;
                }
                let (x0, x1) = (x.saturating_sub(1), (x + 1).min(w - 1));
                let sum: u32 = columns[x0..=x1].iter().sum();
                let count = rows * (x1 - x0 + 1) as u32;
                // Only the edge softens; a selected pixel never drops below half.
                *px = (sum / count).max(128) as u8;
            }
        });
    out
}

#[cfg(test)]
mod tests;
