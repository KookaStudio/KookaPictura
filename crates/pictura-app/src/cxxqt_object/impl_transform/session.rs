use super::super::document_session::{TransformMode, TransformSession, TransformTarget};
use super::super::helpers::*;
use super::super::helpers_composite::*;
use super::super::qobject;
use super::super::state::PictureViewRust;
use super::geometry::{
    apply_session_transform, identity_transform, quad_driven, session_quad, MapParams,
};
use super::group::{
    can_free_transform, commit_decision, group_apply, has_vector_placement, lifted_apply,
    repeatable_transform, transform_group, transform_group_edit, transform_rect, CommitDecision,
};
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::helpers::memory_gate;
use crate::cxxqt_object::pdf_export::import::ffi::VectorPlaceResult;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QString};
use pictura_core::{BlendMode, LayerStack, PsdRect};

/// A transform whose layer rect reaches this area runs its engine on a worker.
const DEFERRED_TRANSFORM_PIXELS: u64 = 4_000_000;

mod move_preview;
mod preview;
mod readouts;
mod selection;
mod view_preview;

use preview::{build_transform_preview, transform_preview_above, transform_preview_bytes};
pub(in crate::cxxqt_object) use preview::{
    rendered_transform_group, set_transform_preview_pixels_for_test, transform_preview_layer_at,
};
use selection::{lift_coverage, lifts_selection};
pub(in crate::cxxqt_object) use view_preview::{MoveView, MoveViewPixels};

/// Whether `mode` may begin on `path`: the Free Transform target rules, plus
/// Distort's and Perspective's refusal for editable type (a lifted selection targets pixels, not
/// the type layer).
pub(super) fn can_begin_mode(rust: &PictureViewRust, path: &str, mode: TransformMode) -> bool {
    if !rust.quick_mask_content_allowed() {
        return false;
    }
    let lift = lifts_selection(rust, path);
    let Some(doc) = rust.session.doc.as_ref() else {
        return false;
    };
    if !lift && !can_free_transform(doc, path) {
        return false;
    }
    let type_member = transform_group(rust, doc, path).iter().any(|member| {
        pictura_render::resolve_path(doc, member).is_some_and(|layer| layer.is_type())
    });
    !(matches!(mode, TransformMode::Distort | TransformMode::Perspective) && !lift && type_member)
}

impl qobject::PictureView {
    /// Whether `path` resolves to a transformable layer, or to a layer whose
    /// selected pixels can be lifted and transformed. Read-only.
    pub fn layer_can_free_transform(&self, path: &QString) -> bool {
        let path = path.to_string();
        let rust = self.rust();
        lifts_selection(rust, &path)
            || rust
                .session
                .doc
                .as_ref()
                .is_some_and(|doc| can_free_transform(doc, &path))
    }

    /// Begin a Free Transform session on the layer at `path`.
    ///
    /// Returns false without changing any session for an untransformable target.
    /// A second begin on the same active path is a no-op; a begin on a different
    /// path first cancels the active session.
    pub fn begin_free_transform(self: Pin<&mut Self>, path: &QString) -> bool {
        self.begin_transform_session(path, TransformMode::Free)
    }

    /// Begin a Skew / Distort / Perspective session on the layer at `path`.
    /// `mode` is `"skew"`, `"distort"`, or `"perspective"`; false for anything
    /// else or an untransformable target, with the same resolution and refusal
    /// rules as [`Self::begin_free_transform`].
    pub fn begin_transform_mode(self: Pin<&mut Self>, path: &QString, mode: &QString) -> bool {
        let Some(mode) = TransformMode::parse(&mode.to_string()) else {
            return false;
        };
        self.begin_transform_session(path, mode)
    }

