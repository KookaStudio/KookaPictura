//! App-native recording for completed paint-tool strokes.
//!
//! A successful `begin_*` opens a capture when the installed Actions recorder
//! reports Allow Tool Recording on; every `paint_dab` sample is appended; a
//! committed `end_paint` hands one [`ToolStrokeStep`] to the recorder, and a
//! cancel drops it. The step is the engine's app-native tool payload, so it
//! carries a format version and is bounded before it can reach an action.
//!
//! The C++ Actions bridge installs the recorder and replays Brush/Pencil
//! payloads through [`replay_tool_stroke_json`]. Native tool-specific and mask
//! strokes are refused until their replay paths land.
#![allow(dead_code)]

mod imp;

pub(crate) use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("panels/actions_tool_recording_bridge.h");
        #[namespace = "pictura"]
        fn active_tool_recording() -> bool;
        #[namespace = "pictura"]
        fn record_active_tool_stroke(payload: &QString) -> bool;
        /// Report a refused stroke to the Actions bridge's error hook.
        #[namespace = "pictura"]
        fn tool_stroke_refused(message: &QString);
    }
}

impl std::error::Error for ToolStrokeError {}
