//! Filter work off the UI thread (#152). On a document the size of the
//! 16507×16196 world map a commit takes seconds and an artistic filter's exact
//! preview longer; running them here keeps the event loop free, so the dialog
//! and the busy indicator stay live. Free functions (their own bridge, so the
//! `PictureView` declaration list does not grow).
//!
//! A worker never touches the live document: it filters a clone (planes are
//! shared copy-on-write, so the clone is cheap and the first write copies only
//! what the filter changes). An exact preview hands back the filtered layer
//! and its reduced picture; a commit also composites the changed region and
//! captures history, which it holds until the UI thread swaps the results in.

mod imp;

use super::super::qobject::PictureView;
pub(in crate::cxxqt_object) use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("cxx-qt-lib/qlist.h");
        type QList_f64 = cxx_qt_lib::QList<f64>;

        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Start committing `kind` with `params` to the active layer on a worker
        /// thread, recorded as the history state `label`. An open preview is put
        /// back first; a finished exact preview of the same filter is reused.
        /// A layer-mask target is filtered at once instead. False when refused
        /// (see `filter_last_error`).
        fn filter_commit_start(
            view: Pin<&mut PictureView>,
            kind: &QString,
            params: &QList_f64,
            label: &QString,
        ) -> bool;

        /// As [`filter_commit_start`] for a Filter Gallery stack: `kinds` in
        /// order, their parameter slots concatenated in `params`.
        fn filter_stack_commit_start(
            view: Pin<&mut PictureView>,
            kinds: &QStringList,
            params: &QList_f64,
            label: &QString,
        ) -> bool;

        /// Whether a started commit is still running.
        fn filter_commit_running(view: &PictureView) -> bool;

        /// Wait for the started commit and swap its document, history and
        /// refreshed region in. True when it applied (or there was nothing
        /// running); false when the worker refused it.
        fn filter_commit_finish(view: Pin<&mut PictureView>) -> bool;

        /// Ask for the exact preview of `kind`/`params`: the full-resolution
        /// layer filtered on a worker, shown reduced like
        /// `filter_canvas_proxy` over the document rect `(x, y, w, h)` at
        /// `scale`. A newer request supersedes an older one. False for a
        /// layer-mask target or an unknown filter.
        #[allow(clippy::too_many_arguments)]
        fn filter_exact_preview_start(
            view: Pin<&mut PictureView>,
            kind: &QString,
            params: &QList_f64,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            scale: f64,
        ) -> bool;

        /// The latest exact preview's picture the first time it is ready, as
        /// a premultiplied display image; null otherwise.
        fn filter_exact_preview_poll(view: Pin<&mut PictureView>) -> QImage;

        /// Whether the latest exact preview is still being computed.
        fn filter_exact_preview_busy(view: Pin<&mut PictureView>) -> bool;

        /// Drop the exact preview; a running worker finishes unobserved.
        fn filter_exact_preview_cancel(view: Pin<&mut PictureView>);

        /// Hold a finished exact preview for `ms` milliseconds (test seam).
        fn filter_exact_preview_set_delay(view: Pin<&mut PictureView>, ms: u64);

        /// The document rect `(x, y, w, h)` with `kind`/`params` applied to the
        /// active layer, at full resolution, without changing the document:
        /// the dialog pane's exact section while the canvas shows a reduced
        /// copy. Null when refused or for a layer-mask target.
        #[allow(clippy::too_many_arguments)]
        fn filter_section_image(
            view: Pin<&mut PictureView>,
            kind: &QString,
            params: &QList_f64,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
        ) -> QImage;
    }
}
