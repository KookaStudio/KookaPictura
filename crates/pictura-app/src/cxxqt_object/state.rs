use crate::history::History;
use cxx_qt_lib::QImage;
use pictura_core::{Document, Layer, PixelBuffer, PsdRect};
use pictura_paint::spacing::DabPlacer;
use pictura_paint::{HealStroke, Stroke, StrokeSample};
use pictura_render::gpu::GpuStroke;
use pictura_render::{CanvasDamage, ViewPyramid};
use pictura_select::Selection;
use std::collections::HashMap;

use super::helpers::TileSet;

/// The live Free Transform gesture mode. `Free` is the existing similarity
/// transform; the other three edit a document-space target quad.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum TransformMode {
    #[default]
    Free,
    Skew,
    Distort,
    Perspective,
}

impl TransformMode {
    /// Parse the command mode name, or `None` for an unknown string.
    pub(super) fn parse(name: &str) -> Option<Self> {
        match name {
            "skew" => Some(Self::Skew),
            "distort" => Some(Self::Distort),
            "perspective" => Some(Self::Perspective),
            _ => None,
        }
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Skew => "skew",
            Self::Distort => "distort",
            Self::Perspective => "perspective",
        }
    }

    pub(super) fn is_projective(self) -> bool {
        self != Self::Free
    }
}

/// The live Free Transform session: the target path, the source rect, the
/// current similarity transform about that rect's centre, and the drag state.
pub struct TransformSession {
    pub(super) path: String,
    pub(super) orig_rect: PsdRect,
    pub(super) scale_x: f64,
    pub(super) scale_y: f64,
    pub(super) angle: f64,
    pub(super) dx: f64,
    pub(super) dy: f64,
    /// Active handle: 0..=7 scale, 8 rotate, 9 move, -1 none.
    pub(super) handle: i32,
    pub(super) press_x: f64,
    pub(super) press_y: f64,
    /// `[scale_x, scale_y, angle, dx, dy]` captured at press.
    pub(super) start: [f64; 5],
    pub(super) dragging: bool,
    /// Gesture mode; `Free` uses the similarity scalars.
    pub(super) mode: TransformMode,
    /// Live target quad for a projective mode; `None` in `Free`.
    pub(super) quad: Option<[(f64, f64); 4]>,
    /// Quad captured at begin/press for a projective gesture.
    pub(super) start_quad: [(f64, f64); 4],
    /// For a selection's pixels lifted onto `path` (a floating layer above the
    /// source): the source path and the document before the lift, which a
    /// cancel restores.
    pub(super) lifted: Option<(String, Box<Document>)>,
}

