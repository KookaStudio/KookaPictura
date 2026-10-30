use super::*;

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

pub(super) fn purge_all(mut view: Pin<&mut PictureView>) -> bool {
    if view.rust().session.doc.is_none() {
        return false;
    }
    view.as_mut().rust_mut().session.history.purge();
    view.as_mut().changed();
    true
}

pub(super) fn purge_history(mut view: Pin<&mut PictureView>) -> bool {
    if view.rust().session.doc.is_none() {
        return false;
    }
    // Drop the undo history but keep the named restore points: purge() clears
    // both, so re-add them is wasteful — instead clear states only.
    view.as_mut().rust_mut().session.history.purge_states();
    view.as_mut().changed();
    true
}

pub(super) fn purge_named_history(mut view: Pin<&mut PictureView>) -> bool {
    if view.rust().session.doc.is_none() {
        return false;
    }
    view.as_mut().rust_mut().session.history.purge_snapshots();
    view.as_mut().changed();
    true
}

pub(super) fn current_history_label(view: &PictureView) -> QString {
    let history = &view.rust().session.history;
    QString::from(history.label(history.index()))
}
