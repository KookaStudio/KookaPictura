//! The paint session: the stroke on a layer's pixels or on a mask's coverage,
//! the backend that runs it, and the present state it accumulates.

mod mask;
mod preview;

pub(in crate::cxxqt_object) use mask::MaskStroke;
pub(in crate::cxxqt_object) use preview::{MaskPreview, PreviewStroke};

use crate::cxxqt_object::helpers::TileSet;
use pictura_core::{Document, PsdRect};
use pictura_paint::spacing::DabPlacer;
use pictura_paint::Stroke;
use pictura_render::gpu::GpuStroke;

/// The live paint session.
///
/// One gesture owns one of these: the layer stroke or the mask stroke, the
/// backend that rasterizes it, and the present frame it accumulates. The two
/// strokes are mutually exclusive — the target is resolved before the session
/// begins — so the session never holds both.
#[derive(Default)]
pub(in crate::cxxqt_object) struct StrokeSession {
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
