use super::document_session::{PreviewStroke, SessionKind, StrokeSession};
use super::helpers::*;
use super::helpers_composite::{patch_composite_region, planes_of};
use super::qobject;
use super::state::PictureViewRust;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{Document, PixelBuffer, PsdRect};
use pictura_paint::spacing::DabPlacer;
use pictura_paint::{spacing::SpacingMode, PaintMode, Stroke, StrokeConfig, StrokeSample};
use pictura_render::gpu::{GpuPaintMode, GpuStroke, GpuStrokeParams};
use pictura_render::ViewPyramid;
use std::time::Instant;

mod mask;
mod mask_gpu;
pub(super) mod tool_recording;

impl PictureViewRust {
    /// Fold a dab's `rect` into the frame-bounded present. `Some(rect)` is the
    /// region to present now — the dab that opens a frame — and `None` means it
    /// accumulates until [`Self::take_pending_present`].
    pub(super) fn queue_present(&mut self, rect: PsdRect) -> Option<PsdRect> {
        if self.session.live.present_flush_due() {
            let pending = self.session.live.take_pending_present();
            self.session.live.set_pending_present(Some(match pending {
                Some(pending) => union_rect(pending, rect),
                None => rect,
            }));
            None
        } else {
            self.session.live.set_present_flush_due(true);
            Some(rect)
        }
    }

    /// Clear the frame and hand back the region accumulated since the last
    /// present, if any.
    pub(super) fn take_pending_present(&mut self) -> Option<PsdRect> {
        self.session.live.set_present_flush_due(false);
        self.session.live.take_pending_present()
    }

    /// Drop a pending present: the stroke start, the commit refresh and the
    /// cancel restore all supersede whatever it would have shown.
    pub(super) fn clear_pending_present(&mut self) {
        self.session.live.set_pending_present(None);
        self.session.live.set_present_flush_due(false);
    }

    /// The stroke's dirty tiles as disjoint document rectangles, falling back to
    /// `union` when the tiles were never marked (a path that bypasses
    /// `take_dirty`) or are not worth decomposing.
    #[cfg(test)]
    pub(super) fn take_stroke_regions(&mut self, union: PsdRect) -> Vec<PsdRect> {
        match self.session.live.take_stroke_tiles() {
            Some(tiles) => tiles.regions(union),
            None => vec![union],
        }
    }

    /// Composite `rect` of the current source (the active stroke's working
    /// document, else the app document), patch the authoritative `doc.composite`,
    /// and hand back the clamped origin, clipped rect and straight sRGB region.
    /// `None` for an empty clamp, an absent source, or an empty region. Level 0,
    /// damage and the pyramid are the caller's to fold in.
    ///
    /// A layer stroke's presents patch the composite as they go, so its commit
    /// composites only what is still pending; cancelling repairs the extent
    /// (`cancel_stroke`). A mask stroke leaves it alone: its cancel restores
    /// the view from the untouched composite.
    pub(super) fn refresh_region_buffer(
        &mut self,
        rect: PsdRect,
    ) -> Option<(i32, i32, PsdRect, PixelBuffer)> {
        let painting = self.session.live.mask_stroke().is_some();
        let source: &Document = self.session.doc.as_ref()?;
        let (x0, y0, w, h) = clamp_region(rect, source.width, source.height)?;
        let gpu_compute = self.gpu_compute;
        let t = Instant::now();
        let buffer = pictura_render::composite_region_active(source, rect, gpu_compute).0;
        paint_timing::record("rr_composite_region", t.elapsed());
        let t = Instant::now();
        let srgb = pictura_codec::buffer_to_srgb(source, &buffer).into_owned();
        paint_timing::record("rr_buffer_to_srgb", t.elapsed());
        if srgb.width == 0 || srgb.height == 0 {
            return None;
        }
        if !painting {
            if let Some(doc) = self.session.doc.as_mut() {
                let t = Instant::now();
                patch_composite_region(doc, &buffer, x0, y0);
                paint_timing::record("rr_patch_composite", t.elapsed());
            }
        }
        let clipped = PsdRect {
            top: y0,
            left: x0,
            bottom: y0 + h as i32,
            right: x0 + w as i32,
        };
        Some((x0, y0, clipped, srgb))
    }

    /// Abandon the live stroke, putting back the pixels it changed (or the
    /// whole pre-stroke document when the stroke changed its structure), and
    /// return the document rectangle it had painted.
    pub(super) fn cancel_stroke(&mut self) -> Option<PsdRect> {
        let session = self.session.live.take_stroke_session()?;
        self.cancel_stroke_session(session)
    }

