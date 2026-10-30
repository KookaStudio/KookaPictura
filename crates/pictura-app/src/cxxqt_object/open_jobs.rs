//! Document opens off the GUI thread (#729).
//!
//! The open entry points used to read, decode, composite, and capture history on
//! the GUI thread. This module splits each open into a pure half that runs on a
//! worker and an apply half that runs on the GUI thread when the worker queues
//! its result. The worker never touches the view; cancellation is the view's
//! flag (checked in the queued closure) or the view's destruction (the queue
//! fails).
//!
//! The synchronous `open` / `open_image` / `open_as_smart_object` / `pdf_open`
//! and the import bridges share the decode helpers, so Revert, Edit Contents,
//! the self-test, and the existing tests keep their behaviour.

mod imp;

use super::pdf_export::import::ffi::ImportTarget;
use super::qobject::PictureView;
pub use imp::*;

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

        include!("pictura_app/src/cxxqt_object/pdf_export/import.cxxqt.h");
        #[namespace = "pictura"]
        type ImportTarget = super::ImportTarget;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Start opening `path` as a PSD/PSB on a worker. The view's
        /// `open_finished` signal reports `"ok"` or `"error"`.
        fn open_psd_start(view: Pin<&mut PictureView>, path: &QString);

        /// Start opening `path` as a raster import on a worker.
        fn open_image_start(view: Pin<&mut PictureView>, path: &QString);

        /// Start opening `path` as a smart-object document on a worker.
        fn open_smart_object_start(view: Pin<&mut PictureView>, path: &QString);

        /// Start probing and decoding `path` as a Photoshop PDF on a worker.
        /// `open_detected` fires when it carries editing data; `open_finished`
        /// reports `"ok"`, `"none"`, `"password"`, or `"wrongpassword"`.
        fn open_pdf_start(view: Pin<&mut PictureView>, path: &QString, password: &QString);

        /// Start rendering PDF page `page` (1-based) cropped to `area` on a
        /// worker, then install it as an untitled document.
        fn open_pdf_page_start(
            view: Pin<&mut PictureView>,
            path: &QString,
            password: &QString,
            page: i32,
            area: &QString,
            target: &ImportTarget,
        );

        /// Start rendering embedded PDF image `index` on a worker.
        fn open_pdf_image_start(
            view: Pin<&mut PictureView>,
            path: &QString,
            index: i32,
            target: &ImportTarget,
        );

        /// Start rasterizing an EPS's bounding box on a worker.
        fn open_eps_start(view: Pin<&mut PictureView>, path: &QString, target: &ImportTarget);

        /// Mark a started open cancelled; the queued result is dropped.
        fn open_cancel(view: Pin<&mut PictureView>);

        /// Hold a started open's completion for `ms` milliseconds (test seam).
        fn open_set_delay(view: Pin<&mut PictureView>, ms: u64);

        /// `File > Revert`: reload `path` in place, preserving history and
        /// named snapshots and appending one `Revert` state; `false` without
        /// mutating on any decode failure.
        fn revert(view: Pin<&mut PictureView>, path: &QString) -> bool;
    }
}
