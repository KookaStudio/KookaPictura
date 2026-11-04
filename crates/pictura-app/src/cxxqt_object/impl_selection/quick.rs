//! Quick Selection: the wand region grows inside one stroke, previews without
//! history, and commits one Selection state.
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::edit::Edit;
use super::super::helpers::combine_mode_from;
use super::super::picture::picture;
use super::super::qobject;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::PsdRect;
use pictura_select::{CombineMode, Selection};

/// Where a selection shape can cover: zero outside `zero_outside`, and when
/// `tight`, exactly the bounds of its covered pixels.
#[derive(Clone, Copy, Debug)]
pub(in crate::cxxqt_object) struct ShapeBox {
    pub(in crate::cxxqt_object) zero_outside: PsdRect,
    pub(in crate::cxxqt_object) tight: bool,
}

pub(super) fn combine_selection(
    rust: &mut super::super::state::PictureViewRust,
    shape: Selection,
    mode: CombineMode,
    keep_reselect: bool,
) {
    combine_selection_boxed(rust, shape, None, mode, keep_reselect);
}

/// [`combine_selection`] for a shape whose [`ShapeBox`] is known: the combine
/// visits only the box, and a result whose bounds follow from it seeds the
/// selection-bounds cache, so the marching ants skip the whole-mask scan.
pub(super) fn combine_selection_boxed(
    rust: &mut super::super::state::PictureViewRust,
    shape: Selection,
    boxed: Option<ShapeBox>,
    mode: CombineMode,
    keep_reselect: bool,
) {
    // `Some(None)`: the shape is known to cover nothing.
    let tight = boxed
        .filter(|b| b.tight)
        .map(|b| (!b.zero_outside.is_empty()).then_some(b.zero_outside));
    // Only a replace and a seeded add read the old bounds: a Quick Selection
    // dab adds to a mask no one has scanned yet.
    let previous_bounds = match mode {
        CombineMode::New => rust.selection_mask_bounds(),
        CombineMode::Add if tight.is_some() => rust.selection_mask_bounds(),
        _ => None,
    };
    let replaced = matches!(mode, CombineMode::New) && previous_bounds.is_some();
    let previous = rust.session.take_selection();
    let seed = if replaced {
        rust.session.deselected_selection = previous;
        rust.session.set_selection(Some(shape));
        tight
    } else {
        if !keep_reselect {
            rust.session.deselected_selection = None;
        }
        let fresh = matches!(mode, CombineMode::New)
            || (previous.is_none() && matches!(mode, CombineMode::Add));
        let (selection, seed) = match previous {
            // A new shape replaces whatever covered nothing, and a shape
            // added to nothing is itself.
            _ if fresh => (shape, tight),
            Some(mut base) => {
                let same = base.width == shape.width && base.height == shape.height;
                let seed = match (mode, tight) {
                    (CombineMode::Add, Some(rect)) => Some(match (rect, previous_bounds) {
                        (Some(rect), Some(prev)) => Some(prev.union(rect)),
                        (rect, prev) => rect.or(prev),
                    }),
                    _ => None,
                };
                match boxed {
                    Some(b) if same => {
                        let _ = base.combine_within(&shape, mode.into(), b.zero_outside);
                    }
                    _ => base.combine_with(&shape, mode),
                }
                (base, seed.filter(|_| same))
            }
            None => (Selection::none(shape.width, shape.height), Some(None)),
        };
        rust.session.set_selection(Some(selection));
        seed
    };
    if let Some(bounds) = seed {
        rust.seed_selection_mask_bounds(bounds);
    }
}

/// [`combine_selection_boxed`] adding `shape`, for the profile.
#[cfg(test)]
pub(in crate::cxxqt_object) fn combine_for_profile(
    rust: &mut super::super::state::PictureViewRust,
    shape: Selection,
    boxed: Option<ShapeBox>,
) {
    combine_selection_boxed(rust, shape, boxed, CombineMode::Add, false);
}

impl qobject::PictureView {
    fn quick_select_shape(&self, x: i32, y: i32, tolerance: i32) -> Option<Selection> {
        let rust = self.rust();
        let doc = rust.session.doc.as_ref()?;
        if x < 0 || y < 0 || x as u32 >= doc.width || y as u32 >= doc.height {
            return None;
        }
        let tolerance = tolerance.clamp(0, 255) as u8;
        pictura_select::magic_wand(&picture(doc, rust), x as u32, y as u32, tolerance, true).ok()
    }

