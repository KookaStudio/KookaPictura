use super::*;

use crate::cxxqt_object::edit::Edit;
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

pub(super) fn apply_history_states(mut view: Pin<&mut PictureView>) -> bool {
    let trimmed = view.as_mut().rust_mut().session.history.apply_state_limit();
    if trimmed {
        view.as_mut().changed();
    }
    trimmed
}

pub(super) fn set_history_options(
    mut view: Pin<&mut PictureView>,
    allow_non_linear: bool,
    visibility_undoable: bool,
    auto_snapshot_on_save: bool,
) {
    let mut rust = view.as_mut().rust_mut();
    rust.history_allow_non_linear = allow_non_linear;
    rust.history_visibility_undoable = visibility_undoable;
    rust.history_auto_snapshot_on_save = auto_snapshot_on_save;
    rust.session.history.set_non_linear(allow_non_linear);
}

pub(super) fn history_new_document(
    source: &PictureView,
    mut target: Pin<&mut PictureView>,
    snapshot: bool,
    index: i32,
) -> bool {
    if index < 0 {
        return false;
    }
    let index = index as usize;
    let state = {
        let rust = source.rust();
        if rust.session.doc.is_none() {
            return false;
        }
        if snapshot {
            rust.session.history.snapshot(index)
        } else {
            rust.session.history.state_snapshot(index)
        }
    };
    let Some(state) = state else {
        return false;
    };
    {
        let mut rust = target.as_mut().rust_mut();
        rust.replace_document(
            Some(state.doc),
            Default::default(),
            Default::default(),
            None,
        );
        rust.select_only_layer("0");
    }
    target.as_mut().edit("Duplicate State", |_| Edit::Full);
    target.as_mut().session_cancelled();
    true
}

pub(super) fn current_history_label(view: &PictureView) -> QString {
    let history = &view.rust().session.history;
    QString::from(history.label(history.index()))
}

pub(super) fn history_snapshot_count(view: &PictureView) -> i32 {
    view.rust().session.history.snapshot_count() as i32
}

pub(super) fn history_snapshot_label(view: &PictureView, i: i32) -> QString {
    if i < 0 {
        return QString::default();
    }
    QString::from(view.rust().session.history.snapshot_label(i as usize))
}
