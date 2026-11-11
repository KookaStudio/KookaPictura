//! The composite of a large Move commit, built off the GUI thread while the
//! canvas keeps presenting the drag preview (#1157).

use super::super::super::edit::Edit;
use super::super::super::helpers::layer_has_effects;
use super::super::super::helpers_composite::{composite_regions, RegionComposite};
use super::super::super::qobject::PictureView;
use super::super::super::{metrics, qobject, PictureViewRust};
use crate::layer_target::LayerTarget;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use pictura_core::{Document, PsdRect};
use std::time::Duration;

/// The document and dirty-union size at which a Move commit leaves its
/// composite to a worker. Mirrors the deferred-pyramid threshold.
pub(in crate::cxxqt_object) const DEFERRED_MOVE_PIXELS: u64 = 16_000_000;

/// A Move commit's composite waiting for its worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::cxxqt_object) struct DeferredComposite {
    pub(in crate::cxxqt_object) generation: u64,
    pub(in crate::cxxqt_object) label: String,
    pub(in crate::cxxqt_object) hint: Option<Vec<PsdRect>>,
    pub(in crate::cxxqt_object) rects: Vec<PsdRect>,
    pub(in crate::cxxqt_object) target: LayerTarget,
}

/// What a worker needs: the generation it answers, the document it reads, and
/// the dirty rectangles.
pub(in crate::cxxqt_object) struct CompositeJob {
    pub(in crate::cxxqt_object) generation: u64,
    pub(in crate::cxxqt_object) doc: Document,
    pub(in crate::cxxqt_object) rects: Vec<PsdRect>,
    pub(in crate::cxxqt_object) gpu_compute: bool,
    pub(in crate::cxxqt_object) delay_ms: u64,
}

/// The engine call and layer extents a deferred transform needs.
#[derive(Clone)]
pub(in crate::cxxqt_object) struct TransformSpec {
    /// Every layer the transform acts on (one, or a linked set).
    pub(in crate::cxxqt_object) paths: Vec<String>,
    pub(in crate::cxxqt_object) orig_rect: PsdRect,
    pub(in crate::cxxqt_object) apply: std::sync::Arc<dyn Fn(&mut Document) -> bool + Send + Sync>,
}

/// A transform engine running on a worker.
#[derive(Clone)]
pub(in crate::cxxqt_object) struct DeferredTransform {
    pub(in crate::cxxqt_object) generation: u64,
    pub(in crate::cxxqt_object) label: String,
    pub(in crate::cxxqt_object) spec: TransformSpec,
}

/// What a transform worker needs: the generation, the engine call and its
/// document snapshot.
pub(in crate::cxxqt_object) struct TransformJob {
    generation: u64,
    spec: TransformSpec,
    doc: Document,
    delay_ms: u64,
}

/// The blit a settle or a landing owes the shell. A blit over stale tiles
/// makes the canvas ask for the visible ones, which land on a worker.
pub(in crate::cxxqt_object) struct SettledMove {
    blit: Option<PsdRect>,
}

impl PictureViewRust {
    /// Whether a Move commit's composite should leave the GUI thread.
    pub(in crate::cxxqt_object) fn should_defer_move(&self, edit: &Edit) -> bool {
        let Some(doc) = self.session.doc.as_ref() else {
            return false;
        };
        if self.has_active_session() || self.pending_previews.any() || self.mask_solo.is_some() {
            return false;
        }
        let canvas = u64::from(doc.width) * u64::from(doc.height);
        let area = match edit {
            Edit::Regions { rects, .. } => rects.iter().map(rect_area).sum(),
            Edit::Full | Edit::Structural { damage: None } => canvas,
            _ => return false,
        };
        canvas >= DEFERRED_MOVE_PIXELS && area >= DEFERRED_MOVE_PIXELS
    }

