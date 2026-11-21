use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use std::pin::Pin;

use super::PictureView;
use crate::{diagnostic_logging, history_log};

pub(super) fn diagnostics_start(path: &QString, stall_for_test: bool) {
    diagnostic_logging::start(&path.to_string(), stall_for_test);
}

pub(super) fn diagnostics_stop() {
    diagnostic_logging::shutdown();
}

pub(super) fn diagnostics_qt(level: i32, category: &str, fatal: bool) {
    diagnostic_logging::qt(level, category, fatal);
}

pub(super) fn diagnostics_outcome(event: &str, success: bool) {
    let key = match event {
        "document.open" => "document.open",
        "gpu.initialize" => "gpu.initialize",
        "gpu.fallback" => "gpu.fallback",
        _ => return,
    };
    diagnostic_logging::outcome(key, success);
}

pub(super) fn diagnostics_dependency_for_test(message: &str) {
    log::warn!(target: "pictura_fixture_dependency", "{message}");
}

pub(super) fn history_log_configure(
    enabled: bool,
    destination: i32,
    verbosity: i32,
    path: &QString,
) {
    history_log::configure(enabled, destination, verbosity, path.to_string());
}

pub(super) fn history_log_start() {
    history_log::lifecycle("Start");
}

pub(super) fn history_log_document(mut view: Pin<&mut PictureView>, name: &QString, opened: bool) {
    view.as_mut()
        .rust_mut()
        .set_history_log_name(&name.to_string(), opened);
}

pub(super) fn history_log_close(name: &QString) {
    history_log::close(&name.to_string());
}

pub(super) fn history_log_quit() {
    history_log::lifecycle("Quit");
}

pub(super) fn history_log_shutdown() {
    history_log::shutdown();
}

pub(super) fn history_log_take_notice() -> QString {
    QString::from(history_log::take_notice())
}

pub(super) fn history_log_stall_for_test(stalled: bool) {
    history_log::stall_for_test(stalled);
}
