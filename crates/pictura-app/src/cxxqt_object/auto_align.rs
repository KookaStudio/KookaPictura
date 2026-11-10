//! Edit > Auto-Align Layers over the Layers panel's selected layers. Free
//! functions over a [`PictureView`] (their own bridge, so the `PictureView`
//! declaration list does not grow). Matching runs on a worker over a document
//! clone and lands as one history state.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
use imp::*;

pub(super) use imp::AutoAlignJob;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Whether Auto-Align applies to `paths`: two eligible layers.
        fn auto_align_can(view: &PictureView, paths: &QStringList) -> bool;

        /// Start a run on a worker. `projection` is the dialog order, 0 Auto …
        /// 6 Lens Correction. False when it does not apply or one is running.
        fn auto_align_start(
            view: Pin<&mut PictureView>,
            paths: &QStringList,
            projection: i32,
            vignette: bool,
            geometric: bool,
        ) -> bool;

        /// Whether the worker is still computing.
        fn auto_align_running(view: &PictureView) -> bool;

        /// Progress of the run, 0 to 1000.
        fn auto_align_progress(view: &PictureView) -> i32;

        /// Ask the worker to stop; [`auto_align_finish`] then commits nothing.
        fn auto_align_cancel(view: Pin<&mut PictureView>);

        /// Land a finished run as one history state. Returns the moved layers'
        /// paths, empty when nothing was committed.
        fn auto_align_finish(view: Pin<&mut PictureView>) -> QStringList;

        /// Why the last run skipped a layer or the lens correction. Empty
        /// otherwise.
        fn auto_align_notice(view: &PictureView) -> QString;

        /// Hold the worker for `ms` before it starts, so a test can act mid-run.
        fn auto_align_set_delay_for_test(view: Pin<&mut PictureView>, ms: i32);
    }
}