/// Backing Rust state for [`super::qobject::PictureView`].
pub struct PictureViewRust {
    pub(super) doc: Option<Document>,
    pub(super) selection: Option<Selection>,
    /// CS6 "last deselected" memory: the selection replaced by Deselect or a
    /// New-mode commit. Restored by `reselect`.
    pub(super) deselected_selection: Option<Selection>,
    /// Selection captured at the start of a move-selection drag; `None` when no
    /// drag is active. Restored by `cancel_selection_move`.
    pub(super) selection_move_origin: Option<Selection>,
    pub(super) history: History,
    pub(super) path: Option<String>,
    /// Panel path of the single active layer, or `None` when the selection is
    /// empty or multiple. The shared resolver turns this into the edit target.
    pub(super) active_layer: Option<String>,
    /// The path calls edit the active shape layer's outline, not the Work
    /// Path (`paths.rs`).
    pub(super) path_on_layer: bool,
    /// The Paths panel row the path calls edit when not on a shape layer.
    pub(super) active_path: super::paths::ActivePath,
    pub(super) dirty: bool,
    /// Output format remembered from the import source, used to preselect the
    /// Save As filter: a lowercased extension (`"png"`) or `"psd"` when native.
    pub(super) source_format: String,
    pub(super) interop: Option<crate::gpu::InteropState>,
    pub(super) pending_lasso: Vec<(i32, i32)>,
    pub(super) pending_lasso_mode: String,
    pub(super) stroke: Option<Stroke>,
    pub(super) stroke_label: String,
    /// A live brush stroke painting a layer mask's coverage instead of the
    /// layer's pixels; `Some` only while the mask edit target is active.
    pub(super) mask_stroke: Option<MaskStroke>,
    /// The layer whose raster mask is the edit target, so the brush, fill, and
    /// filters write its coverage; `None` edits the layer's pixels.
    pub(super) mask_edit_target: Option<String>,
    /// Dabs received since the last in-stroke present, waiting for `flush_present`.
    pub(super) pending_present: Option<PsdRect>,
    /// The stroke's dirty area as document-space tiles, rebuilt per dab and
    /// decomposed into disjoint rectangles at commit.
    pub(super) stroke_tiles: TileSet,
    /// True from the present that opens a frame until its flush runs, so the
    /// frame's later dabs accumulate instead of presenting again.
    pub(super) present_flush_due: bool,
    /// The live healing gesture (Spot Healing Brush / Healing Brush): a
    /// coverage mask accumulated over a layer, healed on release.
    pub(super) heal_stroke: Option<HealStroke>,
    pub(super) move_base: Option<QImage>,
    pub(super) move_layer: Option<QImage>,
    pub(super) move_x: i32,
    pub(super) move_y: i32,
    pub(super) move_opacity: i32,
    pub(super) move_prepared_revision: u64,
    /// The panel path of the layer the move-preview base was built from; empty
    /// when no whole-layer preview is prepared.
    pub(super) move_prepared_layer: String,
    pub(super) move_preview_cache_hit: bool,
    pub(super) transform_session: Option<TransformSession>,
    pub(super) opacity_preview_changed: bool,
    pub(super) fill_preview_changed: bool,
    pub(super) content_revision: u64,
    pub(super) gpu_compute: bool,
    pub(super) color_policy: pictura_codec::Policy,
    /// Outstanding display damage; the canvas redraws the region `take` reports.
    /// No longer consumed by present: it keys on [`Self::canvas_revision`].
    pub(super) damage: CanvasDamage,
    /// Non-consuming revision of the displayed canvas pixels, bumped whenever
    /// `image` or the pyramid is rebuilt. Lets the shell cache without draining
    /// the damage account before `image` reads it.
    pub(super) canvas_revision: u64,
    /// Bumped only when the level-0 frame is rebuilt whole; a region change
    /// always reaches the shell as a `region_blitted`. A shell whose copy of
    /// the frame carries this revision needs no full image on `changed`.
    pub(super) frame_revision: u64,
    /// The document before a stroke that had to change its structure to begin
    /// (the Background Eraser layering a Background); cancelling restores it,
    /// since the stroke's saved tiles cannot undo a structural change.
    pub(super) stroke_base: Option<Document>,
    /// The halved-level view of the composite, fed by `recomposite` and
    /// `refresh_region`.
    pub(super) pyramid: ViewPyramid,
    /// The 4-plane straight **sRGB** level-0 frame the display image, pyramid,
    /// and canvas crops all share, so every path shows the working-space
    /// conversion rather than the raw composite. The display `QImage` is built
    /// from this frame on demand (`PictureView::image`) rather than cached, so
    /// only one full-resolution frame is held.
    pub(super) level0: Option<PixelBuffer>,
    pub(super) link_sets: HashMap<String, u32>,
    /// Magnetic Lasso edge field, live for one gesture (`magnetic_begin` to
    /// `magnetic_end`): one `f32` per pixel, too costly to rebuild per move.
    pub(super) edge_map: Option<pictura_select::EdgeMap>,
    /// The Ruler tool's measuring line: view state, never saved or undone.
    pub(super) ruler: Option<pictura_core::Ruler>,
    /// The reduced-resolution stroke presented while a large brush is down;
    /// `None` whenever the dab fits the raster budget.
    pub(super) preview: Option<PreviewStroke>,
    /// The full-resolution GPU stroke presented while the button is down;
    /// `None` when the GPU is off or the paint mode is not modelled. Each dab
    /// patches the exact working document, so the commit needs no replay and the
    /// committed pixels equal the GPU stroke (within ±1 LSB of the CPU oracle).
    pub(super) gpu_stroke: Option<GpuStroke>,
    /// The GPU present's dab placer, so its dabs space exactly as the CPU
    /// stroke's do.
    pub(super) gpu_placer: Option<DabPlacer>,
    /// Pre-filter pixels for an open filter-dialog preview, so each parameter
    /// change re-filters the original rather than compounding on the last
    /// preview. `None` when no dialog is open.
    pub(super) filter_preview: Option<FilterPreview>,
    /// Pre-filter mask coverage for an open filter-dialog preview on a mask,
    /// with the target path, so each parameter change re-filters the original.
    /// `None` when no mask preview is open.
    pub(super) mask_filter_preview: Option<(String, Vec<u8>)>,
    /// The reduced copy a zoomed-out filter dialog previews on, kept between
    /// parameter changes (`filter_tools::filter_canvas_proxy`). `None` when no
    /// dialog holds one.
    pub(super) canvas_proxy: Option<CanvasProxy>,
    /// Filter work running on worker threads (`filter_tools::jobs`).
    pub(super) filter_jobs: super::filter_tools::jobs::FilterJobs,
    /// The document before an open Indexed Color dialog's live preview, so each
    /// option change converts the original. `None` when no preview is shown.
    pub(super) mode_preview: Option<pictura_core::Document>,
    /// The last committed filter's kind and slot values, for Last Filter and
    /// Last Filter Settings.
    pub(super) last_filter: Option<(String, Vec<f64>)>,
    /// Why the most recent filter apply/preview was refused, for surfacing the
    /// failure to the user. Cleared at the start of every attempt.
    pub(super) filter_error: Option<String>,
    /// Layers Panel Options: name a duplicate `"<name> copy"` (CS6 "Add copy to
    /// Copied Layers and Groups", default on).
    pub(super) add_copy: bool,
    /// Layers Panel Options: a fill/adjustment layer created with an active
    /// selection takes that selection as a layer mask (CS6 "Use Default Masks
    /// on Fill Layers", default on).
    pub(super) use_default_masks: bool,
    /// The Copy Shape Attributes clipboard: the last shape layer's fill and
    /// stroke, pasted onto another shape layer. Session state, never saved.
    pub(super) shape_attributes: Option<pictura_render::ShapeAttributes>,
    /// The background crash-recovery snapshot write in flight, if any
    /// (`impl_core/recovery.rs`); its result is whether the file landed.
    pub(super) recovery_write: Option<std::thread::JoinHandle<bool>>,
}