    /// Abandon a paint session, putting back the pixels it changed (or the
    /// whole pre-stroke document when the stroke changed its structure), and
    /// return the document rectangle it had painted.
    pub(super) fn cancel_stroke_session(&mut self, session: StrokeSession) -> Option<PsdRect> {
        if let Some(mut ctx) = session.mask_stroke {
            let dirty = ctx.stroke.dirty();
            ctx.stroke.cancel(&mut ctx.document);
            if let Some(doc) = self.session.doc.as_mut() {
                pictura_render::write_mask_back(doc, &ctx.path, &ctx.document, None);
            }
            return dirty.map(|rect| PsdRect {
                top: rect.top + ctx.rect.top,
                left: rect.left + ctx.rect.left,
                bottom: rect.bottom + ctx.rect.top,
                right: rect.right + ctx.rect.left,
            });
        }
        let stroke = session.stroke?;
        let dirty = stroke.dirty();
        match (session.stroke_base, self.session.doc.as_mut()) {
            (Some(base), _) => self.session.doc = Some(base),
            (None, Some(doc)) => {
                stroke.cancel(doc);
                // The stroke's presents patched the composite; put the
                // restored pixels back into it before anything reads it.
                if let Some(rect) = dirty {
                    let buffer =
                        pictura_render::composite_region_active(doc, rect, self.gpu_compute).0;
                    if let Some((x0, y0, _, _)) = clamp_region(rect, doc.width, doc.height) {
                        patch_composite_region(doc, &buffer, x0, y0);
                    }
                }
            }
            (None, None) => {}
        }
        dirty
    }

    /// Fold a refreshed region into level 0, the damage account and the pyramid.
    pub(super) fn apply_refreshed_region(
        &mut self,
        x0: i32,
        y0: i32,
        clipped: PsdRect,
        srgb: PixelBuffer,
    ) {
        let t = Instant::now();
        self.refresh_level0_region(srgb, x0, y0);
        paint_timing::record("rr_level0_patch", t.elapsed());
        self.damage.mark(clipped);
        let t = Instant::now();
        self.update_pyramid(clipped);
        paint_timing::record("rr_update_pyramid", t.elapsed());
        // Keep a soloed mask plane in step with the mask's new coverage.
        self.sync_mask_solo_region(clipped);
    }
}

/// The bounding-box area at which a dab's exact rasterization stops fitting the
/// 16 ms input-to-first-pixel Target: 262 144 px at the 58 ns/px that
/// `large_brush_profile_4000` measures on the reference machine.
const RASTER_BUDGET_PX: u64 = 262_144;

/// The level a preview starts at. 1/8 scale keeps the reduced document's
/// construction at a sixty-fourth of the full planes — about 8 ms for an
/// eight-layer 4000² document — which is what makes the preview cheaper than
/// the dab it replaces even for a single click.
const PREVIEW_START_LEVEL: u32 = 3;

/// The level the policy stops at; it also bounds every shift below.
const PREVIEW_MAX_LEVEL: u32 = 16;

/// The view-pyramid level a stroke of `diameter` on a `w × h` document previews
/// at, or `None` when the exact raster already fits the budget.
///
/// The area is the worst case over the stroke: a dab is clipped to the
/// document, so a brush wider than the document costs a document, not a disc.
pub(super) fn preview_level(diameter: i32, w: u32, h: u32) -> Option<u32> {
    let d = diameter.max(0) as u64 + 2;
    let area = d.min(w as u64) * d.min(h as u64);
    if area <= RASTER_BUDGET_PX {
        return None;
    }
    let mut level = PREVIEW_START_LEVEL;
    while level < PREVIEW_MAX_LEVEL && (area >> (2 * level)) > RASTER_BUDGET_PX {
        level += 1;
    }
    Some(level)
}

/// Bytes a stroke-start snapshot of `level` occupies for a `w × h` document.
///
/// The snapshot is the whole stored level, so it is what a brush the size of
/// the document costs to preview against — a few megabytes, not a document.
pub(super) fn preview_snapshot_bytes(w: u32, h: u32, level: u32) -> usize {
    if level >= PREVIEW_MAX_LEVEL {
        return 0;
    }
    let scale = 1u32 << level;
    ((w.div_ceil(scale) * h.div_ceil(scale)) * 4) as usize
}

impl PreviewStroke {
    /// Snapshot `level` of `pyramid` as it stands before the stroke, so every
    /// present can blend the whole extent against the pre-stroke pixels instead
    /// of compounding on its own earlier output. `None` when the level does not
    /// exist yet or is not the size `level0` implies.
    pub(super) fn new(
        level: u32,
        pyramid: &ViewPyramid,
        level0: &PixelBuffer,
        cfg: StrokeConfig,
    ) -> Option<Self> {
        if level >= PREVIEW_MAX_LEVEL {
            return None;
        }
        let scale = 1u32 << level;
        let (width, height) = (level0.width.div_ceil(scale), level0.height.div_ceil(scale));
        if width == 0 || height == 0 || pyramid.level_size(level as usize) != (width, height) {
            return None;
        }
        let whole = PsdRect {
            top: 0,
            left: 0,
            bottom: height as i32,
            right: width as i32,
        };
        let planes = planes_of(level0)?;
        let crop = pyramid.crop(planes, level as usize, whole);
        (crop.width() == width && crop.height() == height).then(|| Self {
            level,
            scale,
            width,
            height,
            snapshot: crop.data().to_vec(),
            coverage: vec![0u8; preview_snapshot_bytes(level0.width, level0.height, level) / 4],
            extent: None,
            samples: Vec::new(),
            cfg,
            mask: None,
        })
    }
}

/// Grow `extent` to cover the level-space pixel `(lx, ly)`.
fn grow_extent(extent: &mut Option<PsdRect>, lx: i32, ly: i32) {
    let next = PsdRect {
        top: ly,
        left: lx,
        bottom: ly + 1,
        right: lx + 1,
    };
    *extent = Some(match extent.take() {
        None => next,
        Some(d) => PsdRect {
            top: d.top.min(next.top),
            left: d.left.min(next.left),
            bottom: d.bottom.max(next.bottom),
            right: d.right.max(next.right),
        },
    });
}

