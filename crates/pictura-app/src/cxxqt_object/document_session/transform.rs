//! The Free Transform session's own state: the gesture mode, the live
//! similarity transform, and the drag the pointer drives.
//!
//! The session slot owns this value, so the transform has one home.

use pictura_core::PsdRect;

/// The live Free Transform gesture mode. `Free` is the unconstrained
/// similarity transform; `Scale` and `Rotate` keep the same similarity state
/// but restrict which gesture a handle drives; the last three edit a
/// document-space target quad.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(in crate::cxxqt_object) enum TransformMode {
    #[default]
    Free,
    Scale,
    Rotate,
    Skew,
    Distort,
    Perspective,
}

impl TransformMode {
    /// Parse the command mode name, or `None` for an unknown string.
    pub(in crate::cxxqt_object) fn parse(name: &str) -> Option<Self> {
        match name {
            "free" => Some(Self::Free),
            "scale" => Some(Self::Scale),
            "rotate" => Some(Self::Rotate),
            "skew" => Some(Self::Skew),
            "distort" => Some(Self::Distort),
            "perspective" => Some(Self::Perspective),
            _ => None,
        }
    }

    pub(in crate::cxxqt_object) fn name(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Scale => "scale",
            Self::Rotate => "rotate",
            Self::Skew => "skew",
            Self::Distort => "distort",
            Self::Perspective => "perspective",
        }
    }

    pub(in crate::cxxqt_object) fn is_projective(self) -> bool {
        matches!(self, Self::Skew | Self::Distort | Self::Perspective)
    }
}

/// A committed similarity transform that `Edit > Transform > Again` can
/// reapply to another target: the scale/rotation/shear, the translation, and
/// the reference point's offset from the source rect's centre (so a reference
/// point moved off-centre keeps its relative position).
#[derive(Clone, Copy)]
pub(in crate::cxxqt_object) struct RepeatTransform {
    pub(in crate::cxxqt_object) scale_x: f64,
    pub(in crate::cxxqt_object) scale_y: f64,
    pub(in crate::cxxqt_object) angle: f64,
    pub(in crate::cxxqt_object) skew_x: f64,
    pub(in crate::cxxqt_object) skew_y: f64,
    pub(in crate::cxxqt_object) dx: f64,
    pub(in crate::cxxqt_object) dy: f64,
    pub(in crate::cxxqt_object) pivot_offset: (f64, f64),
}

/// What a Warp-mode press grabbed; `start` (the press) and `pts0` (the mesh's
/// control grid at the press) are in source space.
pub(in crate::cxxqt_object) enum WarpDrag {
    /// An anchor or handle: it follows the pointer (an anchor takes its handles
    /// along).
    Point {
        index: usize,
        start: [f64; 2],
        pts0: Vec<[f64; 2]>,
    },
    /// Anywhere else on the grid: the surface under the pointer follows it.
    Surface {
        s: f64,
        t: f64,
        start: [f64; 2],
        pts0: Vec<[f64; 2]>,
    },
}

/// The live Free Transform session: the target path, the source rect, the
/// current similarity transform about its reference point, and the drag state.
pub(in crate::cxxqt_object) struct TransformSession {
    pub(in crate::cxxqt_object) path: String,
    pub(in crate::cxxqt_object) orig_rect: PsdRect,
    pub(in crate::cxxqt_object) scale_x: f64,
    pub(in crate::cxxqt_object) scale_y: f64,
    pub(in crate::cxxqt_object) angle: f64,
    pub(in crate::cxxqt_object) dx: f64,
    pub(in crate::cxxqt_object) dy: f64,
    /// Active handle: 0..=7 scale, 8 rotate, 9 move, 10 pivot, -1 none.
    pub(in crate::cxxqt_object) handle: i32,
    pub(in crate::cxxqt_object) press_x: f64,
    pub(in crate::cxxqt_object) press_y: f64,
    /// `[scale_x, scale_y, angle, dx, dy]` captured at press.
    pub(in crate::cxxqt_object) start: [f64; 5],
    pub(in crate::cxxqt_object) dragging: bool,
    /// Gesture mode; `Free` uses the similarity scalars.
    pub(in crate::cxxqt_object) mode: TransformMode,
    /// Whether the target is an editable type layer: Skew keeps it a
    /// parallelogram so the text stays type.
    pub(in crate::cxxqt_object) type_layer: bool,
    /// Live target quad for a projective mode; `None` in `Free`.
    pub(in crate::cxxqt_object) quad: Option<[(f64, f64); 4]>,
    /// `Free` only: the source-space box a Ctrl+Alt+Shift perspective drag
    /// left, after which the similarity scalars apply.
    pub(in crate::cxxqt_object) base_quad: Option<[(f64, f64); 4]>,
    /// Whether the drag in progress is a `Free` Ctrl+Alt+Shift perspective drag.
    pub(in crate::cxxqt_object) free_perspective: bool,
    /// The perspective drag's readout side: (moved corner, its unmoved
    /// neighbour, whether the moved edge is the top or bottom edge).
    pub(in crate::cxxqt_object) perspective_side: Option<(usize, usize, bool)>,
    /// Quad captured at begin/press for a projective gesture.
    pub(in crate::cxxqt_object) start_quad: [(f64, f64); 4],
    /// The reference point (document space) and its position at begin.
    pub(in crate::cxxqt_object) pivot: (f64, f64),
    pub(in crate::cxxqt_object) pivot_origin: (f64, f64),
    /// The reference point captured at press; a Distort drag carries it with
    /// the quad from this anchor.
    pub(in crate::cxxqt_object) start_pivot: (f64, f64),
    /// Whether the canvas draws the reference point.
    pub(in crate::cxxqt_object) show_pivot: bool,
    /// Horizontal/vertical skew in radians.
    pub(in crate::cxxqt_object) skew_x: f64,
    pub(in crate::cxxqt_object) skew_y: f64,
    /// The session's resample method.
    pub(in crate::cxxqt_object) interpolation: pictura_render::Interpolation,
    /// Warp mode: the warp over `orig_rect`, in source space; the box's quad
    /// stays its projective frame, applied after it on display and commit.
    pub(in crate::cxxqt_object) warp: Option<pictura_core::warp::Warp>,
    /// The Warp-mode drag in progress.
    pub(in crate::cxxqt_object) warp_drag: Option<WarpDrag>,
    /// For a selection's pixels lifted onto `path` (a floating layer above the
    /// source): the source path and the document before the lift, which a
    /// cancel restores.
    pub(in crate::cxxqt_object) lifted: Option<(String, Box<pictura_core::Document>)>,
    /// The layer's rect before `trim_to_content` hugged the box to its pixels;
    /// a session dropped without a transform pads the layer back out to it.
    pub(in crate::cxxqt_object) untrimmed: Option<PsdRect>,
    /// The history label a commit records. `Free Transform` for an interactive
    /// session; an exact command (`Rotate 90° Clockwise`, `Transform Again`, …)
    /// sets its own before committing immediately.
    pub(in crate::cxxqt_object) history_label: String,
}
