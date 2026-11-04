//! The Qt half of the document session: present what the session reports.
//!
//! The session itself is plain Rust (`super::session`); this adapter turns its
//! presentation into the signals the shell listens to.

use super::super::qobject;
use core::pin::Pin;
use cxx_qt::CxxQtType;

impl qobject::PictureView {
    /// Cancel whatever live gesture is active and present the result. Only a
    /// cancelled session reports `session_cancelled`, whose handler refreshes
    /// every panel at once; a tool switch with nothing live stays cheap (#1145).
    pub fn cancel_active_session(mut self: Pin<&mut Self>) -> bool {
        let had_session = self.rust().has_active_session();
        let presentation = self.as_mut().rust_mut().cancel_active_sessions();
        let changed = presentation.has_display_change();
        self.as_mut().present(presentation);
        if had_session {
            self.as_mut().session_cancelled();
        }
        changed
    }
}