/// Rasterize one sample into the preview's coverage at `1 << level`, using the
/// same tip profile and flow accumulation the exact stroke uses, sampled at the
/// level pixel's centre. Returns whether any pixel changed.
fn rasterize_preview(preview: &mut PreviewStroke, sample: StrokeSample) -> bool {
    let cfg = preview.cfg;
    let scale = preview.scale as f32;
    let radius = cfg.diameter as f32 * 0.5;
    let flow = cfg.flow as f32 / 100.0;
    let (cx, cy) = (sample.x / scale, sample.y / scale);
    let r = radius / scale;
    let (w, h) = (preview.width as i32, preview.height as i32);
    if w <= 0 || h <= 0 {
        return false;
    }
    let x0 = ((cx - r).floor() as i32).max(0);
    let x1 = ((cx + r).ceil() as i32).min(w - 1);
    let y0 = ((cy - r).floor() as i32).max(0);
    let y1 = ((cy + r).ceil() as i32).min(h - 1);
    let mut changed = false;
    for ly in y0..=y1 {
        for lx in x0..=x1 {
            let dx = (lx as f32 + 0.5 - cx) * scale;
            let dy = (ly as f32 + 0.5 - cy) * scale;
            let tip = pictura_paint::tip_coverage(&cfg, dx, dy);
            if tip <= 0.0 {
                continue;
            }
            let i = ly as usize * preview.width as usize + lx as usize;
            let acc = 1.0 - (1.0 - preview.coverage[i] as f32 / 255.0) * (1.0 - flow * tip);
            preview.coverage[i] = (acc.clamp(0.0, 1.0) * 255.0).round() as u8;
            grow_extent(&mut preview.extent, lx, ly);
            changed = true;
        }
    }
    changed
}

/// The premultiplied level-space region for `rect` of `preview`: the stroke-start
/// snapshot blended with the accumulated coverage under the stroke's paint mode.
/// Normal is source-over; Clear removes the snapshot's coverage (`dst * (1 - k)`);
/// Dissolve, Behind, and Multiply present as Normal, since only the committed
/// pixels must be exact and the preview is thrown away. Wet Edges remaps the
/// coverage as the stroke does, so the preview keeps the stroke's tone.
fn preview_blend(preview: &PreviewStroke, rect: PsdRect) -> Vec<u8> {
    if let Some(mask) = &preview.mask {
        return mask::mask_preview_blend(preview, mask, rect);
    }
    let (rw, rh) = (rect.width().max(0) as u32, rect.height().max(0) as u32);
    if rw == 0 || rh == 0 {
        return Vec::new();
    }
    let colour = preview.cfg.color;
    let opacity = preview.cfg.opacity as f32 / 100.0;
    let tint = colour.a as f32 / 255.0;
    let clear = preview.cfg.mode == PaintMode::Clear;
    let mut rgba = vec![0u8; (rw * rh * 4) as usize];
    for y in 0..rh as usize {
        for x in 0..rw as usize {
            let li = (rect.top as usize + y) * preview.width as usize + (rect.left as usize + x);
            let coverage = if preview.cfg.wet_edges {
                pictura_paint::wet_edge(preview.coverage[li])
            } else {
                preview.coverage[li]
            };
            let alpha = opacity * coverage as f32 / 255.0;
            // Normal carries the foreground alpha; Clear removes coverage and
            // ignores it, exactly as `Stencil::composite` does.
            let k = if clear { alpha } else { alpha * tint };
            let inv = 1.0 - k;
            let o = (y * rw as usize + x) * 4;
            let si = li * 4;
            let (kr, kg, kb, ka) = if clear {
                (0.0, 0.0, 0.0, 0.0)
            } else {
                (
                    colour.r as f32 * k,
                    colour.g as f32 * k,
                    colour.b as f32 * k,
                    255.0 * k,
                )
            };
            rgba[o] = (kr + preview.snapshot[si] as f32 * inv) as u8;
            rgba[o + 1] = (kg + preview.snapshot[si + 1] as f32 * inv) as u8;
            rgba[o + 2] = (kb + preview.snapshot[si + 2] as f32 * inv) as u8;
            rgba[o + 3] = (ka + preview.snapshot[si + 3] as f32 * inv) as u8;
        }
    }
    rgba
}

/// The GPU dab mode for `mode`, or `None` when only the CPU raster models it.
fn gpu_paint_mode(mode: PaintMode) -> Option<GpuPaintMode> {
    match mode {
        PaintMode::Normal => Some(GpuPaintMode::Normal),
        PaintMode::Clear => Some(GpuPaintMode::Clear),
        _ => None,
    }
}

