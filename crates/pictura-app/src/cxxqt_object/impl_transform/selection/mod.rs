//! Transform Selection: the shared transform box driving the selection border.
//!
//! The session is the same [`TransformSession`] a layer transform uses, with
//! `target = Selection`; its box is the selection's coverage bounds and it has
//! no moving image. A commit resamples the coverage mask through the live map
//! (or the warp plus the box frame) and records one `"Transform Selection"`
//! state. See `docs/08-selection/transform-selection.md`.

#[cfg(test)]
mod tests;

use super::super::document_session::{SessionKind, TransformSession, TransformTarget};
use super::super::edit::Edit;
use super::super::helpers::memory_gate;
use super::super::qobject;
use super::super::state::PictureViewRust;
use super::geometry::{session_quad, source_corners};
use super::group::repeatable_transform;
use super::warp::{map_source_point, warp_preview_quad};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::PsdRect;

/// The integer box around a session quad, or `None` for a non-finite corner.
fn quad_rect(quad: &[(f64, f64); 4]) -> Option<PsdRect> {
    let (mut left, mut top, mut right, mut bottom) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for &(x, y) in quad {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
    }
    Some(PsdRect {
        left: left.floor() as i32,
        top: top.floor() as i32,
        right: right.ceil() as i32,
        bottom: bottom.ceil() as i32,
    })
}

fn area(rect: PsdRect) -> u64 {
    rect.width().max(0) as u64 * rect.height().max(0) as u64
}

impl PictureViewRust {
    /// Begin a border transform on the live selection, or report an existing
    /// border session as already begun. A layer session is refused: it is modal
    /// and the command that starts it owns the slot.
    pub(in crate::cxxqt_object) fn begin_selection_transform(&mut self) -> bool {
        if !self.quick_mask_content_allowed() {
            return false;
        }
        if !self.slot_allows(SessionKind::Transform) {
            return false;
        }
        if let Some(session) = self.session.live.transform() {
            return session.target == TransformTarget::Selection;
        }
        let Some(doc) = self.session.doc.as_ref() else {
            return false;
        };
        let Some(selection) = self.session.selection.as_ref() else {
            return false;
        };
        if selection.width != doc.width || selection.height != doc.height {
            return false;
        }
        let Some(orig_rect) = self.selection_mask_bounds() else {
            return false;
        };
        if orig_rect.width() <= 0 || orig_rect.height() <= 0 {
            return false;
        }
        self.transform_controls = false;
        self.session.live.set_transform(TransformSession::new(
            TransformTarget::Selection,
            String::new(),
            Vec::new(),
            orig_rect,
            Default::default(),
            false,
            "Transform Selection",
        ));
        true
    }

    /// Commit the live border transform into the selection. Returns whether the
    /// mask changed; the caller records the history state. The session always
    /// clears.
    pub(in crate::cxxqt_object) fn commit_selection_transform(&mut self) -> bool {
        let Some(session) = self.session.live.take_transform() else {
            return false;
        };
        if session.target != TransformTarget::Selection {
            return false;
        }
        let quad = session_quad(&session);
        let unmoved = quad
            .iter()
            .zip(source_corners(session.orig_rect))
            .all(|(q, source)| (q.0 - source.0).abs() <= 1e-9 && (q.1 - source.1).abs() <= 1e-9);
        if session.warp.is_none() && unmoved {
            return false;
        }
        if session
            .warp
            .as_ref()
            .is_some_and(|warp| warp.is_identity() && unmoved)
        {
            return false;
        }
        let Some(selection) = self.session.selection.clone() else {
            return false;
        };
        let dest = if let Some(warp) = session.warp.as_ref() {
            pictura_render::warp_dest_rect(session.orig_rect, warp, warp_preview_quad(&session))
        } else {
            quad_rect(&quad)
        };
        let cost = area(dest.unwrap_or(session.orig_rect))
            .saturating_mul(8)
            .saturating_add(area(session.orig_rect).saturating_mul(8));
        if !memory_gate::admit_resident(cost).admitted() {
            return false;
        }
        let result = match session.warp.as_ref() {
            Some(warp) => pictura_render::warp_selection(
                &selection,
                session.orig_rect,
                warp,
                warp_preview_quad(&session),
                session.interpolation,
            ),
            None => pictura_render::transform_selection(
                &selection,
                session.orig_rect,
                quad,
                session.interpolation,
            ),
        };
        let Ok(Some(updated)) = result else {
            return false;
        };
        self.session.set_selection_in_place(Some(updated));
        self.session.last_transform = repeatable_transform(&session);
        true
    }

    /// Whether the live transform targets the selection border.
    pub(in crate::cxxqt_object) fn selection_transform_live(&self) -> bool {
        self.session
            .live
            .transform()
            .is_some_and(|session| session.target == TransformTarget::Selection)
    }

    /// The live selection-border outline, mapped through the session's warp and
    /// box frame, encoded as `[count, x, y, …]` per loop. Empty without a
    /// border session or a committed outline.
    pub(in crate::cxxqt_object) fn selection_transform_contour(&self) -> Vec<f64> {
        let Some(session) = self.session.live.transform() else {
            return Vec::new();
        };
        if session.target != TransformTarget::Selection {
            return Vec::new();
        }
        let loops = self.selection_contour_extents();
        if loops.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(loops.len() + loops.len() / 2);
        let mut i = 0;
        while i < loops.len() {
            let count = loops[i].max(0) as usize;
            i += 1;
            out.push(count as f64);
            for _ in 0..count {
                if i + 1 >= loops.len() {
                    break;
                }
                let x = f64::from(loops[i]);
                let y = f64::from(loops[i + 1]);
                i += 2;
                match map_source_point(session, x, y) {
                    Some(point) => {
                        out.push(point[0]);
                        out.push(point[1]);
                    }
                    None => {
                        out.push(f64::NAN);
                        out.push(f64::NAN);
                    }
                }
            }
        }
        out
    }
}

impl qobject::PictureView {
    /// Commit a border session through the edit transaction: one
    /// `"Transform Selection"` state when the mask changed, none for an
    /// identity or a refused map.
    pub(super) fn commit_selection_transform(mut self: Pin<&mut Self>) -> bool {
        let label = self
            .rust()
            .session
            .live
            .transform()
            .map(|session| session.history_label.clone())
            .unwrap_or_default();
        let changed = self.as_mut().rust_mut().commit_selection_transform();
        self.as_mut().clear_transform_preview();
        if changed {
            let label = if label.is_empty() {
                "Transform Selection"
            } else {
                label.as_str()
            };
            let _ = self.as_mut().edit(label, |_| Edit::Meta);
        }
        changed
    }
}
