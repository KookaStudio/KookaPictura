//! Edit > Purge ▸ Undo / Histories / All: drop the undo history and the named
//! restore points. Free functions over a [`PictureView`] (their own bridge, so
//! the `PictureView` declaration list does not grow). `Purge ▸ Clipboard` lives
//! in the clipboard bridge (`clipboard_purge`). Ported from photorust's
//! `Edit > Purge` handlers.
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

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
        /// `Purge ▸ All`: drop every undo state but the current one and every
        /// named restore point; emit `changed` so the History panel refreshes.
        /// Returns false without a document.
        fn purge_all(view: Pin<&mut PictureView>) -> bool;

        /// `Purge ▸ Undo`: drop the undo history but keep the named restore
        /// points. Returns false without a document.
        fn purge_history(view: Pin<&mut PictureView>) -> bool;

        /// `Purge ▸ Histories`: drop the named restore points only, keeping the
        /// undo history. Returns false without a document.
        fn purge_named_history(view: Pin<&mut PictureView>) -> bool;

        /// The label of the current history state, or empty without a document.
        fn current_history_label(view: &PictureView) -> QString;
    }
}

fn purge_all(mut view: Pin<&mut PictureView>) -> bool {
    if view.rust().session.doc.is_none() {
        return false;
    }
    view.as_mut().rust_mut().session.history.purge();
    view.as_mut().changed();
    true
}

fn purge_history(mut view: Pin<&mut PictureView>) -> bool {
    if view.rust().session.doc.is_none() {
        return false;
    }
    // Drop the undo history but keep the named restore points: purge() clears
    // both, so re-add them is wasteful — instead clear states only.
    view.as_mut().rust_mut().session.history.purge_states();
    view.as_mut().changed();
    true
}

fn purge_named_history(mut view: Pin<&mut PictureView>) -> bool {
    if view.rust().session.doc.is_none() {
        return false;
    }
    view.as_mut().rust_mut().session.history.purge_snapshots();
    view.as_mut().changed();
    true
}

fn current_history_label(view: &PictureView) -> QString {
    let history = &view.rust().session.history;
    QString::from(history.label(history.index()))
}
