//! The Free Transform session's own state: the gesture mode, the live
//! similarity transform, and the drag the pointer drives.
//!
//! The session slot owns this value, so the transform has one home.

use super::super::impl_transform::{rect_center, source_corners};
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

/// What a transform session edits: a layer's pixels or the selection border.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(in crate::cxxqt_object) enum TransformTarget {
    #[default]
    Layer,
    Selection,
}

/// A committed similarity transform that `Edit > Transform > Again` can
/// reapply to another target: the scale/rotation/shear, the translation, and
/// the reference point's offset from the source rect's centre in the item's
/// source frame (so a reference point moved off-centre keeps its relative
/// position).
#[derive(Clone, Copy)]
pub(in crate::cxxqt_object) struct RepeatTransform {
    /// Whether the last transform targeted the selection border.
    pub(in crate::cxxqt_object) target: TransformTarget,
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
    /// Optional state owned by a Content-Aware Scale session.
    pub(in crate::cxxqt_object) content_aware: Option<super::super::impl_transform::content_aware::ContentAwareState>,
    /// This session's target: a layer's pixels or the selection border.
    pub(in crate::cxxqt_object) target: TransformTarget,
    pub(in crate::cxxqt_object) path: String,
    /// Every layer the session transforms with the source rect its map is
    /// centred on, in panel order. `orig_rect` is the union of their rects.
    pub(in crate::cxxqt_object) members: Vec<(String, PsdRect)>,
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
    /// Whether the drag in progress is a `Free` Ctrl distort drag.
    pub(in crate::cxxqt_object) free_distort: bool,
    /// Whether the drag in progress is a `Free` Ctrl+Shift skew drag.
    pub(in crate::cxxqt_object) free_skew: bool,
    /// The perspective drag's readout side: (moved corner, its unmoved
    /// neighbour, whether the moved edge is the top or bottom edge).
    pub(in crate::cxxqt_object) perspective_side: Option<(usize, usize, bool)>,
    /// Quad captured at begin/press for a projective gesture.
    pub(in crate::cxxqt_object) start_quad: [(f64, f64); 4],
    /// The reference point: the item's source-frame point in Free, Scale, and
    /// Rotate (its document position is `pivot + (dx, dy)`), and a document
    /// point in the projective modes. `pivot_origin` is the source-rect centre
    /// at begin.
    pub(in crate::cxxqt_object) pivot: (f64, f64),
    pub(in crate::cxxqt_object) pivot_origin: (f64, f64),
    /// The reference point's document position captured at press; a projective
    /// drag carries it from this anchor.
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
    /// Whether Warp mode owns the overlay, gestures, and options page. The mesh
    /// may stay `Some` after Warp mode is left, so the box can still edit it.
    pub(in crate::cxxqt_object) warp_editing: bool,
    /// A source-space homography applied after the warp. Leaving Warp moves the
    /// box onto the warped preview's bounds; this post-frame keeps the rendered
    /// pixels unchanged while the box and grid follow the new bounds.
    pub(in crate::cxxqt_object) warp_post: Option<[f64; 9]>,
    /// For a selection's pixels lifted onto `path` (a floating layer above the
    /// source): the source path and the document before the lift, which a
    /// cancel restores.
    pub(in crate::cxxqt_object) lifted: Option<(String, Box<pictura_core::Document>)>,
    /// Each member's rect before `trim_to_content` hugged the box to its
    /// pixels; a session dropped without a transform pads every member back
    /// out to it.
    pub(in crate::cxxqt_object) untrimmed: Vec<(String, PsdRect)>,
    /// The history label a commit records. `Free Transform` for an interactive
    /// session; an exact command (`Rotate 90° Clockwise`, `Transform Again`, …)
    /// sets its own before committing immediately.
    pub(in crate::cxxqt_object) history_label: String,
}

impl TransformSession {
    /// The only place a session is built: a fresh, ungestured session over
    /// `orig_rect`, with its reference point at the rect's centre and its quad
    /// set for a projective mode.
    ///
    /// `type_layer` and `history_label` are the caller's, because both need
    /// state this type cannot see: the members resolved against the document,
    /// and whether a Place owns the begin.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::cxxqt_object) fn new(
        target: TransformTarget,
        path: String,
        members: Vec<(String, PsdRect)>,
        orig_rect: PsdRect,
        mode: TransformMode,
        type_layer: bool,
        history_label: &str,
    ) -> Self {
        let corners = source_corners(orig_rect);
        let centre = rect_center(orig_rect);
        Self {
            content_aware: None,
            target,
            path,
            members,
            orig_rect,
            scale_x: 1.0,
            scale_y: 1.0,
            angle: 0.0,
            dx: 0.0,
            dy: 0.0,
            handle: -1,
            press_x: 0.0,
            press_y: 0.0,
            start: [1.0, 1.0, 0.0, 0.0, 0.0],
            dragging: false,
            mode,
            type_layer,
            quad: mode.is_projective().then_some(corners),
            base_quad: None,
            free_perspective: false,
            free_distort: false,
            free_skew: false,
            perspective_side: None,
            start_quad: corners,
            pivot: centre,
            pivot_origin: centre,
            start_pivot: centre,
            show_pivot: true,
            skew_x: 0.0,
            skew_y: 0.0,
            interpolation: pictura_render::Interpolation::Bicubic,
            warp: None,
            warp_drag: None,
            warp_editing: false,
            warp_post: None,
            lifted: None,
            untrimmed: Vec::new(),
            history_label: history_label.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TransformMode, TransformSession, TransformTarget};
    use pictura_core::PsdRect;

    fn rect() -> PsdRect {
        PsdRect {
            left: 0,
            top: 0,
            bottom: 10,
            right: 20,
        }
    }

    #[test]
    fn a_free_session_has_no_quad() {
        let session = TransformSession::new(
            TransformTarget::Layer,
            "0".to_string(),
            vec![("0".to_string(), rect())],
            rect(),
            TransformMode::Free,
            false,
            "Free Transform",
        );
        assert!(session.quad.is_none(), "Free is a similarity gesture");
        assert_eq!(
            session.pivot,
            (10.0, 5.0),
            "the reference point is the centre"
        );
        assert_eq!(session.pivot_origin, (10.0, 5.0));
    }

    #[test]
    fn a_projective_session_starts_on_the_source_corners() {
        let session = TransformSession::new(
            TransformTarget::Layer,
            "0".to_string(),
            vec![("0".to_string(), rect())],
            rect(),
            TransformMode::Distort,
            false,
            "Free Transform",
        );
        assert_eq!(
            session.quad,
            Some([(0.0, 0.0), (20.0, 0.0), (20.0, 10.0), (0.0, 10.0)]),
            "the quad starts as the source rect"
        );
        assert_eq!(session.start_quad, session.quad.expect("a quad"));
    }
}
