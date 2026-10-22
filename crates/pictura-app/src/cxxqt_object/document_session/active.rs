//! The live session slot: one value, one variant per kind of session.
//!
//! The slot holds the document's one live edit session. Its variant names the
//! kind, so the kind is read rather than inferred, and the payloads of two
//! kinds can never exist at once.

use super::previews::FilterPreviewSession;
use super::selection::{SelectionGesture, SelectionMoveState};
use super::stroke::StrokeSession;
use super::transform::TransformSession;
use crate::cxxqt_object::helpers::TileSet;
use pictura_core::path::{Subpath, VectorPath};
use pictura_core::{Document, PsdRect};
use pictura_paint::{HealStroke, Stroke};
use pictura_render::gpu::GpuStroke;
use pictura_select::{EdgeMap, Selection};

/// The kind of live session.
///
/// The slot holds one session at a time. A begin of another kind is refused; a
/// repeat begin of the same kind follows that kind's own rule.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::cxxqt_object) enum SessionKind {
    /// A brush or pencil stroke on a layer.
    Paint,
    /// A stroke on a layer mask's coverage.
    MaskPaint,
    /// A spot-healing or healing-brush gesture.
    Healing,
    /// A selection drag.
    SelectionMove,
    /// A Free Transform gesture.
    Transform,
    /// A Pen or Path Selection gesture on a path.
    Path,
    /// A lasso gesture, magnetic or not.
    Lasso,
    /// A filter dialog's live preview.
    FilterPreview,
    /// An image-mode dialog's live preview.
    ModePreview,
}

/// What a path gesture replaced, so an abort can put it back.
pub(in crate::cxxqt_object) enum PathRestore {
    /// The Work or Saved path the gesture edited.
    WorkOrSaved {
        active: super::super::paths::ActivePath,
        path: VectorPath,
    },
    /// A shape layer's outline, which lives in the layer.
    Layer {
        target: String,
        subpaths: Vec<Subpath>,
    },
}

/// The document's one live edit session.
///
/// `Idle` is no session. Every other variant owns the state of the session it
/// names, so two sessions cannot exist at once: the type system rules it out
/// rather than the call sites keeping their word.
#[derive(Default)]
pub(in crate::cxxqt_object) enum ActiveSession {
    /// No live session.
    #[default]
    Idle,
    /// A brush or pencil stroke, on a layer's pixels or on a mask's coverage.
    Stroke(Box<StrokeSession>),
    /// A spot-healing or healing-brush gesture.
    Heal(Box<HealStroke>),
    /// A lasso or magnetic selection gesture.
    Selection(Box<SelectionGesture>),
    /// A selection drag.
    SelectionMove(Box<SelectionMoveState>),
    /// A Free Transform gesture.
    Transform(Box<TransformSession>),
    /// A Pen or Path Selection gesture on a path.
    Path(Box<PathRestore>),
    /// A filter dialog's live preview.
    FilterPreview(Box<FilterPreviewSession>),
    /// An image-mode dialog's live preview.
    ModePreview(Box<Document>),
}