impl qobject::PictureView {
    pub fn begin_paint(
        mut self: Pin<&mut Self>,
        foreground: u32,
        background: u32,
        diameter: i32,
        hardness: i32,
        roundness: i32,
        angle: i32,
        opacity: i32,
        flow: i32,
        spacing: i32,
        mode: &QString,
        aliased: bool,
        auto_erase: bool,
    ) -> bool {
        paint_timing::start("stroke (begin -> commit)");
        let _begin = paint_timing::Scope::new("begin_total");
        let cfg = StrokeConfig {
            color: rgba_from_argb(foreground),
            background: rgba_from_argb(background),
            diameter: diameter.max(0) as u32,
            hardness: hardness.clamp(0, 100) as u8,
            roundness: roundness.clamp(0, 100) as u8,
            angle_deg: angle,
            spacing: SpacingMode::Fixed(spacing.clamp(0, 1000) as u16),
            opacity: opacity.clamp(0, 100) as u8,
            flow: flow.clamp(0, 100) as u8,
            mode: paint_mode_from(&mode.to_string()),
            aliased,
            auto_erase,
            ..StrokeConfig::default()
        }
        // The exact stroke sanitizes internally; the preview and the level policy
        // must use the same clamped config so they rasterize the committed tip.
        .sanitized();
        let label = if aliased { "Pencil" } else { "Brush" };
        self.as_mut().rust_mut().begin_paint(cfg, label)
    }
}

impl PictureViewRust {
    /// Begin a brush stroke on the active layer, or on the active mask's
    /// coverage when a mask is the edit target. False without a document, a
    /// layer target, or when another session owns the slot.
    pub(super) fn begin_paint(&mut self, cfg: StrokeConfig, label: &str) -> bool {
        if self.session.doc.is_none() {
            return false;
        }
        // A mask target paints the mask's coverage instead of the layer: the
        // foreground's Rec.601 luma is the painted value, so black hides and
        // white reveals. The stroke runs on a mask-sized grayscale document.
        // The target is resolved before the slot is taken, so a mask stroke
        // does not begin a second session inside this one.
        if let Some(path) = active_mask_target(self) {
            if !self.slot_allows(SessionKind::MaskPaint) {
                return false;
            }
            let begun = self.begin_mask_paint(&path, mask_stroke_config(cfg), label);
            if begun {
                tool_recording::begin(label, &cfg, tool_recording::ToolStrokeKind::Mask);
            }
            return begun;
        }
        if !self.slot_allows(SessionKind::Paint) {
            return false;
        }
        let begun = {
            let rust = &*self;
            let Some(doc) = rust.session.doc.as_ref() else {
                return false;
            };
            let Some(path) = rust.layer_target.single() else {
                return false;
            };
            if active_pixel_layer(doc, Some(path)).is_none()
                || !active_layer_visible(doc, Some(path))
            {
                return false;
            }
            let t = Instant::now();
            let outcome = Stroke::begin_at(doc, path, cfg);
            paint_timing::record("stroke_begin_at (doc clone)", t.elapsed());
            outcome
        };
        match begun {
            Ok(stroke) => {
                let rust = &mut *self;
                // The stroke's own session takes the slot: the payload and the
                // present state it accumulates travel together.
                let session = StrokeSession {
                    stroke: Some(stroke),
                    stroke_label: label.to_string(),
                    ..Default::default()
                };
                rust.session.live.set_stroke_session(session);
                // A new stroke opens its own present frame.
                rust.clear_pending_present();
                let dims = rust.session.doc.as_ref().map(|doc| (doc.width, doc.height));
                if let Some((width, height)) = dims {
                    rust.session.live.reset_stroke_tiles(width, height);
                }
                tool_recording::begin(label, &cfg, tool_recording::ToolStrokeKind::Paint);
                // GPU first, but only where the CPU has a ceiling to beat: a dab
                // over the raster budget. Below it the exact CPU raster is a few
                // hundred microseconds, while seeding the GPU costs a full-frame
                // upload (~65 ms at 4000²), so a small brush stays on the CPU path.
                let level = rust
                    .session
                    .doc
                    .as_ref()
                    .and_then(|doc| preview_level(cfg.diameter as i32, doc.width, doc.height));
                // The GPU dab shader has no paper grain or wet-edge curve; such a
                // tip stays on the CPU oracle so the stroke matches its
                // committed pixels.
                let gpu_mode = gpu_paint_mode(cfg.mode)
                    .filter(|_| !cfg.auto_erase && cfg.grain == 0 && !cfg.wet_edges);
                if level.is_some() && rust.gpu_compute {
                    let seeded = match (
                        gpu_mode,
                        rust.session.live.stroke(),
                        rust.session.doc.as_ref(),
                    ) {
                        (Some(mode), Some(stroke), Some(doc))
                            if !stroke.transparency_locked()
                                && !(cfg.mode == PaintMode::Clear && !stroke.has_alpha_plane()) =>
                        {
                            let t = Instant::now();
                            let seed = stroke.base_layer_rgba_doc(doc);
                            let (w, h) = (doc.width, doc.height);
                            paint_timing::record("gpu_seed_interleave", t.elapsed());
                            let tip = stroke.tip_params();
                            let params = GpuStrokeParams {
                                radius: tip.radius,
                                round_radius: tip.round_radius,
                                sin_t: tip.sin,
                                cos_t: tip.cos,
                                core: tip.core,
                                denom: tip.denom,
                                flow: cfg.flow as f32 / 100.0,
                                opacity: cfg.opacity as f32 / 100.0,
                                color: [
                                    cfg.color.r as f32 / 255.0,
                                    cfg.color.g as f32 / 255.0,
                                    cfg.color.b as f32 / 255.0,
                                    cfg.color.a as f32 / 255.0,
                                ],
                                aliased: tip.aliased,
                                flip_x: tip.flip_x,
                                flip_y: tip.flip_y,
                                mode,
                                channels: 4,
                            };
                            let t = Instant::now();
                            let gpu = GpuStroke::new(w, h, &seed, params);
                            paint_timing::record("gpu_new (alloc+upload)", t.elapsed());
                            gpu.ok()
                        }
                        _ => None,
                    };
                    if let Some(gpu) = seeded {
                        rust.session.live.set_gpu_stroke(gpu);
                        rust.session
                            .live
                            .set_gpu_placer(DabPlacer::new(cfg.spacing, cfg.diameter as f32));
                    }
                }
                // The reduced-level preview is the fallback when the GPU path is
                // not taken; the exact stroke below still runs, replayed from the
                // log when the stroke commits.
                if rust.session.live.gpu_stroke().is_none() {
                    let t = Instant::now();
                    let preview = match (level, rust.level0.as_ref()) {
                        (Some(level), Some(level0)) => {
                            PreviewStroke::new(level, &rust.pyramid, level0, cfg)
                        }
                        _ => None,
                    };
                    if let Some(preview) = preview {
                        rust.session.live.set_preview(preview);
                    }
                    paint_timing::record("preview_new (snapshot)", t.elapsed());
                }
                true
            }
            Err(_) => false,
        }
    }
}

