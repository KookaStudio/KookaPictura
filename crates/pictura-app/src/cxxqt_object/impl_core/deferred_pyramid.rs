//! The exact pyramid of a large whole-document restore, built off the GUI
//! thread while the canvas presents a coarse level (#1098).

use super::super::helpers_composite::planes_of;
use super::super::{qobject, PictureViewRust};
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use pictura_core::PixelBuffer;
use pictura_render::ViewPyramid;

/// Larger documents build their pyramid on a worker after a restore.
const DEFERRED_PIXELS: u64 = 16_000_000;

/// A coarse `pyramid` waiting for its worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::cxxqt_object) struct DeferredPyramid {
    pub(in crate::cxxqt_object) generation: u64,
    /// The finest level holding pixels, which the canvas presents.
    pub(in crate::cxxqt_object) level: usize,
}

/// What a worker needs: the generation it answers, and level 0 to build from.
pub(in crate::cxxqt_object) struct PyramidJob {
    generation: u64,
    level0: PixelBuffer,
}

impl PictureViewRust {
    /// The level to present coarsely, or `None` to rebuild synchronously.
    pub(super) fn coarse_level(&self) -> Option<usize> {
        if self.session.live.stroke().is_some() {
            return None;
        }
        let level0 = self.level0.as_ref()?;
        planes_of(level0)?;
        (u64::from(level0.width) * u64::from(level0.height) > DEFERRED_PIXELS)
            .then(|| ViewPyramid::coarse_level(level0.width, level0.height))
    }

    /// Rebuild the display for a whole-document restore, returning the job
    /// when the exact pyramid is left to a worker.
    pub(in crate::cxxqt_object) fn reset_pyramid_deferred(&mut self) -> Option<PyramidJob> {
        self.reset_display(true);
        Some(PyramidJob {
            generation: self.deferred_pyramid?.generation,
            level0: self.level0.clone()?,
        })
    }

    /// Install a worker's `pyramid` unless a later reset superseded it.
    fn land_pyramid(&mut self, generation: u64, pyramid: ViewPyramid) -> bool {
        if self.deferred_pyramid.map(|d| d.generation) != Some(generation) {
            return false;
        }
        self.deferred_pyramid = None;
        self.pyramid = pyramid;
        self.canvas_revision = self.canvas_revision.wrapping_add(1);
        true
    }

    /// Build the exact pyramid now, for a reader or writer of the levels the
    /// coarse stand-in left empty. Any worker result is then dropped: it may
    /// predate the write.
    pub(in crate::cxxqt_object) fn settle_pyramid(&mut self) {
        if self.deferred_pyramid.take().is_none() {
            return;
        }
        if let Some(planes) = self.level0.as_ref().and_then(planes_of) {
            self.pyramid = ViewPyramid::rebuild(planes);
            self.canvas_revision = self.canvas_revision.wrapping_add(1);
        }
    }

    /// Bring a soloed mask up to a restore that changed masks.
    pub(in crate::cxxqt_object) fn refresh_mask_solo(&mut self) {
        if self.mask_solo.is_some() {
            self.rebuild_mask_solo_source();
            self.frame_revision = self.frame_revision.wrapping_add(1);
        }
    }
}

impl qobject::PictureView {
    /// Build `job`'s pyramid on a worker and land it on the GUI thread. A
    /// destroyed view drops the result.
    pub(in crate::cxxqt_object) fn build_pyramid(self: Pin<&mut Self>, job: PyramidJob) {
        let qt_thread = self.as_ref().get_ref().qt_thread();
        std::thread::spawn(move || {
            let PyramidJob { generation, level0 } = job;
            let pyramid = planes_of(&level0)
                .map(ViewPyramid::rebuild)
                .unwrap_or_default();
            // Before landing, so a write does not copy a plane nobody reads.
            drop(level0);
            qt_thread
                .queue(move |mut view| {
                    if view.as_mut().rust_mut().land_pyramid(generation, pyramid) {
                        view.pyramid_settled();
                    }
                })
                .ok();
        });
    }
}

#[cfg(test)]
#[path = "deferred_pyramid_tests.rs"]
mod tests;