    pub fn quick_select(
        mut self: Pin<&mut Self>,
        x: i32,
        y: i32,
        tolerance: i32,
        mode: &QString,
    ) -> bool {
        let Some(shape) = self.quick_select_shape(x, y, tolerance) else {
            return false;
        };
        let mode = combine_mode_from(&mode.to_string());
        self.as_mut().apply_selection(shape, mode)
    }

    /// Apply one Quick Selection dab without recording history.
    pub fn quick_select_preview(
        mut self: Pin<&mut Self>,
        x: i32,
        y: i32,
        tolerance: i32,
        mode: &QString,
        is_move: bool,
    ) -> bool {
        // A move inside the region the stroke has already selected adds
        // nothing. Skip before this dab composites, floods, combines, and
        // retraces the outline. New, Subtract, Intersect, the first press, and
        // partially covered pixels still run the full path.
        // ponytail: a seed whose local colour differs can no longer contribute
        // a few extra pixels inside an existing selection; the stroke does not
        // require that growth.
        if is_move && combine_mode_from(&mode.to_string()) == CombineMode::Add {
            let rust = self.rust();
            let inside = rust.session.selection.as_ref().is_some_and(|sel| {
                x >= 0
                    && y >= 0
                    && (x as u32) < sel.width
                    && (y as u32) < sel.height
                    && sel.data.get(y as usize * sel.width as usize + x as usize) == Some(&255)
            });
            if inside {
                return true;
            }
        }
        let Some(shape) = self.quick_select_shape(x, y, tolerance) else {
            return false;
        };
        let mode = combine_mode_from(&mode.to_string());
        self.as_mut().edit("Selection", |rust| {
            // The first dab may set the reselect memory; later dabs of the same
            // stroke keep it instead of clearing it.
            let keep = rust
                .session
                .quick_selection_snapshot
                .as_ref()
                .is_some_and(|s| s.reselect.is_some());
            combine_selection(rust, shape, mode, keep);
            if !keep {
                if let Some(snapshot) = rust.session.quick_selection_snapshot.as_mut() {
                    snapshot.reselect = Some(rust.session.deselected_selection.clone());
                }
            }
            Edit::SelectionPreview
        });
        true
    }

    /// Snapshot the selection before a Quick Selection stroke.
    pub fn quick_select_begin(mut self: Pin<&mut Self>) -> bool {
        let mut rust = self.as_mut().rust_mut();
        if rust.session.doc.is_none() {
            return false;
        }
        let snapshot = super::super::document_session::QuickSelectionSnapshot {
            selection: rust.session.selection.clone(),
            deselected: rust.session.deselected_selection.clone(),
            reselect: None,
        };
        rust.session.quick_selection_snapshot = Some(snapshot);
        true
    }

    /// Record the stroke's one history state. False when there is no snapshot
    /// (no stroke, or already committed) or the selection did not change.
    pub fn quick_select_commit(mut self: Pin<&mut Self>) -> bool {
        let changed = {
            let mut rust = self.as_mut().rust_mut();
            if rust.session.doc.is_none() {
                return false;
            }
            let Some(snapshot) = rust.session.quick_selection_snapshot.take() else {
                return false;
            };
            let changed = rust.session.selection != snapshot.selection;
            if !changed {
                rust.session.deselected_selection = snapshot.deselected;
            }
            changed
        };
        if changed {
            self.as_mut().edit("Selection", |_| Edit::Meta);
        }
        changed
    }

