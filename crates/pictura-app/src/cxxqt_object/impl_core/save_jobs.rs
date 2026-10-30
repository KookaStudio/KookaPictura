//! Native document saves off the GUI thread.
//!
//! The GUI snapshots the document and the view bookkeeping, then a worker
//! refreshes the native composite, encodes through the session cache, and writes
//! atomically. `save_finish` joins the worker and applies the path/dirty result
//! only when the same document is still current; a replaced document is left
//! untouched.

mod imp;

use super::super::qobject::PictureView;
pub(in crate::cxxqt_object) use imp::*;

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
        /// Start a native `.psd`/`.psb` save on a worker. False for a raster or
        /// PDF path, no document, or a save already running.
        fn save_start(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Whether a started native save is still running.
        fn save_running(view: &PictureView) -> bool;

        /// Join the started native save, record its path, and clear modified
        /// when the same document is still current. False when the worker failed
        /// or the document was replaced.
        fn save_finish(view: Pin<&mut PictureView>) -> bool;

        /// Hold a started save's completion for `ms` milliseconds (test seam).
        fn save_set_delay(view: Pin<&mut PictureView>, ms: u64);
    }
}
