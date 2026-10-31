//! Stroke engine: dab coverage accumulation and per-pixel compositing.

use crate::art_history::{layer_local, ArtHistoryBrush, ArtHistoryOptions};
use crate::eraser::{BackgroundEraseOptions, BackgroundEraser};
use crate::focus::{FocusBrush, FocusOptions};
use crate::healing::RgbaImage;
use crate::mixer::{MixerBrush, MixerOptions};
use crate::replace::{ColorReplacer, ReplaceOptions};
use crate::smudge::{SmudgeBrush, SmudgeOptions};
use crate::spacing::DabPlacer;
use crate::stamp::StampSource;
use crate::tip::TipParams;
use crate::tone::{ToneBrush, ToneOptions};
use crate::{PaintMode, Rgba, StrokeConfig, StrokeSample};
use pictura_core::{
    layer_pixel_locked, layer_transparency_locked, BitDepth, Document, Layer, PsdRect,
};
use raster::{plane_slices, split_bands, write_band, BandPlanes, DabRaster};
use rayon::prelude::*;
use std::sync::Arc;

/// Deterministic stroke seed; Dissolve randomness must not vary between runs.
const STROKE_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

#[derive(Debug)]
pub enum PaintError {
    EmptyDocument,
    NoRasterLayer,
    /// The target layer's pixel or transparency lock refuses the stroke.
    Locked,
    /// The tool works on 8-bit documents only.
    UnsupportedDepth,
}

/// What a stroke's dabs do. `Paint` accumulates coverage and composites the
/// paint colour (Brush, Pencil); the others read the layer at every dab and
/// write into it directly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrokeKind {
    Paint,
    /// Color Replacement: `cfg.color` replaces what matches the sample;
    /// `cfg.background` is the Background Swatch reference.
    Replace(ReplaceOptions),
    /// Mixer Brush, starting with `reservoir` on the brush.
    Mixer {
        options: MixerOptions,
        reservoir: Rgba,
    },
    /// Background Eraser: erases what matches the sample to transparency,
    /// sparing `cfg.color` when protected; `cfg.background` is the Background
    /// Swatch reference. Overrides Lock Transparency.
    BackgroundErase(BackgroundEraseOptions),
}

enum DabEngine {
    Replace(ColorReplacer),
    Mixer(MixerBrush),
    ArtHistory(ArtHistoryBrush),
    BackgroundErase(BackgroundEraser),
    Focus(FocusBrush),
    Smudge(SmudgeBrush),
    Tone(ToneBrush),
}

/// A per-dab engine and the layer pixels it edits in place.
struct PerDab {
    engine: DabEngine,
    pixels: RgbaImage,
}

pub struct StrokeOutcome {
    pub document: Document,
    pub dirty: PsdRect,
}

/// Tile edge of the pre-stroke pixels a stroke keeps: a dab saves the tiles it
/// is about to write, the first time it reaches them.
const SAVED_TILE: i32 = 64;

/// The target layer's pre-stroke pixels, saved a tile at a time just before a
/// dab first writes the tile. The planes are layer-sized but zero-filled by the
/// allocator, so only the saved tiles ever take memory.
struct Saved {
    planes: [Option<Vec<u8>>; 4],
    tiles: Vec<bool>,
    columns: i32,
    /// The target layer's bounds when the stroke began.
    rect: PsdRect,
    width: i32,
    height: i32,
}

impl Saved {
    fn new(layer: &Layer, ch: PlaneIndex) -> Self {
        let width = layer.rect.width().max(0);
        let height = layer.rect.height().max(0);
        let columns = (width + SAVED_TILE - 1) / SAVED_TILE;
        let rows = (height + SAVED_TILE - 1) / SAVED_TILE;
        let planes = ch.map(|k| {
            k.and_then(|k| layer.channels.get(k))
                .map(|c| vec![0u8; c.data.len()])
        });
        Self {
            planes,
            tiles: vec![false; (columns * rows) as usize],
            columns,
            rect: layer.rect,
            width,
            height,
        }
    }

    /// Save every unsaved tile meeting the layer-local box `[x0, x1] × [y0, y1]`
    /// (inclusive, clamped to the layer) from `layer`, which still holds them
    /// unpainted.
    fn save(&mut self, layer: &Layer, ch: PlaneIndex, x0: i32, y0: i32, x1: i32, y1: i32) {
        let (x0, y0) = (x0.max(0), y0.max(0));
        let (x1, y1) = (x1.min(self.width - 1), y1.min(self.height - 1));
        if x1 < x0 || y1 < y0 {
            return;
        }
        let w = self.width as usize;
        for ty in y0 / SAVED_TILE..=y1 / SAVED_TILE {
            for tx in x0 / SAVED_TILE..=x1 / SAVED_TILE {
                let t = (ty * self.columns + tx) as usize;
                if self.tiles[t] {
                    continue;
                }
                self.tiles[t] = true;
                let (left, top) = (tx * SAVED_TILE, ty * SAVED_TILE);
                let right = (left + SAVED_TILE).min(self.width) as usize;
                let bottom = (top + SAVED_TILE).min(self.height) as usize;
                for (k, plane) in self.planes.iter_mut().enumerate() {
                    let (Some(plane), Some(src)) =
                        (plane.as_mut(), ch[k].and_then(|k| layer.channels.get(k)))
                    else {
                        continue;
                    };
                    for y in top as usize..bottom {
                        // Read the row, not the whole plane: a banded plane
                        // must not be materialized just to save one tile row.
                        let Some(row) = src.data.row(y, w) else {
                            continue;
                        };
                        let (a, b) = (y * w + left as usize, (y * w + right).min(plane.len()));
                        let from = (left as usize).min(row.len());
                        let to = right.min(row.len());
                        if a < b && from < to {
                            let n = (to - from).min(b - a);
                            plane[a..a + n].copy_from_slice(&row[from..from + n]);
                        }
                    }
                }
            }
        }
    }

