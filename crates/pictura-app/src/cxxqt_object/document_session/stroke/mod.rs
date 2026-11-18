//! The paint session: the stroke on a layer's pixels or on a mask's coverage,
//! the backend that runs it, and the present state it accumulates.

mod mask;
mod preview;

pub(in crate::cxxqt_object) use mask::MaskStroke;
pub(in crate::cxxqt_object) use preview::{MaskPreview, PreviewSample, PreviewStroke};

use super::SessionKind;
use crate::cxxqt_object::fade::FadeCapture;
use crate::cxxqt_object::helpers::{mask_paint_pad_for, paint_pad_for, TileSet};
use crate::cxxqt_object::impl_paint::tool_recording::{self, ToolStrokeKind};
use crate::cxxqt_object::state::PictureViewRust;
use pictura_core::{Document, PsdRect};
use pictura_paint::spacing::DabPlacer;
use pictura_paint::{Stroke, StrokeConfig};
use pictura_render::gpu::GpuStroke;

/// The live paint session.
///
/// One gesture owns one of these: the layer stroke or the mask stroke, the
/// backend that rasterizes it, and the present frame it accumulates. The two
/// strokes are mutually exclusive — the target is resolved before the session
/// begins — so the session never holds both.
#[derive(Default)]
pub(in crate::cxxqt_object) struct StrokeSession {
    pub(in crate::cxxqt_object) quick_mask:
        Option<crate::cxxqt_object::quick_mask::QuickMaskStroke>,
    /// The CPU stroke on a layer's pixels.
    pub(in crate::cxxqt_object) stroke: Option<Stroke>,
    /// The stroke on a layer mask's coverage instead of the layer's pixels.
    pub(in crate::cxxqt_object) mask_stroke: Option<MaskStroke>,
    /// The full-resolution GPU stroke presented while the button is down.
    pub(in crate::cxxqt_object) gpu_stroke: Option<GpuStroke>,
    /// The GPU present's dab placer, so its dabs space exactly as the CPU
    /// stroke's do.
    pub(in crate::cxxqt_object) gpu_placer: Option<DabPlacer>,
    /// The reduced-resolution stroke presented while a large brush is down;
    /// `None` whenever the dab fits the raster budget.
    pub(in crate::cxxqt_object) preview: Option<PreviewStroke>,
    /// The history label the commit records.
    pub(in crate::cxxqt_object) stroke_label: String,
    /// How far the painted layer's effects and mask feather can move the change
    /// outside a dab; every refresh rectangle grows by this.
    pub(in crate::cxxqt_object) paint_pad: i32,
    /// The document before a stroke that had to change its structure to begin
    /// (the Background Eraser layering a Background); cancelling restores it.
    pub(in crate::cxxqt_object) stroke_base: Option<Document>,
    /// The target layer before the stroke, for `Edit > Fade`.
    pub(in crate::cxxqt_object) fade_capture: Option<crate::cxxqt_object::fade::FadeCapture>,
    /// The stroke's dirty area as document-space tiles, rebuilt per dab and
    /// decomposed into disjoint rectangles at commit.
    pub(in crate::cxxqt_object) stroke_tiles: TileSet,
    /// Dabs received since the last in-stroke present, waiting for
    /// `flush_present`.
    pub(in crate::cxxqt_object) pending_present: Option<PsdRect>,
    /// True from the present that opens a frame until its flush runs, so the
    /// frame's later dabs accumulate instead of presenting again.
    pub(in crate::cxxqt_object) present_flush_due: bool,
}

/// What a stroke begin paints.
pub(in crate::cxxqt_object) enum StrokeTarget {
    /// A stroke on the layer at `path`.
    Layer { path: String, stroke: Box<Stroke> },
    /// A stroke on a layer mask's coverage.
    Mask(Box<MaskStroke>),
}

/// A built stroke and the backend the caller selected for it.
///
/// The constructor owns the session invariant; the caller owns the target
/// resolution, memory gate, and GPU/preview choice, because those differ by
/// tool and by layer versus mask.
pub(in crate::cxxqt_object) struct StrokePlan {
    pub(in crate::cxxqt_object) target: StrokeTarget,
    pub(in crate::cxxqt_object) gpu_stroke: Option<GpuStroke>,
    pub(in crate::cxxqt_object) gpu_placer: Option<DabPlacer>,
    pub(in crate::cxxqt_object) preview: Option<PreviewStroke>,
}

impl StrokePlan {
    /// A layer stroke plan with no accelerator.
    pub(in crate::cxxqt_object) fn layer(path: &str, stroke: Stroke) -> Self {
        Self {
            target: StrokeTarget::Layer {
                path: path.to_string(),
                stroke: Box::new(stroke),
            },
            gpu_stroke: None,
            gpu_placer: None,
            preview: None,
        }
    }

    /// A mask stroke plan with no accelerator.
    pub(in crate::cxxqt_object) fn mask(stroke: MaskStroke) -> Self {
        Self {
            target: StrokeTarget::Mask(Box::new(stroke)),
            gpu_stroke: None,
            gpu_placer: None,
            preview: None,
        }
    }