    /// Leave the already-translated layer's composite to a worker. The job is
    /// parked on the view for the QObject to spawn after the transaction
    /// returns.
    pub(in crate::cxxqt_object) fn defer_move_composite(
        &mut self,
        label: &str,
        edit: &Edit,
    ) -> bool {
        let rects = match edit {
            Edit::Regions { rects, .. } => rects.clone(),
            Edit::Full | Edit::Structural { damage: None } => vec![self.canvas_rect()],
            _ => return false,
        };
        let Some(doc) = self.session.doc.clone() else {
            return false;
        };
        let hint = edit.history_hint().map(<[PsdRect]>::to_vec);
        let generation = self.composite_generation.wrapping_add(1);
        self.composite_generation = generation;
        // The move is committed even though its composite is not current yet.
        self.session.dirty = true;
        self.deferred_composite = Some(DeferredComposite {
            generation,
            label: label.to_string(),
            hint,
            rects: rects.clone(),
            target: self.layer_target.clone(),
        });
        self.pending_move_job = Some(CompositeJob {
            generation,
            doc,
            rects,
            gpu_compute: self.gpu_compute,
            delay_ms: self.move_composite_delay_for_test,
        });
        true
    }

    /// Leave a transform engine to a worker. The job is parked on the view for
    /// the QObject to spawn after the commit returns.
    pub(in crate::cxxqt_object) fn defer_transform(
        &mut self,
        label: &str,
        spec: TransformSpec,
    ) -> bool {
        let Some(doc) = self.session.doc.clone() else {
            return false;
        };
        let generation = self.transform_generation.wrapping_add(1);
        self.transform_generation = generation;
        self.deferred_transform = Some(DeferredTransform {
            generation,
            label: label.to_string(),
            spec: spec.clone(),
        });
        self.pending_transform_job = Some(TransformJob {
            generation,
            spec,
            doc,
            delay_ms: self.transform_delay_for_test,
        });
        true
    }

    /// Whether a transform engine is still on its worker.
    pub(in crate::cxxqt_object) fn transform_pending(&self) -> bool {
        self.deferred_transform.is_some()
    }

    /// Install a worker's transformed layers and report the edit to commit.
    pub(in crate::cxxqt_object) fn land_transform(
        &mut self,
        generation: u64,
        layers: Option<Vec<(String, pictura_core::Layer)>>,
    ) -> Option<(String, Edit)> {
        let pending = self.deferred_transform.clone()?;
        if pending.generation != generation {
            return None;
        }
        self.deferred_transform = None;
        self.pending_transform_job = None;
        let layers = layers?;
        let mut dirty = pending.spec.orig_rect;
        {
            let doc = self.session.doc.as_mut()?;
            for (path, layer) in layers {
                let dst = pictura_render::resolve_path_mut(doc, &path)?;
                *dst = layer;
                dirty = dirty.union(dst.rect);
            }
        }
        Some((
            pending.label,
            transform_edit(self, &pending.spec.paths, dirty),
        ))
    }

    /// Apply a pending transform engine on the GUI thread and report its edit.
    pub(in crate::cxxqt_object) fn settle_transform_engine(&mut self) -> Option<(String, Edit)> {
        let pending = self.deferred_transform.take()?;
        self.pending_transform_job = None;
        let spec = pending.spec.clone();
        let applied = self
            .session
            .doc
            .as_mut()
            .is_some_and(|doc| (spec.apply)(doc));
        if !applied {
            return None;
        }
        let dirty = self.session.doc.as_ref().map_or(spec.orig_rect, |doc| {
            spec.paths.iter().fold(spec.orig_rect, |dirty, path| {
                pictura_render::resolve_path(doc, path)
                    .map_or(dirty, |layer| dirty.union(layer.rect))
            })
        });
        Some((pending.label, transform_edit(self, &spec.paths, dirty)))
    }

    /// Patch a stored composite and fold each region into the display tiles
    /// and the damage account, returning their bounding box. With `defer`, a
    /// whole-document region on a large document leaves the tiles stale
    /// instead: repairing every cached level from it would stall the GUI
    /// thread, and only the visible tiles need rebuilding.
    fn apply_composite_regions(
        &mut self,
        regions: Vec<RegionComposite>,
        defer: bool,
    ) -> Option<PsdRect> {
        let large = self.session.doc.as_ref().is_some_and(|doc| {
            u64::from(doc.width) * u64::from(doc.height) > super::super::super::tiles::SYNC_PIXELS
        });
        let mut union: Option<PsdRect> = None;
        for region in regions {
            let whole = self
                .session
                .doc
                .as_ref()
                .is_some_and(|doc| region.is_whole(doc));
            if let Some(doc) = self.session.doc.as_mut() {
                let mut cache = pictura_render::DocumentComposite::new(doc);
                if whole {
                    cache.store(&region.raw);
                } else {
                    cache.patch(&region.raw, region.x0, region.y0);
                }
            }
            if defer && whole && large {
                self.invalidate_tiles();
                self.damage.mark(region.clipped);
                self.canvas_revision = self.canvas_revision.wrapping_add(1);
                self.sync_mask_solo_region(region.clipped);
            } else {
                self.apply_refreshed_region(region.x0, region.y0, region.clipped, region.srgb);
            }
            union = Some(union.map_or(region.clipped, |u| u.union(region.clipped)));
        }
        union
    }