    /// Whether `layer` still has the planes and bounds this stroke began on. A
    /// document restructured or a target moved under a live stroke is refused
    /// rather than painted at stale coordinates or indexed out of bounds.
    fn fits(&self, layer: &Layer, ch: PlaneIndex) -> bool {
        layer.rect == self.rect
            && self
                .planes
                .iter()
                .zip(ch)
                .all(|(plane, k)| match (plane, k) {
                    (Some(plane), Some(k)) => layer
                        .channels
                        .get(k)
                        .is_some_and(|c| c.data.len() == plane.len()),
                    (None, None) => true,
                    _ => false,
                })
    }

    fn planes(&self) -> [Option<&[u8]>; 4] {
        [0, 1, 2, 3].map(|k| self.planes[k].as_deref())
    }

    /// Write every saved tile back into `layer`.
    fn restore(&self, layer: &mut Layer, ch: PlaneIndex) {
        let w = self.width as usize;
        for (t, _) in self.tiles.iter().enumerate().filter(|(_, saved)| **saved) {
            let (tx, ty) = (t as i32 % self.columns, t as i32 / self.columns);
            let (left, top) = (tx * SAVED_TILE, ty * SAVED_TILE);
            let right = (left + SAVED_TILE).min(self.width) as usize;
            let bottom = (top + SAVED_TILE).min(self.height) as usize;
            for (k, plane) in self.planes.iter().enumerate() {
                let (Some(plane), Some(k)) = (plane.as_ref(), ch[k]) else {
                    continue;
                };
                let Some(dst) = layer.channels.get_mut(k) else {
                    continue;
                };
                for y in top as usize..bottom {
                    let (a, b) = (y * w + left as usize, (y * w + right).min(plane.len()));
                    if a < b {
                        dst.data[a..b].copy_from_slice(&plane[a..b]);
                    }
                }
            }
        }
    }
}

/// A live stroke. It paints the document handed to each call in place; the
/// pixels it is about to change are kept in `saved`, which every dab blends
/// against so the stroke's result does not depend on its own earlier dabs.
pub struct Stroke {
    saved: Saved,
    /// The target layer's transparency lock, read once at the start.
    locked: bool,
    cfg: StrokeConfig,
    tip: TipParams,
    placer: DabPlacer,
    scratch: Vec<u8>,
    applied: Vec<u8>,
    dirty: Option<PsdRect>,
    dab_dirty: Option<PsdRect>,
    layer_path: Vec<usize>,
    rect: PsdRect,
    /// Positions of the R, G, B and alpha planes in the layer's channel list.
    /// Fixed for the life of the stroke: re-scanning that list per changed
    /// pixel is the bulk of a large dab's raster cost.
    ch: PlaneIndex,
    paint: Rgba,
    rng: u64,
    started: bool,
    painted: bool,
    per_dab: Option<PerDab>,
    source: Option<StampSource>,
    /// Raster every dab on one thread: the reference the banded raster is
    /// held to.
    serial: bool,
}

impl Stroke {
    pub fn begin_at(doc: &Document, path: &str, cfg: StrokeConfig) -> Result<Stroke, PaintError> {
        Stroke::begin_kind(doc, path, cfg, StrokeKind::Paint)
    }

    /// Start a stroke whose dabs behave as `kind`.
    pub fn begin_kind(
        doc: &Document,
        path: &str,
        cfg: StrokeConfig,
        kind: StrokeKind,
    ) -> Result<Stroke, PaintError> {
        if doc.layers.is_empty() {
            return Err(PaintError::EmptyDocument);
        }
        let indices = parse_layer_path(path).ok_or(PaintError::NoRasterLayer)?;
        let cfg = cfg.sanitized();
        let target = layer_at(doc, &indices).ok_or(PaintError::NoRasterLayer)?;
        if target.is_group || target.adjustment.is_some() {
            return Err(PaintError::NoRasterLayer);
        }
        if layer_pixel_locked(target) {
            return Err(PaintError::Locked);
        }
        if layer_transparency_locked(target) && (cfg.mode == PaintMode::Clear || cfg.auto_erase) {
            return Err(PaintError::Locked);
        }
        let rect = target.rect;
        let w = rect.width().max(0) as usize;
        let h = rect.height().max(0) as usize;
        // ponytail: layer-sized coverage buffer; move to tiled/sparse coverage only if huge layers matter.
        let scratch = vec![0u8; w * h];
        let applied = vec![0u8; w * h];
        let per_dab =
            match kind {
                StrokeKind::Paint => None,
                StrokeKind::Replace(options) => Some(DabEngine::Replace(ColorReplacer::new(
                    w * h,
                    options,
                    cfg.background,
                ))),
                StrokeKind::Mixer { options, reservoir } => Some(DabEngine::Mixer(
                    MixerBrush::new(options, reservoir, layer_transparency_locked(target)),
                )),
                StrokeKind::BackgroundErase(options) => Some(DabEngine::BackgroundErase(
                    BackgroundEraser::new(options, cfg.background),
                )),
            }
            .map(|engine| PerDab {
                engine,
                pixels: layer_rgba(target),
            });
        let ch = plane_index(target);
        Ok(Stroke {
            saved: Saved::new(target, ch),
            locked: layer_transparency_locked(target),
            cfg,
            tip: TipParams::new(&cfg),
            placer: DabPlacer::new(cfg.spacing, cfg.diameter as f32),
            scratch,
            applied,
            dirty: None,
            dab_dirty: None,
            layer_path: indices,
            rect,
            ch,
            paint: cfg.color,
            rng: STROKE_SEED,
            started: false,
            painted: false,
            per_dab,
            source: None,
            serial: false,
        })
    }