/// Declares the accessors for one `Option` payload: a reader, a mutable
/// reader, a taker, a setter, and a clearer. Every one of them is inert
/// unless the slot holds the variant the payload lives in. Each accessor is
/// requested by name, so the slot carries no accessor nothing calls.
macro_rules! option_payload {
    ($variant:ident, $field:ident, $ty:ty, $($rest:tt)*) => {
        option_payload!(@impl $variant, $field, $ty, $($rest)*);
    };
    (@impl $variant:ident, $field:ident, $ty:ty,) => {};
    (@impl $variant:ident, $field:ident, $ty:ty, read $read:ident, $($rest:tt)*) => {
        pub(in crate::cxxqt_object) fn $read(&self) -> Option<&$ty> {
            match self {
                ActiveSession::$variant(payload) => payload.$field.as_ref(),
                _ => None,
            }
        }
        option_payload!(@impl $variant, $field, $ty, $($rest)*);
    };
    (@impl $variant:ident, $field:ident, $ty:ty, read_mut $read_mut:ident, $($rest:tt)*) => {
        pub(in crate::cxxqt_object) fn $read_mut(&mut self) -> Option<&mut $ty> {
            match self {
                ActiveSession::$variant(payload) => payload.$field.as_mut(),
                _ => None,
            }
        }
        option_payload!(@impl $variant, $field, $ty, $($rest)*);
    };
    (@impl $variant:ident, $field:ident, $ty:ty, take $take:ident, $($rest:tt)*) => {
        pub(in crate::cxxqt_object) fn $take(&mut self) -> Option<$ty> {
            match self {
                ActiveSession::$variant(payload) => payload.$field.take(),
                _ => None,
            }
        }
        option_payload!(@impl $variant, $field, $ty, $($rest)*);
    };
    (@impl $variant:ident, $field:ident, $ty:ty, set $set:ident, $($rest:tt)*) => {
        pub(in crate::cxxqt_object) fn $set(&mut self, value: $ty) {
            match self {
                ActiveSession::$variant(payload) => payload.$field = Some(value),
                _ => {}
            }
        }
        option_payload!(@impl $variant, $field, $ty, $($rest)*);
    };
    (@impl $variant:ident, $field:ident, $ty:ty, clear $clear:ident, $($rest:tt)*) => {
        pub(in crate::cxxqt_object) fn $clear(&mut self) {
            match self {
                ActiveSession::$variant(payload) => payload.$field = None,
                _ => {}
            }
        }
        option_payload!(@impl $variant, $field, $ty, $($rest)*);
    };
}

/// Declares the accessors for a variant that owns its payload whole.
macro_rules! direct_payload {
    ($variant:ident, $ty:ty, $($rest:tt)*) => {
        direct_payload!(@impl $variant, $ty, $($rest)*);
    };
    (@impl $variant:ident, $ty:ty,) => {};
    (@impl $variant:ident, $ty:ty, read $read:ident, $($rest:tt)*) => {
        pub(in crate::cxxqt_object) fn $read(&self) -> Option<&$ty> {
            match self {
                ActiveSession::$variant(payload) => Some(&**payload),
                _ => None,
            }
        }
        direct_payload!(@impl $variant, $ty, $($rest)*);
    };
    (@impl $variant:ident, $ty:ty, read_mut $read_mut:ident, $($rest:tt)*) => {
        pub(in crate::cxxqt_object) fn $read_mut(&mut self) -> Option<&mut $ty> {
            match self {
                ActiveSession::$variant(payload) => Some(&mut **payload),
                _ => None,
            }
        }
        direct_payload!(@impl $variant, $ty, $($rest)*);
    };
    (@impl $variant:ident, $ty:ty, take $take:ident, $($rest:tt)*) => {
        pub(in crate::cxxqt_object) fn $take(&mut self) -> Option<$ty> {
            match std::mem::replace(self, ActiveSession::Idle) {
                ActiveSession::$variant(payload) => Some(*payload),
                other => {
                    *self = other;
                    None
                }
            }
        }
        direct_payload!(@impl $variant, $ty, $($rest)*);
    };
    (@impl $variant:ident, $ty:ty, set $set:ident, $($rest:tt)*) => {
        pub(in crate::cxxqt_object) fn $set(&mut self, value: $ty) {
            *self = ActiveSession::$variant(Box::new(value));
        }
        direct_payload!(@impl $variant, $ty, $($rest)*);
    };
    (@impl $variant:ident, $ty:ty, clear $clear:ident, $($rest:tt)*) => {
        pub(in crate::cxxqt_object) fn $clear(&mut self) {
            if matches!(self, ActiveSession::$variant(_)) {
                *self = ActiveSession::Idle;
            }
        }
        direct_payload!(@impl $variant, $ty, $($rest)*);
    };
}

impl ActiveSession {
    /// The kind of the live session, or `None` when the slot is idle.
    ///
    /// The kind comes from the variant, never from a scan of independent
    /// fields: the stroke variant reports the mask stroke or the layer stroke,
    /// whichever it carries.
    pub(in crate::cxxqt_object) fn kind(&self) -> Option<SessionKind> {
        match self {
            ActiveSession::Idle => None,
            ActiveSession::Stroke(session) => {
                if session.mask_stroke.is_some() {
                    Some(SessionKind::MaskPaint)
                } else {
                    Some(SessionKind::Paint)
                }
            }
            ActiveSession::Heal(_) => Some(SessionKind::Healing),
            ActiveSession::Selection(_) => Some(SessionKind::Lasso),
            ActiveSession::SelectionMove(_) => Some(SessionKind::SelectionMove),
            ActiveSession::Transform(_) => Some(SessionKind::Transform),
            ActiveSession::Path(_) => Some(SessionKind::Path),
            ActiveSession::FilterPreview(_) => Some(SessionKind::FilterPreview),
            ActiveSession::ModePreview(_) => Some(SessionKind::ModePreview),
        }
    }