    fn finish_move_composite(
        &mut self,
        pending: DeferredComposite,
        regions: Vec<RegionComposite>,
        defer_pyramid: bool,
    ) -> SettledMove {
        let blit = self.apply_composite_regions(regions, defer_pyramid);
        let target = pending.target;
        self.session.capture_move_without_bookkeeping(
            &target,
            &pending.label,
            pending.hint.as_deref(),
        );
        metrics::mark_composited();
        metrics::end_edit();
        SettledMove { blit }
    }

    /// Finish a pending Move composite synchronously, before a reader or
    /// writer runs.
    pub(in crate::cxxqt_object) fn settle_move_composite(&mut self) -> Option<SettledMove> {
        let pending = self.deferred_composite.take()?;
        self.pending_move_job = None;
        let Some(regions) = self
            .session
            .doc
            .as_ref()
            .map(|doc| composite_regions(doc, &pending.rects, self.gpu_compute))
        else {
            metrics::cancel_edit();
            return None;
        };
        Some(self.finish_move_composite(pending, regions, false))
    }

    /// Install a worker's result unless a settle or a later commit superseded
    /// it.
    pub(in crate::cxxqt_object) fn land_move_composite(
        &mut self,
        generation: u64,
        regions: Vec<RegionComposite>,
    ) -> Option<SettledMove> {
        if self.deferred_composite.as_ref().map(|d| d.generation) != Some(generation) {
            return None;
        }
        if self.session.doc.is_none() {
            self.cancel_move_composite();
            return None;
        }
        let pending = self.deferred_composite.take()?;
        Some(self.finish_move_composite(pending, regions, true))
    }

    /// Drop a pending Move composite or transform: its document is going away.
    pub(in crate::cxxqt_object) fn cancel_move_composite(&mut self) {
        self.pending_move_job = None;
        self.pending_transform_job = None;
        self.deferred_transform = None;
        if self.deferred_composite.take().is_some() {
            metrics::cancel_edit();
        }
    }

    /// Whether a Move composite is still on its worker.
    pub(in crate::cxxqt_object) fn move_composite_pending(&self) -> bool {
        self.deferred_composite.is_some()
    }

    /// A pending move is an undoable state even though its history entry has
    /// not been captured yet.
    pub(in crate::cxxqt_object) fn can_undo(&self) -> bool {
        let pending = self.deferred_composite.is_some() || self.deferred_transform.is_some();
        self.session.history.can_undo() || (pending && self.session.history.count() > 0)
    }

    /// A new capture would truncate redo, so redo is unavailable while a move
    /// or transform is pending.
    pub(in crate::cxxqt_object) fn can_redo(&self) -> bool {
        self.deferred_composite.is_none()
            && self.deferred_transform.is_none()
            && self.session.history.can_redo()
    }
}

fn rect_area(rect: &PsdRect) -> u64 {
    u64::from(rect.width().max(0) as u32) * u64::from(rect.height().max(0) as u32)
}

/// The edit a transformed layer set needs: a bounded region, or the whole
/// document when a member's effects can spread outside its rect.
fn transform_edit(rust: &PictureViewRust, paths: &[String], dirty: PsdRect) -> Edit {
    if rust.session.doc.as_ref().is_some_and(|doc| {
        paths
            .iter()
            .any(|path| pictura_render::resolve_path(doc, path).is_some_and(layer_has_effects))
    }) {
        Edit::Full
    } else {
        Edit::regions(vec![dirty], false)
    }
}