    /// Start a Paint stroke whose colour at each pixel comes from `source`
    /// (Clone Stamp, Pattern Stamp, History Brush) instead of `cfg.color`.
    pub fn begin_source(
        doc: &Document,
        path: &str,
        cfg: StrokeConfig,
        source: StampSource,
    ) -> Result<Stroke, PaintError> {
        let mut stroke = Stroke::begin_kind(doc, path, cfg, StrokeKind::Paint)?;
        stroke.source = Some(source);
        Ok(stroke)
    }

    /// Start an Art History Brush stroke painting stylized strokes coloured
    /// from `source`, a document-space image of the source state's layer.
    /// 8-bit documents only.
    pub fn begin_art_history(
        doc: &Document,
        path: &str,
        cfg: StrokeConfig,
        source: Arc<RgbaImage>,
        options: ArtHistoryOptions,
    ) -> Result<Stroke, PaintError> {
        if doc.depth != BitDepth::Eight {
            return Err(PaintError::UnsupportedDepth);
        }
        let mut stroke = Stroke::begin_kind(doc, path, cfg, StrokeKind::Paint)?;
        let target = layer_at(doc, &stroke.layer_path).ok_or(PaintError::NoRasterLayer)?;
        let brush = ArtHistoryBrush::new(
            options,
            layer_local(&source, stroke.rect),
            layer_transparency_locked(target),
        );
        stroke.per_dab = Some(PerDab {
            engine: DabEngine::ArtHistory(brush),
            pixels: layer_rgba(target),
        });
        Ok(stroke)
    }

    /// Start a Blur or Sharpen stroke. `sampled` is the layer-local composite
    /// Sample All Layers reads; `None` reads the layer. 8-bit documents only.
    pub fn begin_focus(
        doc: &Document,
        path: &str,
        cfg: StrokeConfig,
        options: FocusOptions,
        sampled: Option<RgbaImage>,
    ) -> Result<Stroke, PaintError> {
        Stroke::begin_retouch(doc, path, cfg, |_, target| {
            DabEngine::Focus(FocusBrush::new(
                options,
                sampled,
                layer_transparency_locked(target),
            ))
        })
    }

    /// Start a Mixer Brush stroke. `sampled` as for [`Stroke::begin_focus`].
    pub fn begin_mixer(
        doc: &Document,
        path: &str,
        cfg: StrokeConfig,
        options: MixerOptions,
        reservoir: Rgba,
        sampled: Option<RgbaImage>,
    ) -> Result<Stroke, PaintError> {
        let mut stroke =
            Stroke::begin_kind(doc, path, cfg, StrokeKind::Mixer { options, reservoir })?;
        if let Some(PerDab {
            engine: DabEngine::Mixer(mixer),
            ..
        }) = stroke.per_dab.as_mut()
        {
            mixer.sampled = sampled;
        }
        Ok(stroke)
    }

    /// Start a Smudge stroke; `cfg.color` is the Finger Painting colour.
    /// `sampled` as for [`Stroke::begin_focus`]. 8-bit documents only.
    pub fn begin_smudge(
        doc: &Document,
        path: &str,
        cfg: StrokeConfig,
        options: SmudgeOptions,
        sampled: Option<RgbaImage>,
    ) -> Result<Stroke, PaintError> {
        Stroke::begin_retouch(doc, path, cfg, |_, target| {
            let paint = [cfg.color.r, cfg.color.g, cfg.color.b, cfg.color.a];
            let locked = layer_transparency_locked(target);
            DabEngine::Smudge(SmudgeBrush::new(options, sampled, locked, paint))
        })
    }

    /// Start a Dodge, Burn, or Sponge stroke. 8-bit documents only.
    pub fn begin_tone(
        doc: &Document,
        path: &str,
        cfg: StrokeConfig,
        options: ToneOptions,
    ) -> Result<Stroke, PaintError> {
        Stroke::begin_retouch(doc, path, cfg, |_, _| {
            DabEngine::Tone(ToneBrush::new(options))
        })
    }

    /// A stroke whose dabs run the per-dab `engine` built for the target layer.
    fn begin_retouch(
        doc: &Document,
        path: &str,
        cfg: StrokeConfig,
        engine: impl FnOnce(&Stroke, &Layer) -> DabEngine,
    ) -> Result<Stroke, PaintError> {
        if doc.depth != BitDepth::Eight {
            return Err(PaintError::UnsupportedDepth);
        }
        let mut stroke = Stroke::begin_kind(doc, path, cfg, StrokeKind::Paint)?;
        let target = layer_at(doc, &stroke.layer_path).ok_or(PaintError::NoRasterLayer)?;
        stroke.per_dab = Some(PerDab {
            engine: engine(&stroke, target),
            pixels: layer_rgba(target),
        });
        Ok(stroke)
    }