/// The pre-filter pixels backing a live filter preview.
pub(super) struct FilterPreview {
    /// Panel path of the layer being previewed.
    pub(super) layer_path: String,
    /// What the preview overwrote.
    pub(super) baseline: PreviewBaseline,
}

/// What a filter preview overwrote, so the next one starts from the original.
pub(super) enum PreviewBaseline {
    /// The whole layer, before a whole-layer preview (sharing its planes).
    Layer(Box<Layer>),
    /// The pixels of `rect` (inside the layer) in each layer-sized channel,
    /// before a section preview, which writes nothing outside it. A copy of
    /// the section alone, so the live planes are written in place and never
    /// copied whole.
    Section {
        layer_rect: PsdRect,
        rect: PsdRect,
        planes: Vec<(i16, Vec<u8>)>,
    },
}

/// A filter dialog's reduced canvas copy and what it was built from.
pub(super) struct CanvasProxy {
    /// The document region, the scale's bits and the `content_revision`.
    pub(super) key: (PsdRect, u64, u64),
    pub(super) proxy: pictura_render::FilterProxy,
}

/// A large-brush stroke shown at a view-pyramid level instead of rasterized at
/// full resolution.
///
/// `snapshot` is the level's bytes as they were at stroke start — the pre-stroke
/// pixels the blend always starts from — so `coverage` can be applied to the
/// whole extent on every present without compounding. `samples` is the exact
/// input the commit replays through a real [`Stroke`] on the full-resolution
/// document, which is what keeps the committed pixels byte-identical to a
/// stroke that never previewed.
pub(super) struct PreviewStroke {
    pub(super) level: u32,
    pub(super) scale: u32,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) snapshot: Vec<u8>,
    pub(super) coverage: Vec<u8>,
    pub(super) extent: Option<pictura_core::PsdRect>,
    pub(super) samples: Vec<StrokeSample>,
    /// The stroke's configuration, so the preview rasterizes the same tip,
    /// flow and opacity the exact stroke will.
    pub(super) cfg: pictura_paint::StrokeConfig,
}

/// A live brush stroke painting a layer mask's coverage: the mask exposed as a
/// grayscale document (`document`), its document-space rectangle (`rect`), and
/// the exact [`Stroke`] running on it.
pub(super) struct MaskStroke {
    pub(super) path: String,
    pub(super) rect: pictura_core::PsdRect,
    pub(super) document: Document,
    pub(super) stroke: Stroke,
}

impl Default for PictureViewRust {
    fn default() -> Self {
        Self {
            doc: None,
            selection: None,
            deselected_selection: None,
            selection_move_origin: None,
            history: History::default(),
            path: None,
            active_layer: None,
            path_on_layer: false,
            active_path: super::paths::ActivePath::Work,
            dirty: false,
            source_format: "psd".to_string(),
            interop: None,
            pending_lasso: Vec::new(),
            pending_lasso_mode: String::new(),
            stroke: None,
            stroke_label: String::new(),
            mask_stroke: None,
            mask_edit_target: None,
            pending_present: None,
            stroke_tiles: TileSet::default(),
            present_flush_due: false,
            heal_stroke: None,
            move_base: None,
            move_layer: None,
            move_x: 0,
            move_y: 0,
            move_opacity: 0,
            move_prepared_revision: 0,
            move_prepared_layer: String::new(),
            move_preview_cache_hit: false,
            transform_session: None,
            opacity_preview_changed: false,
            fill_preview_changed: false,
            content_revision: 0,
            gpu_compute: true,
            color_policy: pictura_codec::Policy::Preserve,
            damage: CanvasDamage::default(),
            canvas_revision: 0,
            frame_revision: 0,
            stroke_base: None,
            pyramid: ViewPyramid::default(),
            level0: None,
            link_sets: HashMap::new(),
            edge_map: None,
            ruler: None,
            preview: None,
            gpu_stroke: None,
            gpu_placer: None,
            filter_preview: None,
            mask_filter_preview: None,
            canvas_proxy: None,
            filter_jobs: Default::default(),
            mode_preview: None,
            last_filter: None,
            filter_error: None,
            add_copy: true,
            use_default_masks: true,
            shape_attributes: None,
            recovery_write: None,
        }
    }
}
