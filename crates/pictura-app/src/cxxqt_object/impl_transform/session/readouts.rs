//! The active transform session's readouts (Options bar and Info panel).
//! Split from `session.rs` for the file-size cap.

use super::super::super::qobject;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

impl qobject::PictureView {
    /// Whether a Free Transform session is active.
    pub fn transform_session_active(&self) -> bool {
        self.rust().session.live.transform().is_some()
    }

    /// The handle the live transform drag grabbed, or -1 when no drag is live.
    pub fn transform_drag_handle(&self) -> i32 {
        self.rust()
            .session
            .live
            .transform()
            .filter(|session| session.dragging)
            .map_or(-1, |session| session.handle)
    }
    pub fn transform_session_path(&self) -> QString {
        self.rust()
            .session
            .live
            .transform()
            .map(|session| QString::from(session.path.as_str()))
            .unwrap_or_default()
    }

    pub fn transform_scale_x(&self) -> f64 {
        self.rust()
            .session
            .live
            .transform()
            .map_or(1.0, |session| session.scale_x)
    }

    pub fn transform_scale_y(&self) -> f64 {
        self.rust()
            .session
            .live
            .transform()
            .map_or(1.0, |session| session.scale_y)
    }

    pub fn transform_angle(&self) -> f64 {
        self.rust()
            .session
            .live
            .transform()
            .map_or(0.0, |session| session.angle)
    }

    pub fn transform_dx(&self) -> f64 {
        self.rust()
            .session
            .live
            .transform()
            .map_or(0.0, |session| session.dx)
    }

    pub fn transform_dy(&self) -> f64 {
        self.rust()
            .session
            .live
            .transform()
            .map_or(0.0, |session| session.dy)
    }
}
