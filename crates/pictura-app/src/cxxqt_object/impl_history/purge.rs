//! Edit > Purge ▸ Undo / Histories / All: drop the undo history and the named
//! restore points. Free functions over a [`PictureView`] (their own bridge, so
//! the `PictureView` declaration list does not grow). `Purge ▸ Clipboard` lives
//! in the clipboard bridge (`clipboard_purge`). Ported from photorust's
//! `Edit > Purge` handlers.
//!
//! [`PictureView`]: super::super::qobject::PictureView

mod frame_capture;
mod imp;

use super::super::qobject::PictureView;
use super::display::{frame_revision, preview_present_level, tiles_pending};
use frame_capture::*;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// `Purge ▸ All`: drop every undo state but the current one and every
        /// named restore point; emit `changed` so the History panel refreshes.
        /// Returns false without a document.
        fn purge_all(view: Pin<&mut PictureView>) -> bool;

        /// `Purge ▸ Undo`: drop the undo history but keep the named restore
        /// points. Returns false without a document.
        fn purge_history(view: Pin<&mut PictureView>) -> bool;

        /// `Purge ▸ Histories`: drop the named restore points only, keeping the
        /// undo history. Returns false without a document.
        fn purge_named_history(view: Pin<&mut PictureView>) -> bool;

        /// Trim the history to History States now (#636); emit `changed`
        /// when a state went. Returns whether one did.
        fn apply_history_states(view: Pin<&mut PictureView>) -> bool;

        /// The label of the current history state, or empty without a document.
        fn current_history_label(view: &PictureView) -> QString;

        /// Number of named restore points (capped at 10).
        fn history_snapshot_count(view: &PictureView) -> i32;

        /// Label of named restore point `i`, or empty when out of range.
        fn history_snapshot_label(view: &PictureView, i: i32) -> QString;

        /// The revision of the whole level-0 frame: it changes only when the
        /// frame is rebuilt, never for a region the view blitted. A canvas that
        /// took `image()` at this revision is still current.
        fn frame_revision(view: &PictureView) -> u64;

        /// The view-pyramid level the canvas must present, or 0 when the zoom
        /// picks it: a reduced stroke preview's level, else the coarse level
        /// standing in for a pyramid a worker is still building. A GPU stroke
        /// presents the zoom-selected level.
        fn preview_present_level(view: &PictureView) -> i32;

        /// Whether display tiles are being built on a worker or queued for
        /// one (#1608): the canvas shows it is still rendering.
        fn tiles_pending(view: &PictureView) -> bool;
    }

    #[namespace = "pictura"]
    struct CanvasFrameMeasurement {
        canvas_id: u64,
        input_count: u32,
        input_ns: u64,
        tile_gather_ns: u64,
        paint_ns: u64,
        interval_ns: i64,
        gpu_ns: i64,
    }

    #[namespace = "pictura"]
    struct FrameCropRect {
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    }

    #[namespace = "pictura"]
    extern "Rust" {
        type FrameCropSamples;

        fn frame_capture_generation() -> u64;
        fn frame_capture_record(generation: u64, record: CanvasFrameMeasurement);
        fn frame_samples_new() -> Box<FrameCropSamples>;
        fn frame_samples_crop(
            view: &PictureView,
            level: i32,
            rect: FrameCropRect,
            samples: &mut FrameCropSamples,
        ) -> QImage;
        fn frame_samples_copy(
            target: &mut FrameCropSamples,
            source: &FrameCropSamples,
            area: FrameCropRect,
        );
        fn frame_samples_erase(samples: &mut FrameCropSamples, area: FrameCropRect);
        fn frame_samples_empty(samples: &FrameCropSamples) -> bool;
        fn frame_samples_prune_empty(samples: &mut FrameCropSamples, generation: u64) -> bool;
        fn frame_samples_rects(
            samples: &mut FrameCropSamples,
            generation: u64,
        ) -> Vec<FrameCropRect>;
        fn frame_samples_take(samples: &FrameCropSamples, index: usize, generation: u64) -> i64;
        fn frame_samples_seed_for_test(
            samples: &mut FrameCropSamples,
            rect: FrameCropRect,
            ns: u64,
        );
    }
}