impl qobject::PictureView {
    pub fn paint_dab(mut self: Pin<&mut Self>, x: f64, y: f64, pressure: f64) -> bool {
        let _dab = paint_timing::Scope::new("dab_total");
        tool_recording::sample(x, y, pressure);
        if self.rust().session.live.mask_stroke().is_some() {
            return self.as_mut().mask_dab(x, y, pressure);
        }
        if self.rust().session.live.gpu_stroke().is_some() {
            return self.as_mut().gpu_dab(x, y, pressure);
        }
        if self.rust().session.live.preview().is_some() {
            return self.as_mut().preview_dab(x, y, pressure);
        }
        let dirty = {
            let mut rust = self.as_mut().rust_mut();
            let rust = &mut *rust;
            let t = Instant::now();
            let painted = match (rust.session.live.stroke_mut(), rust.session.doc.as_mut()) {
                (Some(stroke), Some(doc)) => stroke.sample(
                    doc,
                    StrokeSample {
                        x: x as f32,
                        y: y as f32,
                        pressure: pressure as f32,
                    },
                ),
                _ => return false,
            };
            paint_timing::record("cpu_stroke_sample", t.elapsed());
            if !painted {
                return false;
            }
            let dirty = rust.session.live.stroke_mut().and_then(Stroke::take_dirty);
            if let Some(rect) = dirty {
                rust.session.live.mark_stroke_tile(rect);
            }
            dirty
        };
        let Some(rect) = dirty else {
            return false;
        };
        // Frame-bounded present: the dab that opens a frame presents itself and
        // arms the flush; every later dab of that frame accumulates into one
        // pending region that `flush_present` shows on the next event-loop turn.
        let opens_frame = {
            let mut rust = self.as_mut().rust_mut();
            rust.queue_present(rect)
        };
        if let Some(present) = opens_frame {
            let t = Instant::now();
            self.as_mut().refresh_region(present);
            paint_timing::record("cpu_present_region", t.elapsed());
        }
        true
    }

    /// Present the region accumulated since the last in-stroke present, or do
    /// nothing when none is pending. The shell calls it on the next event-loop
    /// turn after a present; a consumer that needs a mid-stroke present sooner
    /// calls it directly.
    pub fn flush_present(mut self: Pin<&mut Self>) -> bool {
        let _flush = paint_timing::Scope::new("flush_present_total");
        let previewing = self.rust().session.live.preview().is_some();
        let pending = {
            let mut rust = self.as_mut().rust_mut();
            rust.take_pending_present()
        };
        if previewing {
            return pending.is_some() && self.as_mut().present_preview();
        }
        match pending {
            Some(rect) => {
                let t = Instant::now();
                self.as_mut().refresh_region(rect);
                paint_timing::record("cpu_present_region", t.elapsed());
                true
            }
            None => false,
        }
    }

    /// One sample of a previewed stroke: log it for the exact replay, rasterize
    /// its coverage at the preview level, and present on the frame-opening dab
    /// like every other stroke.
    fn preview_dab(mut self: Pin<&mut Self>, x: f64, y: f64, pressure: f64) -> bool {
        let sample = StrokeSample {
            x: x as f32,
            y: y as f32,
            pressure: pressure as f32,
        };
        let opens_frame = {
            let mut rust = self.as_mut().rust_mut();
            let Some(preview) = rust.session.live.preview_mut() else {
                return false;
            };
            preview.samples.push(sample);
            let t = Instant::now();
            let rasterized = rasterize_preview(preview, sample);
            paint_timing::record("preview_rasterize", t.elapsed());
            if !rasterized {
                return false;
            }
            let extent = preview.extent.expect("a rasterized pixel grew the extent");
            rust.queue_present(extent)
        };
        match opens_frame {
            Some(_) => self.as_mut().present_preview(),
            None => true,
        }
    }