    fn begin_transform_session(
        mut self: Pin<&mut Self>,
        path: &QString,
        mode: TransformMode,
    ) -> bool {
        memory_gate::clear();
        self.as_mut().settle_pending_composite();
        if !self
            .rust()
            .slot_allows(super::super::document_session::SessionKind::Transform)
        {
            return false;
        }
        let mut path = path.to_string();
        if path.is_empty() {
            let Some(active) = self.rust().layer_target.single().map(str::to_string) else {
                return false;
            };
            path = active;
        }
        {
            let rust = self.rust();
            if let Some(session) = rust.session.live.transform() {
                // A lifted session's target is its source; the floating layer's
                // path is an implementation detail that may name a sibling.
                let target = session.lifted.as_ref().map_or(&session.path, |(s, _)| s);
                if *target == path && session.mode == mode {
                    return true;
                }
            }
        }
        // The outgoing session's floating layer goes back into the document,
        // so the display has to follow the swap.
        let was_lifted = self
            .rust()
            .session
            .live
            .transform()
            .is_some_and(|session| session.lifted.is_some());
        let Some(session) = self.as_mut().rust_mut().prepare_transform(&path, mode) else {
            return false;
        };
        let bounded = session.lifted.is_none();
        let members: Vec<String> = session
            .members
            .iter()
            .map(|(member, _)| member.clone())
            .collect();
        if !self.as_mut().compute_transform_preview(
            &session.path,
            &members,
            session.orig_rect,
            bounded,
        ) {
            self.as_mut().rust_mut().restore_transform_begin(session);
            return false;
        }
        {
            let mut rust = self.as_mut().rust_mut();
            rust.session.live.set_transform(session);
            rust.transform_controls = false;
        }
        if was_lifted {
            self.as_mut().recomposite();
        }
        true
    }

    /// Compute and store the transform overlay base/layer for `path`; keyed with
    /// the `-1` sentinel so the Move cache rebuilds when it next runs.
    pub(super) fn compute_transform_preview(
        mut self: Pin<&mut Self>,
        path: &str,
        members: &[String],
        orig_rect: PsdRect,
        bounded: bool,
    ) -> bool {
        let mut guard = self.as_mut().rust_mut();
        let rust = &mut *guard;
        // The preview is built before the session opens; one that does not
        // fit is refused rather than left to the OOM killer (#1297).
        let fits = rust.session.doc.as_ref().is_some_and(|doc| {
            memory_gate::admit_resident(transform_preview_bytes(
                doc, path, members, orig_rect, bounded,
            ))
            .admitted()
        });
        if !fits {
            return false;
        }
        let prepared = rust.session.doc.as_ref().and_then(|doc| {
            build_transform_preview(doc, path, members, orig_rect, rust.gpu_compute, bounded)
        });
        let Some(preview) = prepared else {
            return false;
        };
        let (above_image, above_origin, above_scale) =
            rust.session.doc.as_ref().map_or((None, None, 1), |doc| {
                transform_preview_above(
                    doc,
                    path,
                    members,
                    preview.level,
                    rust.presented_level,
                    rust.presented_rect,
                    rust.gpu_compute,
                )
            });
        rust.move_base = Some(preview.base);
        rust.move_base_origin = preview.base_origin;
        rust.move_view = None;
        rust.move_view_pixels = Default::default();
        rust.move_layer = Some(preview.image);
        rust.transform_preview_level = preview.level;
        rust.move_x = preview.x;
        rust.move_y = preview.y;
        rust.move_above = above_image;
        rust.move_above_origin = above_origin;
        rust.move_above_scale = above_scale;
        rust.move_above_presented = None;
        rust.move_opacity = preview.opacity;
        rust.move_prepared_revision = rust.session.content_revision;
        rust.move_prepared_layer = String::new();
        rust.move_prepared_union = None;
        true
    }

    /// Clear the session; a lifted selection's pixels go back where they were.
    pub fn cancel_transform(mut self: Pin<&mut Self>) {
        if self.rust().pending_vector_place.is_some() {
            drop(self.as_mut().rust_mut().deferred_transform.take());
        }
        self.as_mut().finish_transform(VectorPlaceResult::Cancelled);
    }

    /// Clear the session and report `result` to a pending vector place.
    fn finish_transform(mut self: Pin<&mut Self>, result: VectorPlaceResult) -> bool {
        self.as_mut().drop_transform_session();
        self.as_mut().clear_transform_preview();
        self.as_mut().finish_vector_place(result)
    }