    /// Whether a begin of `kind` may take the slot.
    ///
    /// The slot holds one session. A begin of another kind is refused. A repeat
    /// begin of the same kind follows that kind's rule: a transform replaces its
    /// own session, a filter or mode preview updates it, and a lasso attaches
    /// to the magnetic gesture it started. A healing, selection-move, path, or
    /// paint begin is refused.
    pub(in crate::cxxqt_object) fn allows(&self, kind: SessionKind) -> bool {
        match self.kind() {
            None => true,
            Some(current) if current == kind => matches!(
                kind,
                SessionKind::Transform
                    | SessionKind::Lasso
                    | SessionKind::FilterPreview
                    | SessionKind::ModePreview
            ),
            Some(_) => false,
        }
    }

    // --- the paint session -------------------------------------------------

    /// The live paint session, when a stroke is live.
    pub(in crate::cxxqt_object) fn stroke_session(&self) -> Option<&StrokeSession> {
        match self {
            ActiveSession::Stroke(session) => Some(session),
            _ => None,
        }
    }

    /// The live paint session, mutably, when a stroke is live.
    pub(in crate::cxxqt_object) fn stroke_session_mut(&mut self) -> Option<&mut StrokeSession> {
        match self {
            ActiveSession::Stroke(session) => Some(session),
            _ => None,
        }
    }

    /// Take the live paint session out of the slot.
    pub(in crate::cxxqt_object) fn take_stroke_session(&mut self) -> Option<StrokeSession> {
        match std::mem::replace(self, ActiveSession::Idle) {
            ActiveSession::Stroke(session) => Some(*session),
            other => {
                *self = other;
                None
            }
        }
    }

    /// Put `session` in the slot as the live paint session.
    pub(in crate::cxxqt_object) fn set_stroke_session(&mut self, session: StrokeSession) {
        *self = ActiveSession::Stroke(Box::new(session));
    }

    option_payload!(
        Stroke,
        stroke,
        Stroke,
        read stroke,
        read_mut stroke_mut,
    );
    option_payload!(
        Stroke,
        mask_stroke,
        super::stroke::MaskStroke,
        read mask_stroke,
        read_mut mask_stroke_mut,
    );
    option_payload!(
        Stroke,
        gpu_stroke,
        GpuStroke,
        read gpu_stroke,
        read_mut gpu_stroke_mut,
        set set_gpu_stroke,
        clear clear_gpu_stroke,
    );
    option_payload!(
        Stroke,
        gpu_placer,
        pictura_paint::spacing::DabPlacer,
        read_mut gpu_placer_mut,
        set set_gpu_placer,
        clear clear_gpu_placer,
    );
    option_payload!(
        Stroke,
        preview,
        super::stroke::PreviewStroke,
        read preview,
        read_mut preview_mut,
        set set_preview,
        clear clear_preview,
    );
    option_payload!(
        Stroke,
        stroke_base,
        Document,
        set set_stroke_base,
        clear clear_stroke_base,
    );

    /// Mark a rectangle as painted by the live stroke.
    pub(in crate::cxxqt_object) fn mark_stroke_tile(&mut self, rect: PsdRect) {
        if let Some(session) = self.stroke_session_mut() {
            session.stroke_tiles.mark(rect);
        }
    }

    /// Reset the live stroke's dirty tiles to a new canvas.
    pub(in crate::cxxqt_object) fn reset_stroke_tiles(&mut self, width: u32, height: u32) {
        if let Some(session) = self.stroke_session_mut() {
            session.stroke_tiles.reset(width, height);
        }
    }

    /// Take the live stroke's dirty tiles out of the slot.
    #[cfg(test)]
    pub(in crate::cxxqt_object) fn take_stroke_tiles(&mut self) -> Option<TileSet> {
        self.stroke_session_mut()
            .map(|session| std::mem::take(&mut session.stroke_tiles))
    }