impl qobject::PictureView {
    /// Composite a Move job on a worker and land it on the GUI thread. A
    /// destroyed view drops the result.
    pub(in crate::cxxqt_object) fn build_move_composite(self: Pin<&mut Self>, job: CompositeJob) {
        let qt_thread = self.as_ref().get_ref().qt_thread();
        std::thread::spawn(move || {
            let CompositeJob {
                generation,
                doc,
                rects,
                gpu_compute,
                delay_ms,
            } = job;
            let regions = composite_regions(&doc, &rects, gpu_compute);
            // Before landing, so a write does not copy planes nobody reads.
            drop(doc);
            if delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(delay_ms));
            }
            qt_thread
                .queue(move |mut view| {
                    if let Some(settled) = view
                        .as_mut()
                        .rust_mut()
                        .land_move_composite(generation, regions)
                    {
                        view.as_mut().show_settled_move(settled);
                    }
                })
                .ok();
        });
    }

    /// Run a transform engine on a worker, install its layer on the GUI thread,
    /// then hand its composite to the deferred edit.
    pub(in crate::cxxqt_object) fn build_transform(self: Pin<&mut Self>, job: TransformJob) {
        let qt_thread = self.as_ref().get_ref().qt_thread();
        std::thread::spawn(move || {
            let TransformJob {
                generation,
                spec,
                mut doc,
                delay_ms,
            } = job;
            let layers = {
                let applied = (spec.apply)(&mut doc);
                applied.then(|| {
                    spec.paths
                        .iter()
                        .filter_map(|path| {
                            pictura_render::resolve_path(&doc, path)
                                .cloned()
                                .map(|layer| (path.clone(), layer))
                        })
                        .collect::<Vec<_>>()
                })
            };
            // Before landing, so a write does not copy planes nobody reads.
            drop(doc);
            if delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(delay_ms));
            }
            qt_thread
                .queue(move |mut view| {
                    if let Some((label, edit)) =
                        view.as_mut().rust_mut().land_transform(generation, layers)
                    {
                        view.as_mut().commit_transform_edit(&label, true, |_| edit);
                        if !view.rust().move_composite_pending() {
                            view.as_mut().move_committed();
                        }
                    } else if !view.rust().transform_pending() && !view.rust().move_drag_active {
                        view.as_mut().move_committed();
                    }
                })
                .ok();
        });
    }

    /// Finish a pending transform or Move composite before a reader or writer
    /// runs.
    pub(in crate::cxxqt_object) fn settle_pending_composite(mut self: Pin<&mut Self>) {
        let had_transform = self.rust().transform_pending();
        if let Some((label, edit)) = self.as_mut().rust_mut().settle_transform_engine() {
            self.as_mut().commit_transform_edit(&label, false, |_| edit);
            if !self.rust().move_composite_pending() {
                self.as_mut().move_committed();
            }
            return;
        }
        if had_transform && !self.rust().move_drag_active {
            self.as_mut().move_committed();
        }
        if let Some(settled) = self.as_mut().rust_mut().settle_move_composite() {
            self.as_mut().show_settled_move(settled);
        }
    }

    fn show_settled_move(mut self: Pin<&mut Self>, settled: SettledMove) {
        if let Some(rect) = settled.blit {
            if rect.width() > 0 && rect.height() > 0 {
                self.as_mut()
                    .region_damaged(rect.left, rect.top, rect.width(), rect.height());
            }
        }
        // A second drag may have started while this commit was on its worker;
        // its preview owns the canvas now.
        if !self.rust().move_drag_active {
            self.as_mut().move_committed();
        }
    }
}

pub(in crate::cxxqt_object) fn move_composite_pending(view: &PictureView) -> bool {
    view.rust().move_composite_pending()
}

pub(in crate::cxxqt_object) fn move_composite_set_delay(mut view: Pin<&mut PictureView>, ms: u64) {
    view.as_mut().rust_mut().move_composite_delay_for_test = ms;
}

pub(in crate::cxxqt_object) fn transform_pending(view: &PictureView) -> bool {
    view.rust().transform_pending()
}

pub(in crate::cxxqt_object) fn transform_set_delay(mut view: Pin<&mut PictureView>, ms: u64) {
    view.as_mut().rust_mut().transform_delay_for_test = ms;
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
