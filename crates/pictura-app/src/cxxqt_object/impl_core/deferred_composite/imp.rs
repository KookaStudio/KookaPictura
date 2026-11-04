//! The composite of a large Move commit, built off the GUI thread while the
//! canvas keeps presenting the drag preview (#1157).

use super::super::super::edit::Edit;
use super::super::super::helpers_composite::{composite_regions, RegionComposite};
use super::super::super::qobject::PictureView;
use super::super::super::{metrics, qobject, PictureViewRust};
use super::super::PyramidJob;
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

/// The blit and deferred pyramid a settle or a landing owes the shell.
pub(in crate::cxxqt_object) struct SettledMove {
    blit: Option<PsdRect>,
    pyramid: Option<PyramidJob>,
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

    /// Patch the composite and fold each region into level 0, damage and the
    /// pyramid. Returns their bounding box and, when `defer_pyramid` is set and
    /// the region is the whole document, the pyramid job for a worker.
    fn apply_composite_regions(
        &mut self,
        regions: Vec<RegionComposite>,
        defer_pyramid: bool,
    ) -> (Option<PsdRect>, Option<PyramidJob>) {
        let defer_pyramid = defer_pyramid
            && self.coarse_level().is_some()
            && self
                .session
                .doc
                .as_ref()
                .is_some_and(|doc| regions.iter().any(|region| region.is_whole(doc)));
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
            if defer_pyramid {
                self.apply_refreshed_region_deferred_pyramid(
                    region.x0,
                    region.y0,
                    region.clipped,
                    region.srgb,
                );
            } else {
                self.apply_refreshed_region(region.x0, region.y0, region.clipped, region.srgb);
            }
            union = Some(union.map_or(region.clipped, |u| u.union(region.clipped)));
        }
        let pyramid = defer_pyramid
            .then(|| self.defer_pyramid_from_level0())
            .flatten();
        (union, pyramid)
    }

    fn finish_move_composite(
        &mut self,
        pending: DeferredComposite,
        regions: Vec<RegionComposite>,
        defer_pyramid: bool,
    ) -> SettledMove {
        let (blit, pyramid) = self.apply_composite_regions(regions, defer_pyramid);
        let target = pending.target;
        self.session.capture_move_without_bookkeeping(
            &target,
            &pending.label,
            pending.hint.as_deref(),
        );
        metrics::mark_composited();
        metrics::end_edit();
        SettledMove { blit, pyramid }
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

    /// Drop a pending Move composite: its document is going away.
    pub(in crate::cxxqt_object) fn cancel_move_composite(&mut self) {
        self.pending_move_job = None;
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
        self.session.history.can_undo()
            || (self.deferred_composite.is_some() && self.session.history.count() > 0)
    }

    /// A new capture would truncate redo, so redo is unavailable while a move
    /// is pending.
    pub(in crate::cxxqt_object) fn can_redo(&self) -> bool {
        self.deferred_composite.is_none() && self.session.history.can_redo()
    }
}

fn rect_area(rect: &PsdRect) -> u64 {
    u64::from(rect.width().max(0) as u32) * u64::from(rect.height().max(0) as u32)
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

    /// Finish a pending Move composite before a reader or writer runs.
    pub(in crate::cxxqt_object) fn settle_pending_composite(mut self: Pin<&mut Self>) {
        if let Some(settled) = self.as_mut().rust_mut().settle_move_composite() {
            self.as_mut().show_settled_move(settled);
        }
    }

    fn show_settled_move(mut self: Pin<&mut Self>, settled: SettledMove) {
        if let Some(job) = settled.pyramid {
            self.as_mut().build_pyramid(job);
        }
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

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