    /// Drop the live stroke's dirty tiles.
    pub(in crate::cxxqt_object) fn clear_stroke_tiles(&mut self) {
        if let Some(session) = self.stroke_session_mut() {
            session.stroke_tiles = TileSet::default();
        }
    }

    /// Take the live stroke's pending present region.
    pub(in crate::cxxqt_object) fn take_pending_present(&mut self) -> Option<PsdRect> {
        self.stroke_session_mut()
            .and_then(|session| session.pending_present.take())
    }

    /// Whether the live stroke's present frame is open.
    pub(in crate::cxxqt_object) fn present_flush_due(&self) -> bool {
        self.stroke_session()
            .is_some_and(|session| session.present_flush_due)
    }

    /// Open or close the live stroke's present frame.
    pub(in crate::cxxqt_object) fn set_present_flush_due(&mut self, due: bool) {
        if let Some(session) = self.stroke_session_mut() {
            session.present_flush_due = due;
        }
    }

    /// Set the live stroke's pending present region.
    pub(in crate::cxxqt_object) fn set_pending_present(&mut self, rect: Option<PsdRect>) {
        if let Some(session) = self.stroke_session_mut() {
            session.pending_present = rect;
        }
    }

    /// The live path gesture, when one is live.
    #[cfg(test)]
    pub(in crate::cxxqt_object) fn path(&self) -> Option<&PathRestore> {
        match self {
            ActiveSession::Path(restore) => Some(restore),
            _ => None,
        }
    }

    // --- the other sessions ------------------------------------------------

    direct_payload!(
        Heal,
        HealStroke,
        read heal_stroke,
        read_mut heal_stroke_mut,
        take take_heal_stroke,
        set set_heal_stroke,
        clear clear_heal_stroke,
    );

    option_payload!(
        Selection,
        edge_map,
        EdgeMap,
        read edge_map,
    );

    /// End the live magnetic gesture: its edge field is the gesture's state.
    pub(in crate::cxxqt_object) fn clear_edge_map(&mut self) {
        if matches!(self, ActiveSession::Selection(_)) {
            *self = ActiveSession::Idle;
        }
    }

    /// The live lasso's points.
    pub(in crate::cxxqt_object) fn lasso_points(&self) -> Option<&[(i32, i32)]> {
        self.selection_gesture()
            .map(|gesture| gesture.points.as_slice())
    }

    /// The live lasso's points, mutably.
    pub(in crate::cxxqt_object) fn lasso_points_mut(&mut self) -> Option<&mut Vec<(i32, i32)>> {
        self.selection_gesture_mut()
            .map(|gesture| &mut gesture.points)
    }

    /// The live lasso's mode.
    pub(in crate::cxxqt_object) fn lasso_mode(&self) -> Option<&str> {
        self.selection_gesture()
            .map(|gesture| gesture.mode.as_str())
    }

    /// The live lasso gesture.
    pub(in crate::cxxqt_object) fn selection_gesture(&self) -> Option<&SelectionGesture> {
        match self {
            ActiveSession::Selection(gesture) => Some(gesture),
            _ => None,
        }
    }

    /// The live lasso gesture, mutably.
    pub(in crate::cxxqt_object) fn selection_gesture_mut(
        &mut self,
    ) -> Option<&mut SelectionGesture> {
        match self {
            ActiveSession::Selection(gesture) => Some(gesture),
            _ => None,
        }
    }

    /// Put a selection-move drag in the slot.
    pub(in crate::cxxqt_object) fn set_selection_move(&mut self, state: SelectionMoveState) {
        *self = ActiveSession::SelectionMove(Box::new(state));
    }

    /// Take the live selection-move drag out of the slot.
    pub(in crate::cxxqt_object) fn take_selection_move(&mut self) -> Option<SelectionMoveState> {
        match std::mem::replace(self, ActiveSession::Idle) {
            ActiveSession::SelectionMove(state) => Some(*state),
            other => {
                *self = other;
                None
            }
        }
    }

    /// Put a lasso gesture in the slot.
    pub(in crate::cxxqt_object) fn set_selection_gesture(&mut self, gesture: SelectionGesture) {
        *self = ActiveSession::Selection(Box::new(gesture));
    }