    /// Patch the preview's stored level and emit [`region_damaged`] over the
    /// changed document rectangle so the shell repaints and schedules the next
    /// frame flush. Returns whether anything was presented.
    ///
    /// The canvas draws the forced preview level meanwhile, never `image_`; the
    /// commit's region refresh repairs level 0 and the levels below it.
    fn present_preview(mut self: Pin<&mut Self>) -> bool {
        let t = Instant::now();
        let Some(rect) = self.as_mut().preview_present() else {
            return false;
        };
        paint_timing::record("preview_present_blend", t.elapsed());
        let t = Instant::now();
        self.region_damaged(rect.left, rect.top, rect.width(), rect.height());
        paint_timing::record("preview_present_blit(C++)", t.elapsed());
        true
    }

    /// Blend the accumulated coverage over the stroke-start snapshot and patch
    /// it into the stored pyramid level. Level 0 is deliberately untouched: the
    /// commit and the cancel both repair the level from it. Returns the changed
    /// document rectangle.
    fn preview_present(mut self: Pin<&mut Self>) -> Option<PsdRect> {
        let mut rust = self.as_mut().rust_mut();
        let (rgba, rect, level, scale) = {
            let preview = rust.session.live.preview()?;
            let rect = preview.extent?;
            let rgba = preview_blend(preview, rect);
            (rgba, rect, preview.level, preview.scale as i32)
        };
        // Level 0 is deliberately untouched: only the stored level changes, so
        // the commit's region refresh repairs it when the stroke ends.
        rust.pyramid.patch_level(level as usize, &rgba, rect);
        rust.canvas_revision = rust.canvas_revision.wrapping_add(1);
        Some(PsdRect {
            top: rect.top * scale,
            left: rect.left * scale,
            bottom: rect.bottom * scale,
            right: rect.right * scale,
        })
    }

    /// One sample of a GPU stroke: place its dabs exactly as the exact stroke
    /// would, patch each dab's changed layer pixels into the working document,
    /// and present on the frame-opening dab like every other stroke. The commit
    /// therefore needs no replay: `stroke.finish()` already holds the
    /// GPU-authored layer.
    fn gpu_dab(mut self: Pin<&mut Self>, x: f64, y: f64, pressure: f64) -> bool {
        let sample = StrokeSample {
            x: x as f32,
            y: y as f32,
            pressure: pressure as f32,
        };
        let mut dabs = Vec::new();
        let mut changed: Option<PsdRect> = None;
        {
            let mut rust = self.as_mut().rust_mut();
            let rust = &mut *rust;
            let Some(placer) = rust.session.live.gpu_placer_mut() else {
                return false;
            };
            placer.feed(sample.x, sample.y, &mut dabs);
            for (cx, cy) in dabs {
                let dab = {
                    let Some(gpu) = rust.session.live.gpu_stroke_mut() else {
                        break;
                    };
                    let t = Instant::now();
                    let dab = gpu.dab(cx, cy).ok().flatten();
                    paint_timing::record("gpu_dab_dispatch_readback", t.elapsed());
                    dab
                };
                let Some((rect, plane_stride, planes)) = dab else {
                    continue;
                };
                let t = Instant::now();
                let patched = match (rust.session.live.stroke_mut(), rust.session.doc.as_mut()) {
                    (Some(stroke), Some(doc)) => {
                        stroke.patch_working_layer(doc, rect, plane_stride, &planes)
                    }
                    _ => false,
                };
                paint_timing::record("gpu_patch_working_layer", t.elapsed());
                if !patched {
                    continue;
                }
                rust.session.live.mark_stroke_tile(rect);
                changed = Some(changed.map_or(rect, |u| union_rect(u, rect)));
            }
        }
        let Some(rect) = changed else {
            return false;
        };
        // Frame-bounded present, exactly as the exact path: the dab that opens
        // a frame presents itself; the rest accumulate until `flush_present`.
        // `refresh_region` composites from the working document, so the live
        // view is exact rather than a source-over approximation.
        let opens_frame = {
            let mut rust = self.as_mut().rust_mut();
            rust.queue_present(rect)
        };
        match opens_frame {
            Some(present) => {
                self.as_mut().refresh_region(present);
                true
            }
            None => true,
        }
    }

    /// The view-pyramid level the active stroke is presenting: the preview
    /// level while a reduced preview runs, `0` otherwise (the zoom picks the
    /// level as usual). A GPU stroke presents the zoom-selected level, because
    /// its region refresh repairs every stored level.
    pub fn preview_present_level(&self) -> i32 {
        let rust = self.rust();
        rust.session
            .live
            .preview()
            .map_or(0, |preview| preview.level as i32)
    }