    /// Feed one sample, painting `doc` (the document the stroke began on) in
    /// place. Returns `true` when any pixel's accumulated coverage changed.
    pub fn sample(&mut self, doc: &mut Document, s: StrokeSample) -> bool {
        let cfg = self.cfg;
        let w = self.rect.width();
        let h = self.rect.height();
        if w <= 0 || h <= 0 {
            return false;
        }
        let (w, h) = (w as usize, h as usize);
        let sx = s.x - self.rect.left as f32;
        let sy = s.y - self.rect.top as f32;

        if !self.started {
            self.started = true;
            if cfg.auto_erase && self.pixel_rgb_matches(doc, sx, sy, cfg.color) {
                self.paint = cfg.background;
            }
        }

        let mut dabs = Vec::new();
        self.placer.feed(sx, sy, &mut dabs);
        if self.per_dab.is_some() {
            return self.apply_per_dab(doc, &dabs);
        }

        let flow = cfg.flow as f32 / 100.0;
        let mut changed = false;
        // Dissolve draws one random roll per painted pixel in raster order, so
        // its dabs stay on one thread; every other pixel depends on nothing but
        // its own inputs, so the bands give the serial raster's bytes.
        let serial = self.serial || cfg.mode == PaintMode::Dissolve;

        // Place and jitter every dab of this step first, so the raster below can
        // split `self` and read the base layer while writing the working one.
        let mut placed: Vec<(f32, f32, Option<StrokeConfig>)> = Vec::new();
        for (dab_x, dab_y) in dabs {
            for _ in 0..cfg.count.max(1) {
                placed.push(self.jittered(dab_x, dab_y));
            }
        }

        // Split the stroke into its disjoint parts so the saved pixels can be
        // read while the layer is written, with the layer path, the channel
        // list and the saved planes all resolved once per sample.
        let Stroke {
            saved,
            locked,
            layer_path,
            scratch,
            applied,
            tip,
            paint,
            rng,
            dirty,
            dab_dirty,
            ch,
            painted,
            rect,
            source,
            ..
        } = self;
        let path: &[usize] = layer_path;
        let Some(work_layer) = layer_at_mut(doc, path).filter(|l| saved.fits(l, *ch)) else {
            return false;
        };
        // Every pixel a dab writes lies in its bounding box, so saving the
        // boxes first leaves the stencil only saved pixels to read.
        for &(dab_x, dab_y, jittered) in &placed {
            let radius = jittered.map_or(cfg.diameter, |t| t.diameter) as f32 * 0.5;
            saved.save(
                work_layer,
                *ch,
                (dab_x - radius).floor() as i32 - 1,
                (dab_y - radius).floor() as i32 - 1,
                (dab_x + radius).ceil() as i32 + 1,
                (dab_y + radius).ceil() as i32 + 1,
            );
        }
        let stencil = Stencil {
            base: saved.planes(),
            locked: *locked,
            cfg,
            paint: *paint,
            source: source.as_ref(),
            rect: *rect,
        };

        for (dab_x, dab_y, jittered) in placed {
            // The hoisted profile is exact when the dynamics left the tip
            // alone; a jittered dab rebuilds it from its own tip.
            let (params, radius) = match jittered {
                None => (*tip, cfg.diameter as f32 * 0.5),
                Some(tip_cfg) => (TipParams::new(&tip_cfg), tip_cfg.diameter as f32 * 0.5),
            };
            let y0 = ((dab_y - radius).floor() as i32).max(0);
            let y1 = ((dab_y + radius).ceil() as i32).min(h as i32 - 1);
            if y1 < y0 {
                continue;
            }
            let rows = (y1 - y0 + 1) as usize;
            let dab = DabRaster {
                params: &params,
                dab_x,
                dab_y,
                flow,
                width: w,
                stencil: &stencil,
            };
            let planes = plane_slices(work_layer, *ch, w, h, y0 as usize, (y1 + 1) as usize);
            let bands = split_bands(y0 as usize, rows, w, scratch, applied, planes);
            let bbox = if bands.len() == 1 {
                bands
                    .into_iter()
                    .next()
                    .and_then(|band| dab.band(band, serial.then_some(&mut *rng)))
            } else if serial {
                let mut acc: Option<PsdRect> = None;
                for band in bands {
                    if let Some(rect) = dab.band(band, Some(&mut *rng)) {
                        acc = Some(acc.map_or(rect, |b| union_rect(b, rect)));
                    }
                }
                acc
            } else {
                bands
                    .into_par_iter()
                    .filter_map(|band| dab.band(band, None))
                    .reduce_with(union_rect)
            };
            if let Some(bbox) = bbox {
                Self::grow_dirty(dirty, dab_dirty, bbox.left, bbox.top);
                Self::grow_dirty(dirty, dab_dirty, bbox.right - 1, bbox.bottom - 1);
                *painted = true;
                changed = true;
            }
        }
        changed
    }

