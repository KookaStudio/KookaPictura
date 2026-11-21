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

pub(super) mod compatibility;
mod imp;
mod raw_open;

use super::pdf_export::import::ffi::ImportTarget;
use super::qobject::PictureView;
pub use super::stack_files::load_files_into_stack;
pub use compatibility::{
    compatibility_preview_available, compatibility_reason, native_save_warning,
    save_compatibility_copy,
};
pub use imp::*;
pub(super) use raw_open::PendingRaw;
pub use raw_open::{
    open_raw_develop_start, open_raw_start, raw_as_shot_temperature, raw_as_shot_tint,
    raw_dng_default_path, raw_preview, raw_save_dng_cancel, raw_save_dng_finish,
    raw_save_dng_progress, raw_save_dng_running, raw_save_dng_start, raw_save_settings,
    raw_seed_settings, raw_sniff_path,
};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

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
        fn output_path_protected(path: &QString) -> bool;
        fn compatibility_reason(view: &PictureView) -> QString;
        fn compatibility_preview_available(view: &PictureView) -> bool;
        fn save_compatibility_copy(
            view: Pin<&mut PictureView>,
            path: &QString,
            confirmed: bool,
        ) -> QString;
        fn native_save_warning(view: &PictureView) -> QString;

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
            password: &QString,
            index: i32,
            target: &ImportTarget,
        );

        /// Start rasterizing an EPS's bounding box on a worker.
        fn open_eps_start(view: Pin<&mut PictureView>, path: &QString, target: &ImportTarget);

        // Camera raw opens (#400), `raw_open.rs`.
        /// Whether `path` holds a camera raw container (DNG or CR2), from its
        /// signature; the extension is not consulted.
        fn raw_sniff_path(path: &QString) -> bool;

        /// Start decoding the raw file `path` on a worker, seeding from its
        /// settings record with `store` (0 database, 1 sidecar) read first.
        /// `open_finished` reports `"raw"` with the source kept on the view,
        /// or `"error:<reason>"`.
        fn open_raw_start(view: Pin<&mut PictureView>, path: &QString, store: i32);

        /// Start developing the kept source with `settings` (Pictura Raw text)
        /// on a worker; `open_finished` reports `"ok"` with the document
        /// installed, or `"error:<reason>"`.
        fn open_raw_develop_start(view: Pin<&mut PictureView>, settings: &QString);

        /// The kept source's seed settings as Pictura Raw text; empty without
        /// one.
        fn raw_seed_settings(view: &PictureView) -> QString;

        /// Save `settings` (Pictura Raw text) as the kept source's settings
        /// record in `store` (0 database, 1 sidecar, #401): `"sidecar"`,
        /// `"database"`, `"fallback:<reason>"` (the sidecar could not be
        /// written; the database holds it), or `"error:<reason>"`.
        fn raw_save_settings(view: &PictureView, settings: &QString, store: i32) -> QString;

        /// The kept source's as-shot temperature (kelvin) and tint, rounded;
        /// 0 without one.
        fn raw_as_shot_temperature(view: &PictureView) -> i32;
        fn raw_as_shot_tint(view: &PictureView) -> i32;

        /// The kept source developed with `settings` with its long side at most
        /// `max_dim`; a null image without a source or when it cannot develop.
        fn raw_preview(view: &PictureView, settings: &QString, max_dim: i32) -> QImage;

        // Save Image (#402), `raw_open/save_dng.rs`.
        /// The kept source's default DNG path: `<stem>.dng` beside it, or
        /// `<stem>-<n>.dng` when taken; empty without a source.
        fn raw_dng_default_path(view: &PictureView) -> QString;

        /// Start writing the kept source as a DNG at `target` with
        /// `settings` (Pictura Raw text) on a worker. `compatibility` 0–4 is
        /// Camera Raw 2.4 … 6.6 and later; `preview` 0 None, 1 Medium Size,
        /// 2 Full Size. `false` without a source or while a save runs.
        fn raw_save_dng_start(
            view: Pin<&mut PictureView>,
            target: &QString,
            settings: &QString,
            compatibility: i32,
            preview: i32,
            embed_original: bool,
            linear: bool,
        ) -> bool;
        fn raw_save_dng_running(view: &PictureView) -> bool;
        /// 0–1000; -1 without a save.
        fn raw_save_dng_progress(view: &PictureView) -> i32;
        fn raw_save_dng_cancel(view: &PictureView);
        /// Wait for the save: `"ok"`, `"cancelled"`, or `"error:<reason>"`.
        fn raw_save_dng_finish(view: Pin<&mut PictureView>) -> QString;

        /// Mark a started open cancelled; the queued result is dropped.
        fn open_cancel(view: Pin<&mut PictureView>);

        /// Hold a started open's completion for `ms` milliseconds (test seam).
        fn open_set_delay(view: Pin<&mut PictureView>, ms: u64);

        /// `File > Revert`: reload `path` in place, preserving history and
        /// named snapshots and appending one `Revert` state; `false` without
        /// mutating on any decode failure.
        fn revert(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// `File > Scripts > Load Files into Stack`: build `view`'s document
        /// from `paths`, one native-size raster layer per decodable file on a
        /// canvas sized to the largest, optionally auto-aligned and combined
        /// into one embedded smart object. Returns the files that could not be
        /// loaded.
        fn load_files_into_stack(
            view: Pin<&mut PictureView>,
            paths: &QStringList,
            align: bool,
            smart_object: bool,
        ) -> QStringList;
    }
}