    /// Drop any session, restoring the document from before a lift, or the
    /// layer's untrimmed extent (the Move tool's controls box opens on every
    /// selected layer, and a trimmed layer would clip later filters).
    fn drop_transform_session(mut self: Pin<&mut Self>) {
        let session = self.as_mut().rust_mut().session.live.take_transform();
        if let Some(session) = &session {
            for (member, rect) in &session.untrimmed {
                if let Some(doc) = self.as_mut().rust_mut().session.doc.as_mut() {
                    pictura_render::untrim_to(doc, member, *rect);
                }
            }
        }
        if let Some((_, before)) = session.and_then(|session| session.lifted) {
            self.as_mut().rust_mut().session.doc.set(Some(*before));
            self.as_mut().recomposite();
        }
        // The controller's own cancel route: the flag goes with the session, so
        // no caller that skips `transform_set_controls` can leave it set.
        self.as_mut().rust_mut().transform_controls = false;
    }

    /// Commit the session: one transform op (the similarity, an affine quad
    /// when a shear is set, or `transform_layer_quad` in a projective mode), or
    /// when a warp is stored the warp followed by the box's frame, a
    /// recomposite, and one `"Free Transform"` history state on success. An
    /// identity transform or warp records nothing; an engine refusal records
    /// nothing. The session always clears.
    pub fn commit_transform(mut self: Pin<&mut Self>) -> bool {
        memory_gate::clear();
        self.as_mut().rust_mut().transform_controls = false;
        if self.rust().selection_transform_live() {
            return self.as_mut().commit_selection_transform();
        }
        if self
            .rust()
            .session
            .live
            .transform()
            .is_some_and(|s| s.path_transform.is_some())
        {
            return super::path::commit(self.as_mut());
        }
        let Some((
            path,
            members,
            projective,
            quad,
            preview_quad,
            params,
            orig_rect,
            interpolation,
            warp,
            history_label,
        )) = self.rust().session.live.transform().map(|session| {
            let quad = session_quad(session);
            let preview_quad = if session.warp.is_some() {
                super::warp::warp_preview_quad(session).unwrap_or(quad)
            } else {
                quad
            };
            let members: Vec<(String, PsdRect)> = session
                .members
                .iter()
                .map(|(member, rect)| (member.clone(), *rect))
                .collect();
            (
                session.path.clone(),
                members,
                quad_driven(session),
                quad,
                preview_quad,
                MapParams::of(session),
                session.orig_rect,
                session.interpolation,
                session.warp.clone(),
                session.history_label.clone(),
            )
        })
        else {
            self.as_mut().clear_transform_preview();
            return false;
        };
        let repeat = self
            .rust()
            .session
            .live
            .transform()
            .and_then(repeatable_transform);
        let identity = identity_transform(quad, orig_rect, params, warp.as_ref(), projective);
        if !identity {
            if let Some(changed) = self.as_mut().commit_smart_object_rect(
                &path,
                &members,
                params,
                warp.as_ref(),
                projective,
                quad,
                &history_label,
                orig_rect,
                repeat,
            ) {
                return changed;
            }
        }
        // One admission answers both: whether the commit fits, and whether it can
        // only fit by spilling (#1290). An identity commit changes nothing, so it
        // never asks.
        let spill = match commit_decision(&preview_quad, orig_rect, warp.is_some(), identity) {
            CommitDecision::Identity => {
                return self.as_mut().finish_transform(VectorPlaceResult::Committed)
            }
            CommitDecision::Refused => {
                return self.as_mut().finish_transform(VectorPlaceResult::Refused)
            }
            CommitDecision::Admitted { spill } => spill,
        };
        let area = u64::from(orig_rect.width().max(0) as u32)
            * u64::from(orig_rect.height().max(0) as u32);
        // A large engine, or one that will spill tiles to scratch (#1290), runs
        // as a cancellable job (#1278); the session stays live until it lands,
        // so a cancel resumes it.
        let vector = has_vector_placement(self.rust().session.doc.as_ref(), &members);
        if vector || area >= DEFERRED_TRANSFORM_PIXELS || spill {
            let lifted = self
                .rust()
                .session
                .live
                .transform()
                .is_some_and(|s| s.lifted.is_some());
            let (paths, apply) = if lifted {
                let apply = lifted_apply(
                    path.clone(),
                    projective,
                    quad,
                    params,
                    interpolation,
                    warp,
                    preview_quad,
                    orig_rect,
                );
                (vec![path], apply)
            } else {
                let paths = members.iter().map(|(member, _)| member.clone()).collect();
                let apply = group_apply(
                    members,
                    projective,
                    quad,
                    params,
                    interpolation,
                    warp,
                    preview_quad,
                    orig_rect,
                );
                (paths, apply)
            };
            let spec = super::super::impl_core::TransformSpec {
                paths,
                orig_rect,
                apply,
            };
            return self
                .as_mut()
                .rust_mut()
                .defer_transform(&history_label, spec, lifted, repeat);
        }
        let mut session = self.as_mut().rust_mut().session.live.take_transform();
        let lifted = session.as_mut().and_then(|session| session.lifted.take());
        if let Some((_, before)) = lifted {
            let changed = match (self.as_mut().rust_mut().session.doc.as_mut(), &warp) {
                (Some(doc), Some(warp)) => pictura_render::transform_layer_warp(
                    doc,
                    &path,
                    warp,
                    Some(preview_quad),
                    interpolation,
                ),
                (Some(doc), None) => apply_session_transform(
                    doc,
                    &path,
                    projective,
                    quad,
                    params,
                    orig_rect,
                    interpolation,
                ),
                (None, _) => false,
            };
            let merged = changed
                && self
                    .as_mut()
                    .rust_mut()
                    .session
                    .doc
                    .as_mut()
                    .is_some_and(|doc| pictura_render::merge_lifted(doc, &path));
            if !merged {
                self.as_mut().rust_mut().session.doc.set(Some(*before));
                self.as_mut().recomposite();
                self.as_mut().clear_transform_preview();
                return false;
            }
            // ponytail: CS6 carries the selection border through the transform; dropped here.
            let mut rust = self.as_mut().rust_mut();
            rust.session.set_selection_in_place(None);
            rust.session.last_transform = repeat;
            // A lifted transform changes pixels, not the layer tree, so it
            // keeps the link set; only the pixels moved.
            self.as_mut()
                .commit_transform_edit(&history_label, true, |_| Edit::Full);
            if !self.rust().move_composite_pending() {
                self.as_mut().move_committed();
            }
            self.as_mut().clear_transform_preview();
            return true;
        }

        let apply = group_apply(
            members.clone(),
            projective,
            quad,
            params,
            interpolation,
            warp.clone(),
            preview_quad,
            orig_rect,
        );
        let changed = self
            .as_mut()
            .rust_mut()
            .session
            .doc
            .as_mut()
            .is_some_and(|doc| (apply)(doc));
        if changed {
            self.as_mut().rust_mut().session.last_transform = repeat;
            let edit = self.rust().session.doc.as_ref().map_or(Edit::Full, |doc| {
                transform_group_edit(doc, &members, orig_rect)
            });
            self.as_mut()
                .commit_transform_edit(&history_label, true, |_| edit);
            if !self.rust().move_composite_pending() {
                self.as_mut().move_committed();
            }
        }
        self.as_mut().clear_transform_preview();
        changed
    }
}

