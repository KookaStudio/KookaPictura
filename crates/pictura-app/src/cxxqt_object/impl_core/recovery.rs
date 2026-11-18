//! Crash-recovery snapshots (`docs/11-cross-cutting/crash-recovery-and-autosave.md`):
//! write the document to a recovery file off the UI thread without touching its
//! path or dirty flag, and open such a file back as an unsaved, untitled
//! document. Free functions over a [`PictureView`] (their own bridge); the frame
//! owns where the files live.
//!
//! [`PictureView`]: super::super::qobject::PictureView

mod imp;

use super::super::qobject::PictureView;
use imp::*;
pub(in crate::cxxqt_object) use imp::{RecoveryAttachment, RecoveryIdentity};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Flush pending recovery records and wait for the worker's result.
        fn recovery_wait(view: Pin<&mut PictureView>) -> bool;

        /// Open the snapshot at `path` as an untitled document with unsaved changes, so Save asks for a destination. False when it cannot be read.
        fn recovery_open(view: Pin<&mut PictureView>, path: &QString) -> bool;

        fn recovery_track(
            view: Pin<&mut PictureView>,
            directory: &QString,
            id: u32,
            name: &QString,
            path: &QString,
            minutes: u32,
        );
        fn recovery_checkpoint(view: &PictureView);
        fn recovery_discard(view: Pin<&mut PictureView>, detach: bool) -> bool;
        fn recovery_discard_async(view: Pin<&mut PictureView>) -> bool;
        fn recovery_status(view: &PictureView) -> i32;
        fn recovery_fail_next_for_test(view: &PictureView, fault: i32);
        // 0 unreadable checkpoint, 1 restored, 2 partial, 3 keep the source.
        fn recovery_open_chain(view: Pin<&mut PictureView>, manifest: &QString) -> i32;
    }
}
