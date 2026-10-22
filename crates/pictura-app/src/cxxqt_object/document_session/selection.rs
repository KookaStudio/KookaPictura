//! The selection gestures: a lasso or magnetic selection, and the drag that
//! moves the selection.

use pictura_select::{EdgeMap, Selection};

/// The live lasso gesture: the magnetic lasso's edge field and the points the
/// user has added.
///
/// The magnetic lasso begins the gesture and creates the edge map; a plain
/// lasso attaches its points to the gesture the magnetic one started.
#[derive(Default)]
pub(in crate::cxxqt_object) struct SelectionGesture {
    /// Magnetic Lasso edge field, live for one gesture (`magnetic_begin` to
    /// `magnetic_end`): one `f32` per pixel, too costly to rebuild per move.
    pub(in crate::cxxqt_object) edge_map: Option<EdgeMap>,
    /// The points the lasso has collected.
    pub(in crate::cxxqt_object) points: Vec<(i32, i32)>,
    /// The lasso's mode.
    pub(in crate::cxxqt_object) mode: String,
}

/// The live selection-move drag: the selection captured at the press and the
/// drag's offset from it.
pub(in crate::cxxqt_object) struct SelectionMoveState {
    /// Selection captured at the start of the move-selection drag.
    pub(in crate::cxxqt_object) origin: Option<Selection>,
    /// The drag's offset from the origin, stored instead of a translated plane
    /// so a pointer move costs no plane allocation.
    pub(in crate::cxxqt_object) delta: Option<(i32, i32)>,
}
