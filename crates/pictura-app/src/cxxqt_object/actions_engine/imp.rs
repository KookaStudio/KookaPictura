use cxx_qt_lib::QString;
use pictura_script::{ActionLibrary, DocumentFacts, Platform, PlaybackSpeed, Player, Recorder};

/// Backing state for [`ffi::ActionsEngine`].
pub struct ActionsEngineRust {
    pub(super) library: ActionLibrary,
    pub(super) recorder: Recorder,
    pub(super) player: Option<Player>,
    pub(super) allow_tool_recording: bool,
    pub(super) facts: DocumentFacts,
    pub(super) speed: PlaybackSpeed,
    pub(super) pause_seconds: f64,
    pub(super) platform: Platform,
    pub(super) last_error: String,
}

impl Default for ActionsEngineRust {
    fn default() -> Self {
        Self {
            library: ActionLibrary::with_default(),
            recorder: Recorder::new(),
            player: None,
            allow_tool_recording: false,
            facts: DocumentFacts::default(),
            speed: PlaybackSpeed::Accelerated,
            pause_seconds: 0.0,
            platform: Platform::Linux,
            last_error: String::new(),
        }
    }
}

impl ActionsEngineRust {
    pub(in crate::cxxqt_object) fn end_sessions(&mut self) {
        self.recorder.stop();
        if let Some(player) = self.player.as_mut() {
            player.cancel();
        }
        self.player = None;
    }
}

pub(super) fn index(value: i32) -> Option<usize> {
    usize::try_from(value).ok()
}

pub(in crate::cxxqt_object) fn q(value: &str) -> QString {
    QString::from(value)
}