    pub fn end_paint(mut self: Pin<&mut Self>) -> bool {
        let t0 = Instant::now();
        let (session, pending) = {
            let mut rust = self.as_mut().rust_mut();
            // Every region a live stroke presented already reached the
            // composite; the commit presents the one still pending.
            let pending = rust.take_pending_present();
            // The commit takes the whole paint session: the stroke, its
            // backend, and the present state it accumulated.
            (rust.session.live.take_stroke_session(), pending)
        };
        let Some(session) = session else {
            tool_recording::finish(false);
            paint_timing::record("end_paint_total", t0.elapsed());
            paint_timing::report();
            return false;
        };
        let StrokeSession {
            stroke,
            mask_stroke,
            preview,
            stroke_label,
            stroke_base,
            mut stroke_tiles,
            ..
        } = session;
        if let Some(ctx) = mask_stroke {
            let ok = self
                .as_mut()
                .end_mask_paint(ctx, preview, &stroke_label, stroke_tiles);
            tool_recording::finish(ok);
            paint_timing::record("end_paint_total", t0.elapsed());
            paint_timing::report();
            return ok;
        }
        let ok = match stroke {
            None => false,
            Some(mut stroke) => {
                // A GPU stroke already patched the exact working document, so it
                // commits with no replay. A previewed stroke never fed the exact
                // stroke: replay its log now, so every committed pixel matches a
                // stroke that rasterized at full resolution from the first dab.
                let previewed = preview.is_some();
                if let Some(preview) = preview {
                    let t = Instant::now();
                    let mut rust = self.as_mut().rust_mut();
                    let rust = &mut *rust;
                    if let Some(doc) = rust.session.doc.as_mut() {
                        for sample in preview.samples {
                            stroke.sample(doc, sample);
                            if let Some(rect) = stroke.take_dirty() {
                                stroke_tiles.mark(rect);
                            }
                        }
                    }
                    paint_timing::record("commit_replay_cpu_stroke", t.elapsed());
                }
                let t = Instant::now();
                let finished = stroke.finish();
                paint_timing::record("commit_finish", t.elapsed());
                match finished {
                    None => {
                        // A stroke that changed nothing leaves no history state,
                        // so a Background it layered goes back to a Background.
                        if let Some(base) = stroke_base {
                            self.as_mut().rust_mut().session.doc = Some(base);
                        }
                        self.as_mut().recomposite();
                        false
                    }
                    Some(rect) => {
                        // A previewed stroke never presented full resolution:
                        // refresh its dirty tiles. Decomposing them into
                        // disjoint rectangles keeps a diagonal stroke from
                        // recompositing its whole bounding box; the collapse in
                        // `TileSet::regions` falls back to the union when the
                        // tiles are not worth splitting. Patching level 0 and
                        // rebuilding from it is also what overwrites the preview
                        // level, so the canvas returns to the zoom-selected one.
                        // Any other stroke presented as it went, so only the
                        // region still pending is left.
                        let regions = if previewed {
                            stroke_tiles.regions(rect)
                        } else {
                            pending.into_iter().collect()
                        };
                        let t = Instant::now();
                        self.as_mut().edit(&stroke_label, |_| Edit::Regions {
                            rects: regions,
                            notify: false,
                        });
                        paint_timing::record("commit_record_history", t.elapsed());
                        true
                    }
                }
            }
        };
        tool_recording::finish(ok);
        paint_timing::record("end_paint_total", t0.elapsed());
        paint_timing::report();
        ok
    }

    pub fn cancel_paint(mut self: Pin<&mut Self>) {
        tool_recording::cancel();
        // The stroke puts back the layer pixels it changed, and
        // `cancel_stroke` re-composites its extent into the document composite
        // that the in-stroke presents had patched (a mask stroke never patched
        // it). Presenting the extent from the composite then repairs level
        // 0, the pyramid and the canvas. A region still pending is dropped:
        // the restore below repaints the whole extent anyway.
        let dirty = {
            let mut rust = self.as_mut().rust_mut();
            let rust = &mut *rust;
            rust.clear_pending_present();
            // The restore below refreshes the stroke's extent from level 0,
            // which rebuilds the stored levels and so drops the preview too.
            // Taking the paint session drops its preview, its GPU stroke and
            // its dirty tiles with it.
            rust.cancel_stroke()
        };
        match dirty {
            Some(rect) => {
                let t = Instant::now();
                let blit = self.as_mut().rust_mut().refresh_from_composite(rect);
                if let Some(damaged) = blit {
                    self.as_mut().region_damaged(
                        damaged.left,
                        damaged.top,
                        damaged.width(),
                        damaged.height(),
                    );
                }
                paint_timing::record("cancel_refresh_region", t.elapsed());
            }
            None => {
                let t = Instant::now();
                self.as_mut().recomposite();
                paint_timing::record("cancel_recomposite", t.elapsed());
            }
        }
        paint_timing::report();
    }