    /// One dab of a step, scattered and jittered per the dynamics; `None` when
    /// the dynamics left the tip alone, `Some(tip)` when they altered it.
    fn jittered(&mut self, x: f32, y: f32) -> (f32, f32, Option<StrokeConfig>) {
        let cfg = self.cfg;
        let (mut x, mut y) = (x, y);
        if cfg.scatter > 0 {
            let reach = cfg.diameter as f32 * cfg.scatter as f32 / 100.0;
            x += (Self::next_f32(&mut self.rng) * 2.0 - 1.0) * reach;
            y += (Self::next_f32(&mut self.rng) * 2.0 - 1.0) * reach;
        }
        let mut tip: Option<StrokeConfig> = None;
        if cfg.size_jitter > 0 {
            // Jitter only ever shrinks, as CS6's does.
            let shrink =
                (1.0 - cfg.size_jitter as f32 / 100.0 * Self::next_f32(&mut self.rng)).max(0.05);
            tip.get_or_insert(cfg).diameter =
                ((cfg.diameter as f32 * shrink).round() as u32).max(1);
        }
        if cfg.angle_jitter > 0 {
            let turn = (Self::next_f32(&mut self.rng) * 2.0 - 1.0) * cfg.angle_jitter as f32;
            tip.get_or_insert(cfg).angle_deg = (cfg.angle_deg as f32 + turn).round() as i32;
        }
        if cfg.roundness_jitter > 0 {
            let flatten = cfg.roundness_jitter as f32 * Self::next_f32(&mut self.rng);
            tip.get_or_insert(cfg).roundness =
                (cfg.roundness as f32 - flatten).clamp(5.0, 100.0) as u8;
        }
        (x, y, tip)
    }

    /// The stroke-constant tip state, so a host that rasters dabs on the GPU
    /// derives the same profile the exact stroke does.
    pub fn tip_params(&self) -> TipParams {
        self.tip
    }

    /// The target layer's pixels in `doc` as a document-sized interleaved
    /// straight-RGBA8 buffer at the layer's document rect; zero outside it.
    /// Called before the first dab, `doc` still holds the pre-stroke pixels the
    /// stencil reads, so a GPU stroke seeded from this starts from the same
    /// source pixels the exact CPU stroke does.
    pub fn base_layer_rgba_doc(&self, doc: &Document) -> Vec<u8> {
        let (dw, dh) = (doc.width as usize, doc.height as usize);
        let mut out = vec![0u8; dw * dh * 4];
        let Some(layer) = layer_at(doc, &self.layer_path) else {
            return out;
        };
        let (lw, lh) = (
            self.rect.width().max(0) as usize,
            self.rect.height().max(0) as usize,
        );
        let [pr, pg, pb, pa] = read_planes(layer, self.ch);
        // A layer that spans the document exactly interleaves straight across,
        // with no per-pixel bounds or plane lookups.
        if self.rect.left == 0 && self.rect.top == 0 && lw == dw && lh == dh {
            if let (Some((r, g)), Some((b, a))) = (pr.zip(pg), pb.zip(pa)) {
                for (px, (((rv, gv), bv), av)) in out
                    .as_chunks_mut::<4>()
                    .0
                    .iter_mut()
                    .zip(r.iter().zip(g).zip(b).zip(a))
                {
                    *px = [*rv, *gv, *bv, *av];
                }
                return out;
            }
        }
        let at = |p: Option<&[u8]>, i: usize| p.and_then(|s| s.get(i)).copied().unwrap_or(0);
        for ly in 0..lh {
            let dy = self.rect.top + ly as i32;
            if dy < 0 || dy >= dh as i32 {
                continue;
            }
            for lx in 0..lw {
                let dx = self.rect.left + lx as i32;
                if dx < 0 || dx >= dw as i32 {
                    continue;
                }
                let i = ly * lw + lx;
                let o = (dy as usize * dw + dx as usize) * 4;
                out[o] = at(pr, i);
                out[o + 1] = at(pg, i);
                out[o + 2] = at(pb, i);
                out[o + 3] = at(pa, i);
            }
        }
        out
    }

    /// Whether the target layer's transparency lock would be violated by the
    /// GPU's source-over alpha. Such a layer stays on the exact CPU path.
    pub fn transparency_locked(&self) -> bool {
        self.locked
    }

    /// Whether the target layer stores an alpha plane; Clear on a layer without
    /// one is a no-op on both the CPU and GPU paths.
    pub fn has_alpha_plane(&self) -> bool {
        self.ch[3].is_some()
    }

