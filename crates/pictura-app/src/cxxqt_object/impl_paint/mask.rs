//! Painting a layer mask: the exact stroke on the mask's coverage document, the
//! GPU stroke for a large brush, and, when the GPU stroke is not taken, the
//! reduced-resolution preview replayed exactly at the commit.

use super::super::helpers::*;
use super::super::helpers_composite::display_rgba_bytes;
use super::super::qobject;
use super::super::state::{MaskPreview, MaskStroke, PictureViewRust, PreviewStroke};
use super::{gpu_paint_mode, mask_gpu, preview_level};
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::PsdRect;
use pictura_paint::spacing::DabPlacer;
use pictura_paint::{PaintMode, Stroke, StrokeConfig, StrokeSample};
use pictura_render::gpu::{GpuStroke, GpuStrokeParams};
use std::time::Instant;

/// `rect` in mask-local coordinates moved to the document space of a mask at
/// `origin`.
fn to_document(rect: PsdRect, origin: PsdRect) -> PsdRect {
    PsdRect {
        top: rect.top + origin.top,
        left: rect.left + origin.left,
        bottom: rect.bottom + origin.top,
        right: rect.right + origin.left,
    }
}

/// The reduced-resolution preview a mask stroke of `cfg` on the mask of the
/// layer at `path` starts with, or `None` when its dab fits the raster budget
/// or the preview level is not built.
fn mask_preview(rust: &PictureViewRust, path: &str, cfg: StrokeConfig) -> Option<PreviewStroke> {
    let doc = rust.doc.as_ref()?;
    let level = preview_level(cfg.diameter as i32, doc.width, doc.height)?;
    let t = Instant::now();
    let mut preview = PreviewStroke::new(level, &rust.pyramid, rust.level0.as_ref()?, cfg)?;
    let whole = PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    };
    let proxy = pictura_render::FilterProxy::new(doc, None, whole, 1.0 / preview.scale as f64)?;
    let extremes = proxy.mask_extremes(path)?;
    let display = |buffer| display_rgba_bytes(&pictura_codec::buffer_to_srgb(doc, buffer));
    preview.mask = Some(MaskPreview {
        width: extremes.width,
        height: extremes.height,
        hidden: display(&extremes.hidden),
        revealed: display(&extremes.revealed),
        coverage: extremes.coverage,
    });
    paint_timing::record("mask_preview_new (proxy)", t.elapsed());
    Some(preview)
}