    pub fn is_painting(&self) -> bool {
        self.rust().session.live.stroke().is_some()
            || self.rust().session.live.mask_stroke().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::{gpu_paint_mode, preview_blend, preview_level, rasterize_preview};
    use pictura_core::PsdRect;
    use pictura_paint::{tip_coverage, PaintMode, Rgba, StrokeConfig, StrokeSample};
    use pictura_render::gpu::GpuPaintMode;

    use super::PreviewStroke;

    fn rect(w: u32, h: u32) -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        }
    }

    fn preview(
        w: u32,
        h: u32,
        level: u32,
        snapshot: Vec<u8>,
        coverage: Vec<u8>,
        cfg: StrokeConfig,
    ) -> PreviewStroke {
        PreviewStroke {
            level,
            scale: 1 << level,
            width: w,
            height: h,
            snapshot,
            coverage,
            extent: Some(rect(w, h)),
            samples: Vec::new(),
            cfg,
            mask: None,
        }
    }

    #[test]
    fn gpu_mode_covers_normal_and_clear_only() {
        assert_eq!(
            gpu_paint_mode(PaintMode::Normal),
            Some(GpuPaintMode::Normal)
        );
        assert_eq!(gpu_paint_mode(PaintMode::Clear), Some(GpuPaintMode::Clear));
        assert_eq!(gpu_paint_mode(PaintMode::Dissolve), None);
        assert_eq!(gpu_paint_mode(PaintMode::Behind), None);
        assert_eq!(gpu_paint_mode(PaintMode::Multiply), None);
    }

    #[test]
    fn a_dab_that_fits_the_budget_is_not_previewed() {
        assert_eq!(preview_level(500, 4000, 4000), None, "500 px fits");
        assert_eq!(preview_level(1, 4000, 4000), None);
        assert_eq!(preview_level(0, 4000, 4000), None);
    }

    #[test]
    fn a_dab_over_the_budget_previews_at_a_level_that_fits() {
        for (diameter, w, h) in [
            (600, 4000, 4000),
            (1024, 4000, 4000),
            (2000, 4000, 4000),
            (5000, 4000, 4000),
            (5000, 16000, 16000),
        ] {
            let level = preview_level(diameter, w, h)
                .unwrap_or_else(|| panic!("{diameter} on {w}x{h} must preview"));
            let d = (diameter as u64 + 2).min(w as u64).min(h as u64);
            let area = d * d;
            assert!(
                (area >> (2 * level)) <= 262_144,
                "{diameter} on {w}x{h} previews at {level} with {} px",
                area >> (2 * level)
            );
        }
    }

    #[test]
    fn a_bounded_brush_previews_at_the_smallest_level_that_fits() {
        // A 16000² document with a 5000 px brush is the one case the starting
        // level does not cover on its own.
        assert_eq!(preview_level(5000, 4000, 4000), Some(3));
        assert_eq!(preview_level(5000, 16000, 16000), Some(4));
        assert_eq!(preview_level(600, 4000, 4000), Some(3));
    }

    #[test]
    fn preview_blend_is_source_over_and_clear_removes_the_snapshot() {
        let snapshot = vec![10u8, 20, 30, 255];
        let blue = Rgba {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        };
        let normal = |coverage| {
            let cfg = StrokeConfig {
                color: blue,
                mode: PaintMode::Normal,
                ..Default::default()
            };
            preview(1, 1, 3, snapshot.clone(), vec![coverage], cfg)
        };
        assert_eq!(
            preview_blend(&normal(0), rect(1, 1)),
            snapshot,
            "zero coverage leaves the snapshot untouched"
        );
        assert_eq!(
            preview_blend(&normal(255), rect(1, 1)),
            vec![0, 0, 255, 255],
            "full opaque coverage is exactly the paint colour"
        );

        let clear = preview(
            1,
            1,
            3,
            snapshot.clone(),
            vec![255],
            StrokeConfig {
                color: blue,
                mode: PaintMode::Clear,
                ..Default::default()
            },
        );
        assert_eq!(
            preview_blend(&clear, rect(1, 1)),
            vec![0, 0, 0, 0],
            "Clear removes the snapshot's coverage"
        );

        // Clear ignores the foreground alpha (the exact stroke does), so a
        // semi-transparent colour clears exactly as an opaque one does.
        let clear_translucent = preview(
            1,
            1,
            3,
            snapshot.clone(),
            vec![255],
            StrokeConfig {
                color: Rgba {
                    r: 0,
                    g: 0,
                    b: 255,
                    a: 128,
                },
                mode: PaintMode::Clear,
                ..Default::default()
            },
        );
        assert_eq!(
            preview_blend(&clear_translucent, rect(1, 1)),
            preview_blend(&clear, rect(1, 1)),
            "Clear ignores the foreground alpha"
        );
    }

    #[test]
    fn rasterize_preview_samples_the_tip_at_scaled_offsets() {
        let cfg = StrokeConfig {
            diameter: 512,
            hardness: 100,
            ..Default::default()
        };
        let (w, h) = (64u32, 64u32);
        let mut p = preview(
            w,
            h,
            3,
            vec![0; (w * h * 4) as usize],
            vec![0; (w * h) as usize],
            cfg,
        );
        // The sample is document-space; level 3 means an 8x reduction.
        let centre = 32.0 * p.scale as f32;
        assert!(rasterize_preview(
            &mut p,
            StrokeSample {
                x: centre,
                y: centre,
                pressure: 1.0,
            }
        ));
        let flow = cfg.flow as f32 / 100.0;
        assert_eq!(p.coverage[32 * w as usize + 32], 255, "centre is full");
        for (lx, ly) in [(32usize, 20usize), (40, 32), (20, 20), (32, 44)] {
            let dx = (lx as f32 + 0.5 - 32.0) * p.scale as f32;
            let dy = (ly as f32 + 0.5 - 32.0) * p.scale as f32;
            let tip = tip_coverage(&cfg, dx, dy);
            let expected = ((flow * tip).clamp(0.0, 1.0) * 255.0).round() as u8;
            assert_eq!(
                p.coverage[ly * w as usize + lx],
                expected,
                "reduced coverage at level pixel ({lx},{ly}) is the full tip at its scaled offset"
            );
        }
    }
}