    /// Write a GPU dab's region into the working document, clipped to the
    /// layer's own rect. `planes` holds four byte planes (R, G, B, A); plane `k`
    /// starts at `k * plane_stride` and pixel `(row, col)` sits at
    /// `row * doc_rect.width() + col`. Copies run a row at a time straight into
    /// the layer's channel planes, so the GPU only has to de-interleave.
    /// Returns whether any pixel was patched.
    pub fn patch_working_layer(
        &mut self,
        doc: &mut Document,
        doc_rect: PsdRect,
        plane_stride: usize,
        planes: &[u8],
    ) -> bool {
        let bw = doc_rect.width().max(0) as usize;
        let bh = doc_rect.height().max(0) as usize;
        if bw == 0 || bh == 0 || plane_stride < bw * bh {
            return false;
        }
        let (rl, rt) = (self.rect.left, self.rect.top);
        let (lw, lh) = (self.rect.width().max(0), self.rect.height().max(0));
        let left = (doc_rect.left - rl).max(0);
        let top = (doc_rect.top - rt).max(0);
        let right = (doc_rect.right - rl).min(lw);
        let bottom = (doc_rect.bottom - rt).min(lh);
        if right <= left || bottom <= top {
            return false;
        }
        // A plane's bytes are one contiguous run: its columns start `runner`
        // pixels into each region row, at the region row `top - (doc_rect.top - rt)`.
        let runner = (left - (doc_rect.left - rl)) as usize;
        let first_row = (top - (doc_rect.top - rt)) as usize;
        let run = (right - left) as usize;
        let ch = self.ch;
        let Some(layer) = layer_at_mut(doc, &self.layer_path).filter(|l| self.saved.fits(l, ch))
        else {
            return false;
        };
        self.saved.save(layer, ch, left, top, right - 1, bottom - 1);
        for (idx, c) in layer.channels.iter_mut().enumerate() {
            if ch.contains(&Some(idx)) {
                c.data.fracture(lw.max(0) as usize);
            }
        }
        let mut changed: Option<PsdRect> = None;
        for (row, ly) in (top..bottom).enumerate() {
            let src = (first_row + row) * bw + runner;
            let mut row_changed = false;
            for (idx, c) in layer.channels.iter_mut().enumerate() {
                let Some(k) = ch.iter().position(|&x| x == Some(idx)) else {
                    continue;
                };
                let Some(bytes) = planes.get(k * plane_stride + src..) else {
                    continue;
                };
                let Some(slot) = c.data.row_mut(ly as usize, lw.max(0) as usize) else {
                    continue;
                };
                let at = left as usize;
                if at + run <= slot.len() {
                    if let Some(chunk) = bytes.get(..run) {
                        if slot[at..at + run] != chunk[..] {
                            slot[at..at + run].copy_from_slice(chunk);
                            row_changed = true;
                        }
                    }
                }
            }
            if row_changed {
                let row_box = PsdRect {
                    top: ly,
                    left,
                    bottom: ly + 1,
                    right,
                };
                changed = Some(changed.map_or(row_box, |c| union_rect(c, row_box)));
            }
        }
        let Some(rect) = changed else {
            return false;
        };
        Self::grow_dirty(&mut self.dirty, &mut self.dab_dirty, rect.left, rect.top);
        Self::grow_dirty(
            &mut self.dirty,
            &mut self.dab_dirty,
            rect.right - 1,
            rect.bottom - 1,
        );
        self.painted = true;
        true
    }

    /// The paint on a Mixer Brush after the dabs so far; `None` for other kinds.
    pub fn mixer_reservoir(&self) -> Option<Rgba> {
        match &self.per_dab {
            Some(PerDab {
                engine: DabEngine::Mixer(mixer),
                ..
            }) => Some(mixer.reservoir()),
            _ => None,
        }
    }

    fn apply_per_dab(&mut self, doc: &mut Document, dabs: &[(f32, f32)]) -> bool {
        let cfg = self.cfg;
        // The composite engines keep their own alpha, but a layer without an
        // alpha plane cannot store Clear: match the regular raster's no-op.
        if cfg.mode == PaintMode::Clear && self.ch[3].is_none() {
            return false;
        }
        let Some(per) = self.per_dab.as_mut() else {
            return false;
        };
        let mut changed = None;
        for &(x, y) in dabs {
            let dirty = match &mut per.engine {
                DabEngine::Replace(replacer) => {
                    replacer.dab(&mut per.pixels, &cfg, x, y, cfg.color)
                }
                DabEngine::Mixer(mixer) => mixer.dab(&mut per.pixels, &cfg, x, y),
                DabEngine::ArtHistory(brush) => brush.dab(&mut per.pixels, &cfg, x, y),
                DabEngine::BackgroundErase(eraser) => {
                    eraser.dab(&mut per.pixels, &cfg, x, y, cfg.color)
                }
                DabEngine::Focus(brush) => brush.dab(&mut per.pixels, &cfg, x, y),
                DabEngine::Smudge(brush) => brush.dab(&mut per.pixels, &cfg, x, y),
                DabEngine::Tone(brush) => brush.dab(&mut per.pixels, &cfg, x, y),
            };
            if let Some(d) = dirty {
                changed = Some(changed.map_or(d, |c: PsdRect| union_rect(c, d)));
            }
        }
        let Some(rect) = changed else {
            return false;
        };
        let ch = self.ch;
        let Some(layer) = layer_at_mut(doc, &self.layer_path).filter(|l| self.saved.fits(l, ch))
        else {
            return false;
        };
        self.saved.save(
            layer,
            ch,
            rect.left,
            rect.top,
            rect.right - 1,
            rect.bottom - 1,
        );
        let w = per.pixels.width;
        let mut written: Option<PsdRect> = None;
        for y in rect.top..rect.bottom {
            for x in rect.left..rect.right {
                let i = (y * w + x) as usize;
                let [r, g, b, a] = per.pixels.data[i];
                if write_pixel(layer, ch, i, PaintMode::Normal, (r, g, b, a)) {
                    let pixel = PsdRect {
                        top: y,
                        left: x,
                        bottom: y + 1,
                        right: x + 1,
                    };
                    written = Some(written.map_or(pixel, |c| union_rect(c, pixel)));
                }
            }
        }
        let Some(rect) = written else {
            return false;
        };
        Self::grow_dirty(&mut self.dirty, &mut self.dab_dirty, rect.left, rect.top);
        Self::grow_dirty(
            &mut self.dirty,
            &mut self.dab_dirty,
            rect.right - 1,
            rect.bottom - 1,
        );
        self.painted = true;
        true
    }

