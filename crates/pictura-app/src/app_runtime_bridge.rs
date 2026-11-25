//! Process-wide GPU selection and logging, including startup before QApplication.

mod gpu_probe;
mod logging;

use crate::cxxqt_object::qobject::PictureView;
use gpu_probe::*;
use logging::*;

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
    struct GpuProbeReply {
        code: i32,
        detail: String,
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Parent side: spawn `child_command --gpu-probe <backend>`, wait up
        /// to 10 s, and return the outcome code (0 ok, 1 no adapter/clean
        /// failure, 2 validation failure, crash, or timeout) with its
        /// diagnostic detail.
        fn gpu_probe(kind: i32, child_command: &str) -> GpuProbeReply;
        /// Child side: pin the backend, create the device, and run the CPU
        /// oracle validation. 0 on success, 1 when no adapter exists, 2 when a
        /// device exists but fails validation. Prints the detail line.
        fn gpu_probe_child(kind: i32) -> i32;
        /// Pin the process to one backend. False when a different pin already
        /// resolved or the code is unknown.
        fn gpu_set_backend(kind: i32) -> bool;
        /// Mark the GPU path blocked (chain exhausted or validation failed).
        fn gpu_set_safe_mode(active: bool);
        /// The ordered candidate list for this launch, after the environment
        /// override and the compiled-backend intersection.
        fn gpu_candidates() -> Vec<i32>;
        /// The five-state status label for the status bar and About.
        fn gpu_active_backend_label(gpu_compute: bool) -> String;

        /// Name the global rayon pool's workers; call once before any parallel work.
        fn worker_threads_init();

        fn diagnostics_start(path: &QString, stall_for_test: bool);
        fn diagnostics_stop();
        fn diagnostics_qt(level: i32, category: &str, fatal: bool);
        fn diagnostics_outcome(event: &str, success: bool);
        fn diagnostics_dependency_for_test(message: &str);
        fn history_log_configure(enabled: bool, destination: i32, verbosity: i32, path: &QString);
        fn history_log_start();
        fn history_log_document(view: Pin<&mut PictureView>, name: &QString, opened: bool);
        fn history_log_close(name: &QString);
        fn history_log_quit();
        fn history_log_shutdown();
        fn history_log_take_notice() -> QString;
        fn history_log_stall_for_test(stalled: bool);
    }
}
