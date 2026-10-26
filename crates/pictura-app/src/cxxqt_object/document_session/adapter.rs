//! The Qt half of the document session: present what the session reports.
//!
//! The session itself is plain Rust (`super::session`); this adapter turns its
//! report into the signals the shell listens to.

use super::super::qobject;
use super::session::SessionChange;
use core::pin::Pin;
use cxx_qt::CxxQtType;

impl qobject::PictureView {
    /// Apply a session report to the QObject: blit its regions, then emit one
    /// notification or rebuild according to the effect.
    pub(in crate::cxxqt_object) fn apply_session_change(
        mut self: Pin<&mut Self>,
        change: SessionChange,
    ) {
        for region in change.regions {
            match region.srgb {
                Some(srgb) => {
                    let rect = region.rect;
                    self.as_mut()
                        .rust_mut()
                        .apply_refreshed_region(rect.left, rect.top, rect, srgb);
                    self.as_mut()
                        .region_damaged(rect.left, rect.top, rect.width(), rect.height());
                }
                None => self.as_mut().refresh_region(region.rect),
            }
        }
        match change.effect {
            super::super::edit::Effect::Nothing => {}
            super::super::edit::Effect::Notify => {
                self.as_mut().changed();
            }
            super::super::edit::Effect::Blit { notify } => {
                if notify {
                    self.as_mut().changed();
                }
            }
            super::super::edit::Effect::Rebuild => {
                self.as_mut().recomposite();
            }
        }
    }

    /// Cancel whatever live gesture is active and present the result.
    pub fn cancel_active_session(mut self: Pin<&mut Self>) -> bool {
        let change = self.as_mut().rust_mut().cancel_active_sessions();
        let changed = change.changed_display();
        self.as_mut().apply_session_change(change);
        self.as_mut().session_cancelled();
        changed
    }
}