    /// The document-space rectangle the dabs placed since the last call changed,
    /// then clears it. The next call reports only newer dabs, so the per-dab
    /// refresh cost stays dab-sized instead of growing with the stroke.
    pub fn take_dirty(&mut self) -> Option<PsdRect> {
        self.dab_dirty.take().map(|d| self.dirty_doc(d))
    }

    /// The document rectangle the stroke has changed so far, or `None`.
    pub fn dirty(&self) -> Option<PsdRect> {
        self.dirty.map(|d| self.dirty_doc(d))
    }

    /// End the stroke, leaving its paint in the document. Returns the document
    /// rectangle it changed, or `None` when no pixel changed.
    pub fn finish(self) -> Option<PsdRect> {
        if !self.painted {
            return None;
        }
        let local = self.dirty.expect("painted implies dirty");
        Some(self.dirty_doc(local))
    }

    /// The same stroke rastering every dab on one thread.
    #[cfg(test)]
    fn serialized(mut self) -> Self {
        self.serial = true;
        self
    }

    /// Save every tile up front, as a stroke that copied the whole layer would
    /// have: the oracle the lazily saving stroke is held to.
    #[cfg(test)]
    fn save_all(&mut self, doc: &Document) {
        if let Some(layer) = layer_at(doc, &self.layer_path) {
            let (w, h) = (self.saved.width, self.saved.height);
            self.saved.save(layer, self.ch, 0, 0, w - 1, h - 1);
        }
    }

    /// Abandon the stroke, writing every pixel it changed in `doc` back to its
    /// pre-stroke value.
    pub fn cancel(self, doc: &mut Document) {
        if let Some(layer) =
            layer_at_mut(doc, &self.layer_path).filter(|l| self.saved.fits(l, self.ch))
        {
            self.saved.restore(layer, self.ch);
        }
    }

    fn dirty_doc(&self, local: PsdRect) -> PsdRect {
        PsdRect {
            top: self.rect.top + local.top,
            left: self.rect.left + local.left,
            bottom: self.rect.top + local.bottom,
            right: self.rect.left + local.right,
        }
    }

    fn grow_dirty(dirty: &mut Option<PsdRect>, dab_dirty: &mut Option<PsdRect>, lx: i32, ly: i32) {
        let grow = |d: Option<PsdRect>| {
            Some(match d {
                None => PsdRect {
                    top: ly,
                    left: lx,
                    bottom: ly + 1,
                    right: lx + 1,
                },
                Some(d) => PsdRect {
                    top: d.top.min(ly),
                    left: d.left.min(lx),
                    bottom: d.bottom.max(ly + 1),
                    right: d.right.max(lx + 1),
                },
            })
        };
        *dirty = grow(*dirty);
        *dab_dirty = grow(*dab_dirty);
    }

    fn pixel_rgb_matches(&self, doc: &Document, lx: f32, ly: f32, color: Rgba) -> bool {
        let (w, h) = (self.rect.width(), self.rect.height());
        let px = lx.floor() as i32;
        let py = ly.floor() as i32;
        if px < 0 || py < 0 || px >= w || py >= h {
            return false;
        }
        let i = py as usize * w as usize + px as usize;
        let Some(layer) = layer_at(doc, &self.layer_path) else {
            return false;
        };
        let (r, g, b, _) = read_pixel(layer, i);
        r == color.r && g == color.g && b == color.b
    }

    fn next_f32(rng: &mut u64) -> f32 {
        *rng = rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 40) as f32 / 16_777_216.0
    }
}

/// A stroke's per-pixel inputs, resolved once per sample so a dab's loop walks
/// neither the layer path nor the channel list and re-derives no profile.
struct Stencil<'a> {
    base: [Option<&'a [u8]>; 4],
    locked: bool,
    cfg: StrokeConfig,
    paint: Rgba,
    /// Clone Stamp / Pattern Stamp / History Brush: the colour at each pixel
    /// comes from the source instead of `paint`.
    source: Option<&'a StampSource>,
    rect: PsdRect,
}

impl Stencil<'_> {
    /// Lay the paint at `coverage` over the saved pixel `i` (layer-local) and
    /// write it to `planes` at `at`, the same pixel's index in the band the
    /// planes were cut to. `rng` is the stroke's Dissolve state.
    fn composite(
        &self,
        planes: &mut BandPlanes<'_>,
        at: usize,
        i: usize,
        rng: Option<&mut u64>,
        coverage: u8,
    ) -> bool {
        let cov = coverage as f32 / 255.0;
        let a = (cov * self.cfg.opacity as f32 / 100.0).clamp(0.0, 1.0);
        let plane = |k: usize, missing: u8| {
            self.base[k]
                .and_then(|d| d.get(i))
                .copied()
                .unwrap_or(missing)
        };
        let dr = plane(0, 0);
        let dg = plane(1, 0);
        let db = plane(2, 0);
        // A layer without an alpha channel (a Background) is opaque.
        let da = plane(3, 255);
        let s = match self.source {
            None => self.paint,
            Some(source) => {
                let w = self.rect.width() as usize;
                let (x, y) = ((i % w) as i32, (i / w) as i32);
                match source.at(self.rect.left + x, self.rect.top + y) {
                    Some(color) => color,
                    None => return false,
                }
            }
        };
        let mode = self.cfg.mode;
        let roll = match rng {
            Some(rng) if mode == PaintMode::Dissolve => Stroke::next_f32(rng),
            _ => 0.0,
        };
        let Some((r, g, b, alpha)) = blend_pixel(mode, (dr, dg, db, da), &s, a, roll) else {
            return false;
        };
        // A transparency lock preserves each pixel's alpha: fully transparent
        // pixels stay untouched, everything else keeps its pre-stroke alpha.
        // (Clear/auto-erase are refused for the whole stroke in `begin_at`.)
        if self.locked {
            if da == 0 {
                return false;
            }
            write_band(planes, at, mode, (r, g, b, da))
        } else {
            write_band(planes, at, mode, (r, g, b, alpha))
        }
    }
}