    /// Restore the selection captured at the start of a Quick Selection stroke.
    pub fn quick_select_cancel(mut self: Pin<&mut Self>) -> bool {
        let snapshot = self
            .as_mut()
            .rust_mut()
            .session
            .quick_selection_snapshot
            .take();
        let Some(snapshot) = snapshot else {
            return false;
        };
        self.as_mut().edit("Selection", |rust| {
            rust.session.set_selection(snapshot.selection);
            rust.session.deselected_selection = snapshot.deselected;
            Edit::SelectionPreview
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::helpers::feathered_within;
    use super::super::super::state::PictureViewRust;
    use super::*;

    const W: u32 = 120;
    const H: u32 = 90;

    fn lasso(points: &[(i32, i32)], feather: f64) -> (Selection, Option<ShapeBox>) {
        let (shape, bounds) = Selection::polygon_bounded(W, H, points);
        feathered_within(shape, bounds, feather)
    }

    fn commit(rust: &mut PictureViewRust, points: &[(i32, i32)], mode: CombineMode, feather: f64) {
        let (shape, boxed) = lasso(points, feather);
        combine_selection_boxed(rust, shape, boxed, mode, false);
    }

    /// The selection a whole-mask combine of the same lassos builds.
    fn reference(steps: &[(&[(i32, i32)], CombineMode)]) -> Selection {
        let mut out: Option<Selection> = None;
        for &(points, mode) in steps {
            let shape = Selection::polygon(W, H, points);
            out = Some(match (out, mode) {
                (Some(mut base), m) if m != CombineMode::New => {
                    base.combine_with(&shape, m);
                    base
                }
                _ => shape,
            });
        }
        out.expect("at least one step")
    }

    const FIRST: &[(i32, i32)] = &[(10, 10), (70, 15), (40, 60)];
    const SECOND: &[(i32, i32)] = &[(50, 20), (110, 30), (90, 80), (55, 70)];

    #[test]
    fn a_lasso_seeds_its_bounds_and_matches_the_whole_mask_combine() {
        let mut rust = PictureViewRust::default();
        commit(&mut rust, FIRST, CombineMode::New, 0.0);
        let sel = rust.session.selection.clone().expect("a selection");
        assert_eq!(sel, reference(&[(FIRST, CombineMode::New)]));
        assert_eq!(rust.selection_mask_bounds(), sel.bounds());
        assert_eq!(
            rust.selection_bounds_computes.get(),
            0,
            "New seeds the bounds"
        );

        commit(&mut rust, SECOND, CombineMode::Add, 0.0);
        let added = rust.session.selection.clone().expect("a selection");
        assert_eq!(
            added,
            reference(&[(FIRST, CombineMode::New), (SECOND, CombineMode::Add)])
        );
        assert_eq!(rust.selection_mask_bounds(), added.bounds());
        assert_eq!(
            rust.selection_bounds_computes.get(),
            0,
            "Add seeds the union"
        );

        commit(&mut rust, FIRST, CombineMode::Subtract, 0.0);
        let cut = rust.session.selection.clone().expect("a selection");
        let steps = [
            (FIRST, CombineMode::New),
            (SECOND, CombineMode::Add),
            (FIRST, CombineMode::Subtract),
        ];
        assert_eq!(cut, reference(&steps));
        assert_eq!(rust.selection_mask_bounds(), cut.bounds());
        assert_eq!(rust.selection_bounds_computes.get(), 1, "Subtract rescans");

        commit(&mut rust, SECOND, CombineMode::Intersect, 0.0);
        let mut both = cut.clone();
        both.combine_with(&Selection::polygon(W, H, SECOND), CombineMode::Intersect);
        assert_eq!(rust.session.selection.as_ref(), Some(&both));
    }

    #[test]
    fn a_feathered_lasso_equals_the_whole_mask_feather_and_is_rescanned() {
        let mut rust = PictureViewRust::default();
        commit(&mut rust, SECOND, CombineMode::New, 6.0);
        let sel = rust.session.selection.clone().expect("a selection");
        assert_eq!(sel, Selection::polygon(W, H, SECOND).feather(6.0));
        assert_eq!(rust.selection_mask_bounds(), sel.bounds());
        assert_eq!(
            rust.selection_bounds_computes.get(),
            1,
            "a feather is not tight"
        );
    }

    #[test]
    fn a_lasso_outside_the_canvas_selects_nothing() {
        let mut rust = PictureViewRust::default();
        commit(
            &mut rust,
            &[(-40, -40), (-10, -40), (-20, -5)],
            CombineMode::New,
            0.0,
        );
        assert_eq!(rust.selection_mask_bounds(), None);
        assert_eq!(rust.selection_bounds_computes.get(), 0);
    }
}
