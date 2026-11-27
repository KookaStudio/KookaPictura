use super::PictureView;
use cxx_qt::CxxQtType;
use std::collections::HashSet;
use std::sync::{Condvar, Mutex, OnceLock};

#[derive(Default)]
struct Gate {
    held: bool,
    started: HashSet<u64>,
    released: HashSet<u64>,
    finished: HashSet<u64>,
}
fn gate() -> &'static (Mutex<Gate>, Condvar) {
    static GATE: OnceLock<(Mutex<Gate>, Condvar)> = OnceLock::new();
    GATE.get_or_init(|| (Mutex::new(Gate::default()), Condvar::new()))
}
pub(super) fn started(id: u64) {
    let (lock, wake) = gate();
    if let Ok(mut g) = lock.lock() {
        g.started.insert(id);
        wake.notify_all();
    }
}
pub(super) fn wait(id: u64) {
    let (lock, wake) = gate();
    if let Ok(mut g) = lock.lock() {
        while g.held && !g.released.contains(&id) {
            match wake.wait(g) {
                Ok(next) => g = next,
                Err(_) => break,
            }
        }
    }
}
pub(super) fn finished(id: u64) {
    let (lock, wake) = gate();
    if let Ok(mut g) = lock.lock() {
        g.finished.insert(id);
        wake.notify_all();
    }
}
pub(in crate::cxxqt_object::impl_transform) fn content_aware_test_hold(
    _view: &PictureView,
    hold: bool,
) {
    let (lock, wake) = gate();
    if let Ok(mut g) = lock.lock() {
        if hold {
            *g = Gate {
                held: true,
                ..Gate::default()
            };
        } else {
            g.held = false;
            wake.notify_all();
        }
    }
}
pub(in crate::cxxqt_object::impl_transform) fn content_aware_test_release(
    _view: &PictureView,
    id: u64,
) {
    let (lock, wake) = gate();
    if let Ok(mut g) = lock.lock() {
        g.released.insert(id);
        wake.notify_all();
    }
}
pub(in crate::cxxqt_object::impl_transform) fn content_aware_test_started(
    _view: &PictureView,
    id: u64,
) -> bool {
    gate().0.lock().is_ok_and(|g| g.started.contains(&id))
}
pub(in crate::cxxqt_object::impl_transform) fn content_aware_test_finished(
    _view: &PictureView,
    id: u64,
) -> bool {
    gate().0.lock().is_ok_and(|g| g.finished.contains(&id))
}
pub(in crate::cxxqt_object::impl_transform) fn content_aware_test_generation(
    view: &PictureView,
) -> u64 {
    view.rust()
        .session
        .live
        .transform()
        .and_then(|s| s.content_aware.as_ref())
        .map_or(0, |s| s.generation)
}