/// The premultiplied level-space region for `rect` of a mask stroke's preview:
/// the stroke-start snapshot moved toward the revealed (or hidden) composite by
/// as much as the accumulated coverage moves each pixel's mask value. Behind
/// presents no change, since the mask document is opaque.
///
/// The proxy grid can be a pixel short of the level's, so its last row and
/// column stand in for the missing one.
pub(super) fn mask_preview_blend(
    preview: &PreviewStroke,
    mask: &MaskPreview,
    rect: PsdRect,
) -> Vec<u8> {
    let (rw, rh) = (rect.width().max(0) as usize, rect.height().max(0) as usize);
    if rw == 0 || rh == 0 || mask.width == 0 || mask.height == 0 {
        return Vec::new();
    }
    let cfg = preview.cfg;
    let strength = if cfg.mode == PaintMode::Behind {
        0.0
    } else {
        cfg.opacity as f32 / 100.0 * cfg.color.a as f32 / 255.0
    };
    // `mask_stroke_config` paints the foreground's luma in every channel.
    let value = cfg.color.r as f32;
    let (mw, mh) = (mask.width as usize, mask.height as usize);
    let mut rgba = vec![0u8; rw * rh * 4];
    for y in 0..rh {
        let ly = rect.top as usize + y;
        let my = ly.min(mh - 1);
        for x in 0..rw {
            let lx = rect.left as usize + x;
            let li = ly * preview.width as usize + lx;
            let mi = my * mw + lx.min(mw - 1);
            let coverage = if cfg.wet_edges {
                pictura_paint::wet_edge(preview.coverage[li])
            } else {
                preview.coverage[li]
            };
            let k = strength * coverage as f32 / 255.0;
            let start = mask.coverage[mi] as f32;
            let t = (value - start) * k / 255.0;
            let o = (y * rw + x) * 4;
            for c in 0..4 {
                let delta = mask.revealed[mi * 4 + c] as f32 - mask.hidden[mi * 4 + c] as f32;
                let v = preview.snapshot[li * 4 + c] as f32 + t * delta;
                rgba[o + c] = v.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    rgba
}

/// The stroke-start snapshot of `preview` over the level-space `rect`.
fn snapshot_region(preview: &PreviewStroke, rect: PsdRect) -> Vec<u8> {
    let (rw, rh) = (rect.width().max(0) as usize, rect.height().max(0) as usize);
    let mut rgba = Vec::with_capacity(rw * rh * 4);
    for y in 0..rh {
        let row = (rect.top as usize + y) * preview.width as usize + rect.left as usize;
        rgba.extend_from_slice(&preview.snapshot[row * 4..(row + rw) * 4]);
    }
    rgba
}

impl qobject::PictureView {
    /// Begin a stroke on the raster mask of the layer at `path`: the mask is
    /// exposed as a mask-sized grayscale document and an ordinary [`Stroke`]
    /// runs on it, so the mask gains the exact brush behaviour. A dab over the
    /// raster budget runs the GPU stroke, or previews at a pyramid level when
    /// the GPU stroke is not taken. False without a mask, on a zero-sized mask
    /// rectangle, or when the stroke is refused.
    pub(in super::super) fn begin_mask_paint(
        mut self: Pin<&mut Self>,
        path: &str,
        cfg: StrokeConfig,
        label: &str,
    ) -> bool {
        let built = {
            let rust = self.rust();
            if rust.stroke.is_some() || rust.mask_stroke.is_some() {
                return false;
            }
            let Some(doc) = rust.doc.as_ref() else {
                return false;
            };
            let Some(mask) = pictura_render::mask_document_coverage(doc, path) else {
                return false;
            };
            Stroke::begin_at(&mask.document, "0", cfg)
                .ok()
                .map(|stroke| MaskStroke {
                    path: path.to_string(),
                    rect: mask.rect,
                    document: mask.document,
                    stroke,
                })
        };
        let Some(mask_stroke) = built else {
            return false;
        };
        // A large mask brush runs the same GPU stroke the pixel path uses, with
        // one channel: the shader blends the coverage byte only, so the mask
        // gains accelerator parity without an RGBA round-trip. The shader has
        // no paper grain or wet-edge curve, so such a tip stays on the CPU.
        let gpu = {
            let (mw, mh) = (mask_stroke.document.width, mask_stroke.document.height);
            match (
                preview_level(cfg.diameter as i32, mw, mh),
                gpu_paint_mode(cfg.mode).filter(|_| cfg.grain == 0 && !cfg.wet_edges),
                self.rust().gpu_compute,
            ) {
                (Some(_), Some(mode), true) => {
                    let seed = mask_gpu::mask_rgba_seed(&mask_stroke.document);
                    let tip = mask_stroke.stroke.tip_params();
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
                        channels: 1,
                    };
                    GpuStroke::new(mw, mh, &seed, params).ok()
                }
                _ => None,
            }
        };
        // Without the GPU stroke (no compute, or a mode only the CPU raster
        // models), a large brush previews at a pyramid level like a pixel one.
        let preview = match gpu {
            Some(_) => None,
            None => mask_preview(self.rust(), path, cfg.sanitized()),
        };
        let has_gpu = gpu.is_some();
        let mut rust = self.as_mut().rust_mut();
        rust.mask_stroke = Some(mask_stroke);
        rust.stroke_label = label.to_string();
        rust.clear_pending_present();
        rust.gpu_stroke = gpu;
        rust.gpu_placer = if has_gpu {
            Some(DabPlacer::new(cfg.spacing, cfg.diameter as f32))
        } else {
            None
        };
        rust.preview = preview;
        let dims = rust.doc.as_ref().map(|doc| (doc.width, doc.height));
        if let Some((width, height)) = dims {
            rust.stroke_tiles.reset(width, height);
        }
        true
    }

    /// One dab of a mask stroke: sample the mask-local point on the mask
    /// document, copy the changed region back into the real mask, and present
    /// it. Returns whether a pixel changed.
    pub(super) fn mask_dab(mut self: Pin<&mut Self>, x: f64, y: f64, pressure: f64) -> bool {
        if self.rust().gpu_stroke.is_some() {
            return self.as_mut().mask_gpu_dab(x, y, pressure);
        }
        // A previewed mask stroke previews like a pixel one; the exact mask
        // stroke runs at the commit.
        if self.rust().preview.is_some() {
            return self.as_mut().preview_dab(x, y, pressure);
        }
        let (dirty, rect) = {
            let mut rust = self.as_mut().rust_mut();
            let rust = &mut *rust;
            let Some(ctx) = rust.mask_stroke.as_mut() else {
                return false;
            };
            let sample = StrokeSample {
                x: (x - ctx.rect.left as f64) as f32,
                y: (y - ctx.rect.top as f64) as f32,
                pressure: pressure as f32,
            };
            if !ctx.stroke.sample(&mut ctx.document, sample) {
                return false;
            }
            (ctx.stroke.take_dirty(), ctx.rect)
        };
        let Some(dirty) = dirty else {
            return false;
        };
        {
            let mut rust = self.as_mut().rust_mut();
            let rust = &mut *rust;
            let (Some(doc), Some(ctx)) = (rust.doc.as_mut(), rust.mask_stroke.as_ref()) else {
                return false;
            };
            pictura_render::write_mask_back(doc, &ctx.path, &ctx.document, Some(dirty));
        }
        if let Some(region) = self
            .as_mut()
            .rust_mut()
            .queue_present(to_document(dirty, rect))
        {
            self.as_mut().refresh_region(region);
        }
        true
    }

    /// Commit a mask stroke. The dabs of an exact or GPU stroke already wrote
    /// the mask. A previewed stroke never fed the exact stroke, so its log is
    /// replayed on the mask document and written back once, and the preview's
    /// level goes back to its stroke-start snapshot first: the preview can paint
    /// level pixels the exact stroke never touches (a dab crossing the mask's
    /// rectangle, a sample the exact spacing skips), which the refresh of the
    /// exact stroke's regions would otherwise leave behind. The stroke's regions
    /// are then refreshed and recorded; a stroke that changed nothing
    /// recomposites and records nothing.
    pub(super) fn end_mask_paint(
        mut self: Pin<&mut Self>,
        mut ctx: MaskStroke,
        preview: Option<PreviewStroke>,
        label: &str,
    ) -> bool {
        if let Some(preview) = preview {
            let t = Instant::now();
            let mut rust = self.as_mut().rust_mut();
            let rust = &mut *rust;
            if let Some(extent) = preview.extent {
                let restore = snapshot_region(&preview, extent);
                rust.pyramid
                    .patch_level(preview.level as usize, &restore, extent);
                rust.canvas_revision = rust.canvas_revision.wrapping_add(1);
            }
            for sample in preview.samples {
                let local = StrokeSample {
                    x: sample.x - ctx.rect.left as f32,
                    y: sample.y - ctx.rect.top as f32,
                    ..sample
                };
                ctx.stroke.sample(&mut ctx.document, local);
                if let Some(rect) = ctx.stroke.take_dirty() {
                    rust.stroke_tiles.mark(to_document(rect, ctx.rect));
                }
            }
            if let (Some(dirty), Some(doc)) = (ctx.stroke.dirty(), rust.doc.as_mut()) {
                pictura_render::write_mask_back(doc, &ctx.path, &ctx.document, Some(dirty));
            }
            paint_timing::record("commit_replay_mask_stroke", t.elapsed());
        }
        match ctx.stroke.finish() {
            None => {
                self.as_mut().recomposite();
                false
            }
            Some(rect) => {
                let rect = to_document(rect, ctx.rect);
                let regions = self.as_mut().rust_mut().take_stroke_regions(rect);
                self.as_mut().edit(label, |_| Edit::Regions {
                    rects: regions,
                    notify: false,
                });
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{mask_preview_blend, snapshot_region, MaskPreview, PreviewStroke};
    use pictura_core::PsdRect;
    use pictura_paint::{PaintMode, Rgba, StrokeConfig};

    const PIXEL: PsdRect = PsdRect {
        top: 0,
        left: 0,
        bottom: 1,
        right: 1,
    };

    fn black(mode: PaintMode) -> StrokeConfig {
        StrokeConfig {
            color: Rgba {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            mode,
            ..Default::default()
        }
    }

    fn preview(coverage: u8, snapshot: [u8; 4], cfg: StrokeConfig) -> PreviewStroke {
        PreviewStroke {
            level: 3,
            scale: 8,
            width: 1,
            height: 1,
            snapshot: snapshot.to_vec(),
            coverage: vec![coverage],
            extent: Some(PIXEL),
            samples: Vec::new(),
            cfg,
            mask: None,
        }
    }

    fn revealed_mask() -> MaskPreview {
        MaskPreview {
            width: 1,
            height: 1,
            coverage: vec![255],
            hidden: vec![255, 255, 255, 255],
            revealed: vec![34, 68, 170, 255],
        }
    }

    #[test]
    fn mask_preview_moves_the_snapshot_by_the_coverage_change() {
        let mask = revealed_mask();
        let snapshot = [34, 68, 170, 255];
        assert_eq!(
            mask_preview_blend(
                &preview(0, snapshot, black(PaintMode::Normal)),
                &mask,
                PIXEL
            ),
            snapshot.to_vec(),
            "zero coverage presents the snapshot"
        );
        assert_eq!(
            mask_preview_blend(
                &preview(255, snapshot, black(PaintMode::Normal)),
                &mask,
                PIXEL
            ),
            mask.hidden,
            "full black coverage over a revealing mask presents the hidden composite"
        );
        let half = mask_preview_blend(
            &preview(128, snapshot, black(PaintMode::Normal)),
            &mask,
            PIXEL,
        );
        assert!(
            (140..150).contains(&half[0]),
            "half coverage lands halfway: {half:?}"
        );
        assert_eq!(
            mask_preview_blend(
                &preview(255, snapshot, black(PaintMode::Behind)),
                &mask,
                PIXEL
            ),
            snapshot.to_vec(),
            "Behind changes nothing on the opaque mask"
        );
    }

    #[test]
    fn mask_preview_applies_wet_edges_like_the_commit() {
        let mask = revealed_mask();
        let snapshot = [34, 68, 170, 255];
        let wet = StrokeConfig {
            wet_edges: true,
            ..black(PaintMode::Normal)
        };
        let out = mask_preview_blend(&preview(255, snapshot, wet), &mask, PIXEL);
        let k = pictura_paint::wet_edge(255) as f32 / 255.0;
        let expected = 34.0 + k * (255.0 - 34.0);
        assert!(
            (out[0] as f32 - expected).abs() <= 1.0,
            "full coverage previews the wet-edged interior, not the full hidden tone: {out:?}"
        );
    }

    #[test]
    fn mask_preview_clamps_to_the_proxy_grid() {
        // A 2×1 level over a 1×1 proxy: the second column reads the proxy's last.
        let mut p = preview(255, [34, 68, 170, 255], black(PaintMode::Normal));
        p.width = 2;
        p.snapshot.extend_from_slice(&[34, 68, 170, 255]);
        p.coverage.push(255);
        let rect = PsdRect { right: 2, ..PIXEL };
        let mask = revealed_mask();
        let out = mask_preview_blend(&p, &mask, rect);
        assert_eq!(out[4..], mask.hidden[..]);
    }

    #[test]
    fn a_dab_past_the_mask_edge_previews_there_and_restores() {
        use super::super::super::helpers_composite::display_rgba_bytes;
        use super::super::super::tests::pixel_layer;
        use super::super::rasterize_preview;
        use pictura_core::{BitDepth, ColorMode, Document, LayerMask};
        use pictura_paint::StrokeSample;

        // A 64² layer whose mask covers only its top 32 rows; below, the
        // mask's default colour reveals the layer.
        let mut doc = Document::new(64, 64, ColorMode::Rgb, BitDepth::Eight);
        let mut layer = pixel_layer("top", 64, 64, (200, 100, 50));
        layer.mask = Some(LayerMask {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 32,
                right: 64,
            },
            default_color: 255,
            data: Some(vec![255u8; 64 * 32].into()),
            ..Default::default()
        });
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        let whole = PsdRect {
            top: 0,
            left: 0,
            bottom: 64,
            right: 64,
        };
        let proxy = pictura_render::FilterProxy::new(&doc, None, whole, 1.0 / 8.0).unwrap();
        let ex = proxy.mask_extremes("0").unwrap();
        let display = |b| display_rgba_bytes(&pictura_codec::buffer_to_srgb(&doc, b));
        let revealed = display(&ex.revealed);
        let mask = MaskPreview {
            width: ex.width,
            height: ex.height,
            hidden: display(&ex.hidden),
            revealed: revealed.clone(),
            coverage: ex.coverage,
        };
        let cfg = StrokeConfig {
            diameter: 32,
            hardness: 100,
            ..black(PaintMode::Normal)
        };
        let mut p = preview(0, [0; 4], cfg);
        (p.width, p.height) = (8, 8);
        p.coverage = vec![0; 64];
        p.snapshot = revealed;
        assert!(rasterize_preview(
            &mut p,
            StrokeSample {
                x: 32.0,
                y: 30.0,
                pressure: 1.0
            }
        ));
        let extent = p.extent.unwrap();
        assert!(extent.bottom > 4, "the dab crosses the mask's last row");
        // Level pixel (4, 4) covers document rows 32..40, below the mask.
        let below = PsdRect {
            top: 4,
            left: 4,
            bottom: 5,
            right: 5,
        };
        let start = snapshot_region(&p, below);
        assert_ne!(
            mask_preview_blend(&p, &mask, below),
            start,
            "the preview paints past the mask's rectangle"
        );
        let mask_doc = pictura_render::mask_document_coverage(&doc, "0").unwrap();
        assert_eq!(
            mask_doc.document.height, 32,
            "the exact stroke cannot reach it"
        );
        let restore = snapshot_region(&p, extent);
        let row = (below.top - extent.top) as usize * extent.width() as usize;
        let i = (row + (below.left - extent.left) as usize) * 4;
        assert_eq!(
            restore[i..i + 4],
            start[..],
            "the commit restore puts it back"
        );
    }

    #[test]
    fn snapshot_region_crops_the_stroke_start_level() {
        // A 3×2 level whose pixel (x, y) is [x, y, 0, 255].
        let mut p = preview(0, [0; 4], black(PaintMode::Normal));
        (p.width, p.height) = (3, 2);
        p.snapshot = (0..2u8)
            .flat_map(|y| (0..3u8).flat_map(move |x| [x, y, 0, 255]))
            .collect();
        let rect = PsdRect {
            top: 1,
            left: 1,
            bottom: 2,
            right: 3,
        };
        assert_eq!(snapshot_region(&p, rect), vec![1, 1, 0, 255, 2, 1, 0, 255]);
    }
}