    /// Attach the backend the caller selected.
    pub(in crate::cxxqt_object) fn with_backend(
        mut self,
        gpu_stroke: Option<GpuStroke>,
        gpu_placer: Option<DabPlacer>,
        preview: Option<PreviewStroke>,
    ) -> Self {
        self.gpu_stroke = gpu_stroke;
        self.gpu_placer = gpu_placer;
        self.preview = preview;
        self
    }
}

/// A finished layer stroke, ready for the presenter.
pub(in crate::cxxqt_object) struct LayerStrokeFinish {
    pub(in crate::cxxqt_object) stroke: Stroke,
    pub(in crate::cxxqt_object) preview: Option<PreviewStroke>,
    pub(in crate::cxxqt_object) label: String,
    pub(in crate::cxxqt_object) stroke_base: Option<Document>,
    pub(in crate::cxxqt_object) paint_pad: i32,
    pub(in crate::cxxqt_object) fade_capture: Option<FadeCapture>,
    pub(in crate::cxxqt_object) stroke_tiles: TileSet,
    pub(in crate::cxxqt_object) pending: Option<PsdRect>,
}

/// A finished mask stroke, ready for the presenter.
pub(in crate::cxxqt_object) struct MaskStrokeFinish {
    pub(in crate::cxxqt_object) mask_stroke: MaskStroke,
    pub(in crate::cxxqt_object) preview: Option<PreviewStroke>,
    pub(in crate::cxxqt_object) label: String,
    pub(in crate::cxxqt_object) paint_pad: i32,
    pub(in crate::cxxqt_object) stroke_tiles: TileSet,
}

/// The typed result of ending a paint session.
pub(in crate::cxxqt_object) enum StrokeFinish {
    /// No stroke was live.
    None,
    /// A layer stroke, with everything its commit or cancel needs.
    Layer(LayerStrokeFinish),
    /// A mask stroke, with everything its commit or cancel needs.
    Mask(MaskStrokeFinish),
    /// A quick-mask stroke, which `end_paint` handles before it takes the slot.
    QuickMask,
}

impl PictureViewRust {
    /// Begin a stroke through the one lifecycle constructor.
    ///
    /// The constructor owns the slot check, label, pad, fade capture, pending
    /// present, tile set, and recording. `build` runs only after the slot check,
    /// so a refused begin leaves the live state untouched.
    pub(in crate::cxxqt_object) fn begin_stroke(
        &mut self,
        kind: SessionKind,
        label: &str,
        cfg: &StrokeConfig,
        recording: ToolStrokeKind,
        build: impl FnOnce(&PictureViewRust) -> Option<StrokePlan>,
    ) -> bool {
        if self.session.read_only() || !self.slot_allows(kind) {
            return false;
        }
        let Some(plan) = build(self) else {
            return false;
        };
        let Some(doc) = self.session.doc.as_ref() else {
            return false;
        };
        let (paint_pad, fade_capture) = match &plan.target {
            StrokeTarget::Layer { path, .. } => (
                paint_pad_for(doc, path),
                crate::cxxqt_object::fade::capture(self),
            ),
            StrokeTarget::Mask(mask) => (mask_paint_pad_for(doc, &mask.path), None),
        };
        let (stroke, mask_stroke) = match plan.target {
            StrokeTarget::Layer { stroke, .. } => (Some(*stroke), None),
            StrokeTarget::Mask(mask) => (None, Some(*mask)),
        };
        let session = StrokeSession {
            quick_mask: None,
            stroke,
            mask_stroke,
            gpu_stroke: plan.gpu_stroke,
            gpu_placer: plan.gpu_placer,
            preview: plan.preview,
            stroke_label: label.to_string(),
            paint_pad,
            stroke_base: None,
            fade_capture,
            stroke_tiles: TileSet::default(),
            pending_present: None,
            present_flush_due: false,
        };
        self.session.live.set_stroke_session(session);
        self.clear_pending_present();
        if let Some((width, height)) = self.session.doc.as_ref().map(|doc| (doc.width, doc.height))
        {
            self.session.live.reset_stroke_tiles(width, height);
        }
        tool_recording::begin(label, &cfg.sanitized(), recording);
        true
    }
}

impl StrokeSession {
    /// Map the live session to the typed result `end_paint` consumes.
    pub(in crate::cxxqt_object) fn finish(self, pending: Option<PsdRect>) -> StrokeFinish {
        let StrokeSession {
            quick_mask,
            stroke,
            mask_stroke,
            preview,
            stroke_label,
            paint_pad,
            stroke_base,
            fade_capture,
            stroke_tiles,
            ..
        } = self;
        if quick_mask.is_some() {
            return StrokeFinish::QuickMask;
        }
        if let Some(mask_stroke) = mask_stroke {
            return StrokeFinish::Mask(MaskStrokeFinish {
                mask_stroke,
                preview,
                label: stroke_label,
                paint_pad,
                stroke_tiles,
            });
        }
        let Some(stroke) = stroke else {
            return StrokeFinish::None;
        };
        StrokeFinish::Layer(LayerStrokeFinish {
            stroke,
            preview,
            label: stroke_label,
            stroke_base,
            paint_pad,
            fade_capture,
            stroke_tiles,
            pending,
        })
    }
}