    /// Take the live lasso gesture out of the slot.
    pub(in crate::cxxqt_object) fn take_selection_gesture(&mut self) -> Option<SelectionGesture> {
        match std::mem::replace(self, ActiveSession::Idle) {
            ActiveSession::Selection(gesture) => Some(*gesture),
            other => {
                *self = other;
                None
            }
        }
    }

    option_payload!(
        SelectionMove,
        origin,
        Selection,
        read selection_move_origin,
    );
    option_payload!(
        SelectionMove,
        delta,
        (i32, i32),
        read selection_move_delta,
        set set_selection_move_delta,
    );

    direct_payload!(
        Transform,
        TransformSession,
        read transform,
        read_mut transform_mut,
        take take_transform,
        set set_transform,
    );

    direct_payload!(
        Path,
        PathRestore,
        take take_path,
        set set_path,
        clear clear_path,
    );

    /// The live filter preview on a layer's pixels.
    pub(in crate::cxxqt_object) fn filter_preview(
        &self,
    ) -> Option<&super::previews::FilterPreview> {
        match self {
            ActiveSession::FilterPreview(preview) => match &**preview {
                FilterPreviewSession::Layer(layer) => Some(layer),
                FilterPreviewSession::Mask { .. } => None,
            },
            _ => None,
        }
    }

    /// The live filter preview on a mask's coverage: the mask's panel path
    /// and its pre-preview coverage.
    pub(in crate::cxxqt_object) fn mask_filter_preview(&self) -> Option<(&String, &Vec<u8>)> {
        match self {
            ActiveSession::FilterPreview(preview) => match &**preview {
                FilterPreviewSession::Mask { path, original } => Some((path, original)),
                FilterPreviewSession::Layer(_) => None,
            },
            _ => None,
        }
    }

    /// Put a layer preview in the slot, replacing any live preview.
    pub(in crate::cxxqt_object) fn set_filter_preview(
        &mut self,
        preview: super::previews::FilterPreview,
    ) {
        *self = ActiveSession::FilterPreview(Box::new(FilterPreviewSession::Layer(preview)));
    }

    /// Put a mask preview in the slot, replacing any live preview.
    pub(in crate::cxxqt_object) fn set_mask_filter_preview(
        &mut self,
        path: String,
        original: Vec<u8>,
    ) {
        *self =
            ActiveSession::FilterPreview(Box::new(FilterPreviewSession::Mask { path, original }));
    }

    /// Take the live layer preview out of the slot. A mask preview stays.
    pub(in crate::cxxqt_object) fn take_filter_preview(
        &mut self,
    ) -> Option<super::previews::FilterPreview> {
        if !matches!(
            self,
            ActiveSession::FilterPreview(preview)
                if matches!(&**preview, FilterPreviewSession::Layer(_))
        ) {
            return None;
        }
        match std::mem::replace(self, ActiveSession::Idle) {
            ActiveSession::FilterPreview(preview) => match *preview {
                FilterPreviewSession::Layer(layer) => Some(layer),
                FilterPreviewSession::Mask { .. } => None,
            },
            other => {
                *self = other;
                None
            }
        }
    }

    /// Drop the live mask preview.
    pub(in crate::cxxqt_object) fn clear_mask_filter_preview(&mut self) {
        if matches!(
            self,
            ActiveSession::FilterPreview(preview)
                if matches!(&**preview, FilterPreviewSession::Mask { .. })
        ) {
            *self = ActiveSession::Idle;
        }
    }

    /// Drop the live layer preview.
    pub(in crate::cxxqt_object) fn clear_filter_preview(&mut self) {
        if matches!(
            self,
            ActiveSession::FilterPreview(preview)
                if matches!(&**preview, FilterPreviewSession::Layer(_))
        ) {
            *self = ActiveSession::Idle;
        }
    }

    /// Take the live filter preview out of the slot, whichever kind it is.
    pub(in crate::cxxqt_object) fn take_filter_preview_session(
        &mut self,
    ) -> Option<FilterPreviewSession> {
        match std::mem::replace(self, ActiveSession::Idle) {
            ActiveSession::FilterPreview(preview) => Some(*preview),
            other => {
                *self = other;
                None
            }
        }
    }

    direct_payload!(
        ModePreview,
        Document,
        read mode_preview,
        take take_mode_preview,
        set set_mode_preview,
    );
}
