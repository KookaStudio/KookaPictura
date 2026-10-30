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
        /// Start writing the document as a PSD (PSB past the PSD size limit) to `path` on a worker thread, after waiting for any earlier write. The path and dirty flag are untouched. False without a document.
        fn recovery_write(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Wait for the write `recovery_write` started; whether it landed (true when none was pending).
        fn recovery_wait(view: Pin<&mut PictureView>) -> bool;

        /// Open the snapshot at `path` as an untitled document with unsaved changes, so Save asks for a destination. False when it cannot be read.
        fn recovery_open(view: Pin<&mut PictureView>, path: &QString) -> bool;
    }
}