impl qobject::PictureView {
    /// Drop the transform overlay's prepared base/layer. The C++ canvas clears
    /// its copy separately; this keeps the Rust side from handing a stale
    /// preview to a later undo or move.
    pub fn clear_transform_preview(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().clear_preview_overlay();
    }
}

impl PictureViewRust {
    /// Prepare a transform session on `path` in `mode`: the layer is trimmed to
    /// its content (or the selection lifted onto a floating layer) and the
    /// session's scalars are set. `None` when the target cannot be
    /// transformed; the caller restores the document when the preview fails.
    pub(in crate::cxxqt_object) fn prepare_transform(
        &mut self,
        path: &str,
        mode: TransformMode,
    ) -> Option<TransformSession> {
        let lift = lifts_selection(self, path);
        if !can_begin_mode(self, path, mode) {
            return None;
        }
        // The outgoing session goes first: its trims and its floating layer go
        // back, so the new session starts from the same document. Its preview
        // goes with it, so a refusal below cannot leave it on the canvas.
        let outgoing = self.session.live.take_transform();
        if outgoing.is_some() {
            self.transform_controls = false;
            self.clear_preview_overlay();
        }
        if let Some(session) = &outgoing {
            for (member, rect) in &session.untrimmed {
                if let Some(doc) = self.session.doc.as_mut() {
                    pictura_render::untrim_to(doc, member, *rect);
                }
            }
        }
        if let Some((_, before)) = outgoing.and_then(|session| session.lifted) {
            self.session.doc.set(Some(*before));
        }
        let mut path = path.to_string();
        let mut lifted = None;
        let mut untrimmed: Vec<(String, PsdRect)> = Vec::new();
        let mut members: Vec<(String, PsdRect)> = Vec::new();
        if lift {
            let coverage = lift_coverage(self)?;
            let doc = self.session.doc.as_mut()?;
            let before = Box::new(doc.clone());
            let floating = pictura_render::lift_selection(doc, &path, &coverage);
            if floating.is_empty() {
                *doc = *before;
                return None;
            }
            lifted = Some((std::mem::replace(&mut path, floating), before));
            let rect = self
                .session
                .doc
                .as_ref()
                .and_then(|doc| transform_rect(doc, &path));
            if let Some(rect) = rect {
                members.push((path.clone(), rect));
            }
        } else {
            let group = {
                let doc = self.session.doc.as_ref()?;
                transform_group(self, doc, &path)
            };
            for member in group {
                // The box hugs each layer's visible pixels, not its (often
                // canvas-sized) rect; the composite is unchanged. A hidden
                // linked partner is trimmed too: it transforms with the set.
                let before = self
                    .session
                    .doc
                    .as_ref()
                    .and_then(|doc| pictura_render::resolve_path(doc, &member))
                    .map(|layer| layer.rect);
                let trimmed = self
                    .session
                    .doc
                    .as_mut()
                    .is_some_and(|doc| pictura_render::trim_to_content(doc, &member));
                if trimmed {
                    if let Some(rect) = before {
                        untrimmed.push((member.clone(), rect));
                    }
                }
                if let Some(rect) = self
                    .session
                    .doc
                    .as_ref()
                    .and_then(|doc| transform_rect(doc, &member))
                {
                    if rect.width() > 0 && rect.height() > 0 {
                        members.push((member, rect));
                    }
                }
            }
        }
        let Some(orig_rect) = members.iter().map(|(_, rect)| *rect).reduce(PsdRect::union) else {
            // A refused begin leaves every layer as it was.
            for (member, rect) in &untrimmed {
                if let Some(doc) = self.session.doc.as_mut() {
                    pictura_render::untrim_to(doc, member, *rect);
                }
            }
            if let Some((_, before)) = lifted {
                self.session.doc.set(Some(*before));
            }
            return None;
        };
        // Any editable type in the set keeps the projective gestures a
        // parallelogram (and refuses Distort/Perspective), as a single type
        // layer does.
        let type_layer = self.session.doc.as_ref().is_some_and(|doc| {
            members.iter().any(|(member, _)| {
                pictura_render::resolve_path(doc, member).is_some_and(|layer| layer.is_type())
            })
        });
        let history_label = if self.pending_vector_place.is_some() {
            "Place"
        } else {
            "Free Transform"
        };
        let mut session = TransformSession::new(
            TransformTarget::Layer,
            path,
            members,
            orig_rect,
            mode,
            type_layer,
            history_label,
        );
        session.lifted = lifted;
        session.untrimmed = untrimmed;
        Some(session)
    }

    /// Put back what a refused transform begin changed: the pre-trim layer
    /// extent and the pre-lift document.
    pub(in crate::cxxqt_object) fn restore_transform_begin(&mut self, session: TransformSession) {
        for (member, rect) in &session.untrimmed {
            if let Some(doc) = self.session.doc.as_mut() {
                pictura_render::untrim_to(doc, member, *rect);
            }
        }
        if let Some((_, before)) = session.lifted {
            self.session.doc.set(Some(*before));
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod cow_tests;
#[cfg(test)]
mod move_profile;
#[cfg(test)]
mod preview_tests;