/// Feed a whole sample list into a destructive stroke and commit on success.
pub fn paint_stroke(
    doc: &mut Document,
    path: &str,
    cfg: &StrokeConfig,
    samples: &[StrokeSample],
) -> Option<PsdRect> {
    let mut stroke = Stroke::begin_at(doc, path, *cfg).ok()?;
    for &s in samples {
        stroke.sample(doc, s);
    }
    stroke.finish()
}

pub(crate) fn union_rect(a: PsdRect, b: PsdRect) -> PsdRect {
    PsdRect {
        top: a.top.min(b.top),
        left: a.left.min(b.left),
        bottom: a.bottom.max(b.bottom),
        right: a.right.max(b.right),
    }
}

/// Positions of the R, G, B and alpha planes in a layer's channel list, or
/// `None` where the layer has no such plane.
type PlaneIndex = [Option<usize>; 4];

/// The plane positions for `layer`; a stroke resolves them once instead of
/// re-scanning `channels` for every changed pixel.
fn plane_index(layer: &Layer) -> PlaneIndex {
    let at = |id| layer.channels.iter().position(|c| c.id == id);
    [at(0), at(1), at(2), at(-1)]
}

/// A layer without an alpha channel (a Background) is opaque.
fn read_idx(layer: &Layer, ch: PlaneIndex, i: usize) -> (u8, u8, u8, u8) {
    let g = |k: Option<usize>, missing: u8| {
        k.and_then(|k| layer.channels.get(k))
            .and_then(|c| c.data.get(i))
            .copied()
            .unwrap_or(missing)
    };
    (g(ch[0], 0), g(ch[1], 0), g(ch[2], 0), g(ch[3], 255))
}

fn read_pixel(layer: &Layer, i: usize) -> (u8, u8, u8, u8) {
    read_idx(layer, plane_index(layer), i)
}

/// The four planes of `layer` at the stroke's resolved positions, borrowed for
/// the life of a sample.
fn read_planes(layer: &Layer, ch: PlaneIndex) -> [Option<&[u8]>; 4] {
    let at = |k: Option<usize>| {
        k.and_then(|k| layer.channels.get(k))
            .map(|c| c.data.as_slice())
    };
    [at(ch[0]), at(ch[1]), at(ch[2]), at(ch[3])]
}

fn write_pixel(
    layer: &mut Layer,
    ch: PlaneIndex,
    i: usize,
    mode: PaintMode,
    rgba: (u8, u8, u8, u8),
) -> bool {
    let (r, g, b, a) = rgba;
    let mut changed = false;
    if !matches!(mode, PaintMode::Clear) {
        for (k, v) in [(ch[0], r), (ch[1], g), (ch[2], b)] {
            if let Some(k) = k {
                if layer.channels[k].data[i] != v {
                    layer.channels[k].data[i] = v;
                    changed = true;
                }
            }
        }
    }
    if let Some(k) = ch[3] {
        if layer.channels[k].data[i] != a {
            layer.channels[k].data[i] = a;
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
fn channel_data(layer: &Layer, id: i16) -> Option<&[u8]> {
    layer
        .channels
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.data.as_slice())
}

#[cfg(test)]
fn channel_data_mut(layer: &mut Layer, id: i16) -> Option<&mut [u8]> {
    layer
        .channels
        .iter_mut()
        .find(|c| c.id == id)
        .map(|c| c.data.as_mut_slice())
}

pub(crate) fn layer_at<'a>(doc: &'a Document, path: &[usize]) -> Option<&'a Layer> {
    let mut node = doc.layers.get(*path.first()?)?;
    for &idx in &path[1..] {
        node = node.children.get(idx)?;
    }
    Some(node)
}

pub(crate) fn layer_at_mut<'a>(doc: &'a mut Document, path: &[usize]) -> Option<&'a mut Layer> {
    let mut node = doc.layers.get_mut(*path.first()?)?;
    for &idx in &path[1..] {
        node = node.children.get_mut(idx)?;
    }
    Some(node)
}

/// Resolve a panel path (`"0"`, `"0/1"`) to layer indices; `None` when malformed.
pub(crate) fn parse_layer_path(path: &str) -> Option<Vec<usize>> {
    path.split('/')
        .map(|part| part.parse::<usize>().ok())
        .collect()
}

mod blend;
mod raster;
mod surface;
#[cfg(test)]
mod tests;

pub(crate) use blend::blend_pixel;
#[cfg(test)]
pub(crate) use surface::layer_rgba_serial;
pub(crate) use surface::{layer_planes, layer_rgba};
