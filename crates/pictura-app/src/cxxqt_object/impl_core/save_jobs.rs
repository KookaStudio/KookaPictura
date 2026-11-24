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

pub(in crate::cxxqt_object) use super::deferred_composite::{
    move_composite_pending, move_composite_set_delay, transform_commit_cancel,
    transform_commit_finish, transform_commit_progress, transform_commit_running,
    transform_pending, transform_set_delay,
};
pub(in crate::cxxqt_object) use super::document_job::{
    document_job_arm, document_job_attempt, document_job_cancel, document_job_edit_source_for_test,
    document_job_finish, document_job_finish_for, document_job_panic_for_test,
    document_job_pending, document_job_progress, document_job_running, document_job_set_delay,
    document_job_status, document_op_result,
};

#[cxx_qt::bridge]
pub mod ffi {
    #[namespace = "pictura"]
    #[derive(Clone, Copy, Debug, Default)]
    struct NativeSaveResult {
        code: u32,
        publication: i32,
        admission: i32,
        replaced: bool,
    }

    #[namespace = "pictura"]
    struct DocumentJobResult {
        code: u32,
        applied: bool,
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Start a native save on a worker with a resolved catalog id; -1 resolves
        /// from the path. False for a non-native format, no document, or a save
        /// already running.
        fn save_start(view: Pin<&mut PictureView>, path: &QString, format_id: i32) -> bool;

        /// Start a native copy on a worker without recording the path or clearing
        /// modified; -1 resolves the catalog id from the path.
        fn save_copy_start(view: Pin<&mut PictureView>, path: &QString, format_id: i32) -> bool;

        fn native_save_can_omit_merged(view: &PictureView) -> bool;

        fn native_compatibility_save_start(
            view: Pin<&mut PictureView>,
            path: &QString,
            format_id: i32,
            copy: bool,
            include: bool,
        ) -> bool;

        /// Whether a started native save is still running.
        fn save_running(view: &PictureView) -> bool;

        /// Join the started native save, record its path, and clear modified
        /// when the same document is still current. False when the worker failed
        /// or the document was replaced.
        fn save_finish(view: Pin<&mut PictureView>) -> bool;
        fn save_finish_result(view: Pin<&mut PictureView>) -> NativeSaveResult;
        fn save_last_result(view: &PictureView) -> NativeSaveResult;
        fn save_set_fault_for_test(view: Pin<&mut PictureView>, fault: i32);

        /// Worker phase percentage, or -1 when no job exists.
        fn save_progress(view: &PictureView) -> i32;

        /// Join and apply the job: 0 failed, 1 saved, 2 uncertain, 3 replaced, 4 failed after replacement.
        fn save_finish_outcome(view: Pin<&mut PictureView>) -> i32;

        fn save_set_sync_failure(view: Pin<&mut PictureView>, fail: bool);

        /// Hold a started save's completion for `ms` milliseconds (test seam).
        fn save_set_delay(view: Pin<&mut PictureView>, ms: u64);

        /// Whether a Move commit's composite is still on its worker.
        fn move_composite_pending(view: &PictureView) -> bool;

        /// Hold a started Move composite for `ms` milliseconds (test seam).
        fn move_composite_set_delay(view: Pin<&mut PictureView>, ms: u64);

        /// Whether a transform job is in flight or waiting to land.
        fn transform_pending(view: &PictureView) -> bool;

        /// Whether a large Free Transform commit's worker is still running.
        fn transform_commit_running(view: &PictureView) -> bool;

        /// The running transform commit's progress, 0 to 1000.
        fn transform_commit_progress(view: &PictureView) -> i32;

        /// Ask the running transform commit to stop; its session resumes.
        fn transform_commit_cancel(view: Pin<&mut PictureView>);

        /// Land the transform commit: true when it recorded its history state,
        /// false after a cancel or a refusal (the session is still active).
        fn transform_commit_finish(view: Pin<&mut PictureView>) -> bool;

        /// Hold a started transform engine for `ms` milliseconds (test seam).
        fn transform_set_delay(view: Pin<&mut PictureView>, ms: u64);

        /// Arm (or disarm) the view around a whole-document command the shell
        /// shows a progress window for; arming clears the last outcome.
        fn document_job_arm(view: Pin<&mut PictureView>, armed: bool);

        /// Whether a whole-document job is running or waiting to land.
        fn document_job_pending(view: &PictureView) -> bool;

        /// Whether the whole-document job's worker is still running.
        fn document_job_running(view: &PictureView) -> bool;

        /// The job's progress, 0 to 1000, or -1 for a busy bar.
        fn document_job_progress(view: &PictureView) -> i32;

        /// Ask the whole-document job to stop; nothing changes.
        fn document_job_cancel(view: Pin<&mut PictureView>);

        /// Land the job: true when the command applied (or already had).
        fn document_job_finish(view: Pin<&mut PictureView>) -> bool;
        fn document_job_finish_for(view: Pin<&mut PictureView>, attempt: u64) -> DocumentJobResult;
        fn document_job_status(view: &PictureView) -> u32;
        fn document_job_attempt(view: &PictureView) -> u64;
        fn document_job_panic_for_test(view: Pin<&mut PictureView>);
        fn document_job_edit_source_for_test(view: Pin<&mut PictureView>);

        /// The last applied command's outcome: a merged path, a replaced count.
        fn document_op_result(view: &PictureView) -> QString;

        /// Hold a started whole-document engine for `ms` milliseconds (test seam).
        fn document_job_set_delay(view: Pin<&mut PictureView>, ms: u64);
    }
}
