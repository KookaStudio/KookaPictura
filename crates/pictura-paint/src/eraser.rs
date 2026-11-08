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
use pictura_core::mask_ops::{coverage_bounds, TiledMask};
use pictura_core::{layer_transparency_locked, Channel, Document, Layer, PsdRect};
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
    let n = (layer.rect.width().max(0) * layer.rect.height().max(0)) as usize;
    layer.channels.push(Channel {
        id: -1,
        data: vec![255; n].into(),
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
    mask: &TiledMask,
    opacity: f32,
    background: Rgba,
) -> Option<PsdRect> {
    let opacity = opacity.clamp(0.0, 1.0);
    let canvas = (doc.width as i32, doc.height as i32);
    let (cw, ch) = (canvas.0.max(0) as usize, canvas.1.max(0) as usize);
    if opacity <= 0.0 || mask.len() < cw * ch {
        return None;
    }
    let (mx0, my0, mx1, my1) = coverage_bounds(mask, cw, ch)?;
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
        let covered: Vec<usize> = channel
            .data
            .tiles_in(xs.clone(), ys.clone())
            .into_iter()
            .filter(|&t| {
                channel.data.tile_rect(t).is_some_and(|(lx, ly, tw, th)| {
                    // The tile's part on the canvas, in document pixels.
                    let x0 = (rect.left as i64 + lx as i64).clamp(0, cw as i64) as usize;
                    let y0 = (rect.top as i64 + ly as i64).clamp(0, ch as i64) as usize;
                    let x1 = (rect.left as i64 + (lx + tw) as i64).clamp(0, cw as i64) as usize;
                    let y1 = (rect.top as i64 + (ly + th) as i64).clamp(0, ch as i64) as usize;
                    x0 < x1 && y0 < y1 && mask.uniform_over(x0..x1, y0..y1) != Some(0)
                })
            })
            .collect();
        let changed = channel
            .data
            .tiles_mut_at(&covered)
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
                    mask.read_rect_into(
                        x0 as usize..x1 as usize,
                        y as usize..y as usize + 1,
                        cw,
                        &mut row[(x0 - left) as usize..(x1 - left) as usize],
                    );
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
mod tests {
    use super::*;
    use crate::stamp::layer_surface;
    use crate::stroke::layer_rgba;
    use crate::{Rgba, StrokeSample};
    use pictura_core::{BitDepth, Channel, ColorMode, LockFlags, PsdRect};

    const GREY: [u8; 4] = [90, 90, 90, 255];
    const TEAL: Rgba = Rgba {
        r: 0,
        g: 128,
        b: 128,
        a: 255,
    };

    fn doc(background: bool) -> Document {
        let (w, h) = (40, 20);
        let mut doc = Document::new(w as u32, h as u32, ColorMode::Rgb, BitDepth::Eight);
        let ids: &[(i16, usize)] = if background {
            &[(0, 0), (1, 1), (2, 2)]
        } else {
            &[(0, 0), (1, 1), (2, 2), (-1, 3)]
        };
        doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: h,
                right: w,
            },
            channels: ids
                .iter()
                .map(|&(id, c)| Channel {
                    id,
                    data: vec![GREY[c]; (w * h) as usize].into(),
                })
                .collect(),
            background,
            ..Default::default()
        }]);
        doc
    }

    fn erase(doc: &Document, mode: EraserMode, history: Option<StampSource>) -> Document {
        let cfg = StrokeConfig {
            diameter: 8,
            background: TEAL,
            ..StrokeConfig::default()
        };
        let mut stroke = begin_erase(doc, "0", cfg, mode, history, 1.0).expect("begin");
        let mut out = doc.clone();
        for x in [8.0, 16.0, 24.0] {
            stroke.sample(
                &mut out,
                StrokeSample {
                    x,
                    y: 10.0,
                    pressure: 1.0,
                },
            );
        }
        stroke.finish().expect("erased");
        out
    }

    fn pixel(doc: &Document, x: i32, y: i32) -> [u8; 4] {
        layer_rgba(&doc.layers[0]).get(x, y)
    }

    /// The layer's pixels the stroke cleared.
    fn erased_pixels(doc: &Document) -> usize {
        let layer = layer_rgba(&doc.layers[0]);
        (0..layer.height)
            .flat_map(|y| (0..layer.width).map(move |x| (x, y)))
            .filter(|&(x, y)| layer.get(x, y)[3] == 0)
            .count()
    }

    #[test]
    fn a_layer_erases_to_transparency_and_the_background_to_the_background_colour() {
        let layer = erase(&doc(false), EraserMode::Brush, None);
        assert_eq!(pixel(&layer, 16, 10)[3], 0, "the stroke left alpha behind");
        assert_eq!(pixel(&layer, 16, 18), GREY, "erased outside the tip");

        let background = erase(&doc(true), EraserMode::Brush, None);
        assert_eq!(pixel(&background, 16, 10), [0, 128, 128, 255]);
    }

    #[test]
    fn a_transparency_lock_paints_the_background_colour() {
        let mut locked = doc(false);
        locked.layers[0].lock = LockFlags::default().with(LockFlags::TRANSPARENCY, true);
        assert_eq!(
            pixel(&erase(&locked, EraserMode::Brush, None), 16, 10),
            [0, 128, 128, 255]
        );
    }

    #[test]
    fn half_opacity_leaves_half_the_alpha() {
        let cfg = StrokeConfig {
            diameter: 8,
            opacity: 50,
            ..StrokeConfig::default()
        };
        let mut out = doc(false);
        let mut stroke = begin_erase(&out, "0", cfg, EraserMode::Brush, None, 1.0).unwrap();
        stroke.sample(
            &mut out,
            StrokeSample {
                x: 16.0,
                y: 10.0,
                pressure: 1.0,
            },
        );
        stroke.finish().unwrap();
        let a = pixel(&out, 16, 10)[3];
        assert!((a as i32 - 128).abs() <= 1, "alpha {a}");
    }

    #[test]
    fn block_erases_a_hard_square_at_full_strength() {
        // Settings inherited from a pencil or wet-edged preset must not leak in.
        let cfg = StrokeConfig {
            opacity: 10,
            flow: 50,
            grain: 55,
            wet_edges: true,
            ..StrokeConfig::default()
        };
        let mut out = doc(false);
        let mut stroke = begin_erase(&out, "0", cfg, EraserMode::Block, None, 1.0).unwrap();
        stroke.sample(
            &mut out,
            StrokeSample {
                x: 20.0,
                y: 10.0,
                pressure: 1.0,
            },
        );
        stroke.finish().unwrap();
        // The square's corner is erased; a round 16 px tip would miss it.
        assert_eq!(pixel(&out, 13, 3)[3], 0);
        assert_eq!(pixel(&out, 20, 10)[3], 0);
        assert_eq!(pixel(&out, 11, 10)[3], 255);

        // A half-pixel dab covers the same square: it is exactly 16 px wide,
        // not one wider on the far side.
        let mut out = doc(false);
        let mut stroke = begin_erase(&out, "0", cfg, EraserMode::Block, None, 1.0).unwrap();
        stroke.sample(
            &mut out,
            StrokeSample {
                x: 20.5,
                y: 10.5,
                pressure: 1.0,
            },
        );
        stroke.finish().unwrap();
        assert_eq!(pixel(&out, 13, 3)[3], 0, "the square's corner");
        assert_eq!(pixel(&out, 27, 17)[3], 0, "the square's far corner");
        assert_eq!(pixel(&out, 28, 10)[3], 255, "the square is 16 px wide");
        assert_eq!(pixel(&out, 20, 18)[3], 255, "the square is 16 px tall");
        assert_eq!(erased_pixels(&out), 16 * 16, "exactly one square");
    }

    #[test]
    fn the_block_square_scales_with_the_zoom() {
        // The Block is 16 screen pixels wide, so its document side is
        // 16 / zoom.
        assert_eq!(block_side_doc_px(1.0), 16);
        assert_eq!(block_side_doc_px(4.0), 4);
        assert_eq!(block_side_doc_px(0.25), 64);
        assert_eq!(block_side_doc_px(32.0), 1, "a document pixel is the floor");
        assert_eq!(block_side_doc_px(0.0), 16, "a bad zoom is 100 %");
        assert_eq!(block_side_doc_px(f64::NAN), 16, "a bad zoom is 100 %");

        // At 400 % the block is 4 document pixels a side, so it covers 16 of
        // them whatever the dab's position in its pixel.
        let mut out = doc(false);
        let mut stroke = begin_erase(
            &out,
            "0",
            StrokeConfig::default(),
            EraserMode::Block,
            None,
            4.0,
        )
        .unwrap();
        stroke.sample(
            &mut out,
            StrokeSample {
                x: 20.5,
                y: 10.5,
                pressure: 1.0,
            },
        );
        stroke.finish().unwrap();
        assert_eq!(erased_pixels(&out), 4 * 4, "4 document px a side at 400 %");

        // At 3200 % the block is one document pixel, so it covers one: the
        // inclusive boundary would have covered two.
        let mut out = doc(false);
        let mut stroke = begin_erase(
            &out,
            "0",
            StrokeConfig::default(),
            EraserMode::Block,
            None,
            32.0,
        )
        .unwrap();
        stroke.sample(
            &mut out,
            StrokeSample {
                x: 20.5,
                y: 10.5,
                pressure: 1.0,
            },
        );
        stroke.finish().unwrap();
        assert_eq!(erased_pixels(&out), 1, "one document px at 3200 %");

        // The brush preset's shape dynamics do not apply: the Block is one
        // square of one size.
        let mut out = doc(false);
        let mut stroke = begin_erase(
            &out,
            "0",
            StrokeConfig {
                scatter: 120,
                count: 3,
                size_jitter: 60,
                ..StrokeConfig::default()
            },
            EraserMode::Block,
            None,
            1.0,
        )
        .unwrap();
        stroke.sample(
            &mut out,
            StrokeSample {
                x: 20.5,
                y: 10.5,
                pressure: 1.0,
            },
        );
        stroke.finish().unwrap();
        assert_eq!(erased_pixels(&out), 16 * 16, "the dynamics do not apply");
    }

    #[test]
    fn pencil_erases_without_a_soft_edge() {
        let out = erase(&doc(false), EraserMode::Pencil, None);
        let layer = layer_rgba(&out.layers[0]);
        assert!(layer.data.iter().all(|p| p[3] == 0 || p[3] == 255));
    }

    #[test]
    fn erase_to_history_paints_the_source_back() {
        let mut past = doc(false);
        for c in &mut past.layers[0].channels {
            if c.id == 0 {
                c.data.fill(250);
            }
        }
        let source = StampSource::new(layer_surface(&past, "0").unwrap(), (0, 0));
        let out = erase(&doc(false), EraserMode::Brush, Some(source));
        assert_eq!(pixel(&out, 16, 10), [250, 90, 90, 255]);
    }

    fn split() -> Document {
        // Sky (blue) on the left, a subject (yellow) from x = 20.
        let mut d = doc(false);
        let w = 40;
        for c in &mut d.layers[0].channels {
            let (sky, subject) = match c.id {
                0 => (30, 220),
                1 => (120, 200),
                2 => (220, 40),
                _ => (255, 255),
            };
            for (i, v) in c.data.iter_mut().enumerate() {
                *v = if i % w < 20 { sky } else { subject };
            }
        }
        d
    }

    fn background_erase(doc: &Document, options: BackgroundEraseOptions, x: f32) -> Document {
        let cfg = StrokeConfig {
            diameter: 16,
            ..StrokeConfig::default()
        };
        let mut stroke =
            Stroke::begin_kind(doc, "0", cfg, crate::StrokeKind::BackgroundErase(options))
                .expect("begin");
        let mut out = doc.clone();
        stroke.sample(
            &mut out,
            StrokeSample {
                x,
                y: 10.0,
                pressure: 1.0,
            },
        );
        stroke.finish();
        out
    }

    #[test]
    fn the_background_eraser_cuts_out_what_it_sampled() {
        let options = BackgroundEraseOptions {
            tolerance: 40,
            ..BackgroundEraseOptions::default()
        };
        // Crosshair on the sky, close enough that the dab overlaps the subject.
        let out = background_erase(&split(), options, 16.0);
        assert_eq!(pixel(&out, 14, 10)[3], 0, "the sampled sky survived");
        assert_eq!(pixel(&out, 22, 10)[3], 255, "the subject was erased too");
    }

    #[test]
    fn protect_foreground_keeps_its_colour() {
        let options = BackgroundEraseOptions {
            sampling: Sampling::BackgroundSwatch,
            limits: Limits::Discontiguous,
            tolerance: 40,
            protect_foreground: true,
        };
        let sky = Rgba {
            r: 30,
            g: 120,
            b: 220,
            a: 255,
        };
        let cfg = StrokeConfig {
            diameter: 16,
            color: sky,
            background: sky,
            ..StrokeConfig::default()
        };
        let kind = crate::StrokeKind::BackgroundErase(options);
        let mut out = split();
        let mut stroke = Stroke::begin_kind(&out, "0", cfg, kind).unwrap();
        stroke.sample(
            &mut out,
            StrokeSample {
                x: 8.0,
                y: 10.0,
                pressure: 1.0,
            },
        );
        assert!(stroke.finish().is_none(), "a protected colour was erased");
    }

    #[test]
    fn the_magic_eraser_takes_the_masked_region_scaled_by_opacity() {
        let mut d = doc(false);
        let mut mask = vec![0u8; 40 * 20];
        mask[10 * 40 + 5] = 255;
        let dirty = magic_erase(&mut d, "0", &TiledMask::from(mask), 0.5, TEAL).expect("erased");
        assert_eq!((dirty.left, dirty.top), (5, 10));
        let half = pixel(&d, 5, 10);
        assert!((half[3] as i32 - 128).abs() <= 1, "alpha {}", half[3]);
        assert_eq!(&half[..3], &GREY[..3], "erasing changed the colour");
        assert_eq!(pixel(&d, 6, 10)[3], 255);
    }

    #[test]
    fn the_magic_eraser_fills_a_locked_layer_with_the_background_colour() {
        let mut d = doc(true);
        let mask = vec![255u8; 40 * 20];
        magic_erase(&mut d, "0", &TiledMask::from(mask), 1.0, TEAL).expect("filled");
        assert_eq!(pixel(&d, 3, 3), [0, 128, 128, 255]);
        let mut layer = d.layers[0].clone();
        layer.background = false;
        ensure_alpha(&mut layer);
        assert!(
            !erases_to_background(&layer),
            "an alpha channel was not added"
        );
    }

    #[test]
    fn an_antialiased_mask_softens_only_its_edge() {
        let mut mask = vec![0u8; 9 * 9];
        for y in 2..7 {
            for x in 2..7 {
                mask[y * 9 + x] = 255;
            }
        }
        let soft = antialias_mask(&mask, 9);
        assert_eq!(soft[4 * 9 + 4], 255);
        assert!(soft[2 * 9 + 2] < 255 && soft[2 * 9 + 2] >= 128);
        assert_eq!(soft[0], 0);
    }

    /// The per-pixel 3×3 box the parallel one replaced.
    fn antialias_reference(mask: &[u8], w: usize) -> Vec<u8> {
        let h = mask.len() / w;
        let mut out = mask.to_vec();
        for y in 0..h {
            for x in 0..w {
                let (mut sum, mut count) = (0u32, 0u32);
                for yy in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                    for xx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                        sum += u32::from(mask[yy * w + xx]);
                        count += 1;
                    }
                }
                if mask[y * w + x] > 0 {
                    out[y * w + x] = (sum / count).max(128) as u8;
                }
            }
        }
        out
    }

    #[test]
    fn the_parallel_antialias_matches_the_per_pixel_box() {
        let mut seed = 0x2545_f491_u32;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        for (w, h) in [(1, 1), (1, 7), (7, 1), (2, 2), (37, 23)] {
            // Solid runs, gaps, and partial coverage, plus a ragged tail.
            let mut mask: Vec<u8> = (0..w * h)
                .map(|_| match next() % 5 {
                    0 => 0,
                    1 => (next() % 256) as u8,
                    _ => 255,
                })
                .collect();
            for v in &mut mask[..w.min(w * h)] {
                *v = 255;
            }
            // A tail shorter than a row is not part of the mask.
            let tail: &[u8] = if w > 1 { &[77] } else { &[] };
            mask.extend_from_slice(tail);
            assert_eq!(
                antialias_mask(&mask, w),
                antialias_reference(&mask[..w * h], w)
                    .into_iter()
                    .chain(tail.iter().copied())
                    .collect::<Vec<_>>(),
                "{w}x{h}"
            );
        }
        assert_eq!(antialias_mask(&[5, 6], 0), vec![5, 6]);
    }

    /// The Magic Eraser as it was before #1214: the wand over the whole
    /// interleaved surface, a dense mask, and a per-pixel erase.
    fn magic_erase_reference(
        d: &mut Document,
        seed: (i32, i32),
        tolerance: u8,
        contiguous: bool,
        soften: bool,
    ) {
        let surface = layer_surface(d, "0").unwrap();
        let (w, h) = (d.width as usize, d.height as usize);
        let at = |p: usize| surface.data[p];
        let target = at(seed.1 as usize * w + seed.0 as usize);
        let near = |p: usize| (0..3).all(|k| at(p)[k].abs_diff(target[k]) <= tolerance);
        let mut flood: Vec<u8> = (0..w * h).map(|p| u8::from(near(p)) * 255).collect();
        if contiguous {
            flood = pictura_core::mask_ops::flood_from(
                w,
                h,
                seed.1 as usize * w + seed.0 as usize,
                |_, to| near(to),
            )
            .unwrap();
        }
        let mask = if soften {
            antialias_mask(&flood, w)
        } else {
            flood
        };
        let layer = &mut d.layers[0];
        let rect = layer.rect;
        let lw = rect.width() as usize;
        let alpha = layer.channels.iter().position(|c| c.id == -1).unwrap();
        let data = layer.channels[alpha].data.as_mut_slice();
        for y in rect.top.max(0)..rect.bottom.min(h as i32) {
            for x in rect.left.max(0)..rect.right.min(w as i32) {
                let cover = mask[y as usize * w + x as usize] as f32 / 255.0 * 0.8;
                if cover <= 0.0 {
                    continue;
                }
                let i = (y - rect.top) as usize * lw + (x - rect.left) as usize;
                data[i] = (data[i] as f32 * (1.0 - cover)).round() as u8;
            }
        }
    }

    #[test]
    fn magic_erase_tiled_matches_dense() {
        use crate::bucket::{antialias, magic_flood, BucketSource};
        let (w, h) = (600i32, 420i32);
        let mut base = crate::fill::tests::doc_at((w, h), (-20, 15), [90, 90, 90, 255], false);
        let n = base.layers[0].rect.width() * base.layers[0].rect.height();
        let lw = base.layers[0].rect.width();
        let mut state = 0x77u64;
        let cells: Vec<u8> = (0..64)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                [90u8, 92, 200, 10][(state % 4) as usize]
            })
            .collect();
        for c in &mut base.layers[0].channels[..3] {
            let data: Vec<u8> = (0..n)
                .map(|i| cells[(((i / lw) / 70) * 8 + (i % lw) / 90) as usize % 64])
                .collect();
            c.data = data.into();
        }
        let canvas = PsdRect {
            top: 0,
            left: 0,
            bottom: h,
            right: w,
        };
        for contiguous in [true, false] {
            for soften in [true, false] {
                let mut expected = base.clone();
                magic_erase_reference(&mut expected, (100, 100), 4, contiguous, soften);
                let mut got = base.clone();
                let size = (w as u32, h as u32);
                let source = BucketSource::layer(&got, "0").unwrap();
                let flood = magic_flood(source, size, (100, 100), 4, contiguous).unwrap();
                let mask = if soften {
                    antialias(flood, size, canvas, None)
                } else {
                    flood
                };
                magic_erase(&mut got, "0", &mask, 0.8, TEAL);
                for (a, b) in got.layers[0]
                    .channels
                    .iter()
                    .zip(&expected.layers[0].channels)
                {
                    assert!(a.data == b.data, "channel {} {contiguous} {soften}", a.id);
                }
            }
        }
    }
}
