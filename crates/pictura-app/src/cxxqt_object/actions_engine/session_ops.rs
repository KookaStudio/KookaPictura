//! Recording, playback, and palette persistence operations.

use super::ffi::ScriptPlaybackRow;
use super::{index, q};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_script::{
    command::{CommandId, CommandParams},
    ActionError, DocumentFacts, PlaybackCommand, PlaybackOptions, PlaybackSelection, PlaybackSpeed,
    Player, ResetMode, Selection, ToolStroke,
};

impl super::ffi::ActionsEngine {
    pub fn set_allow_tool_recording(mut self: Pin<&mut Self>, allow: bool) {
        self.as_mut().rust_mut().get_mut().allow_tool_recording = allow;
        self.as_mut().changed();
    }

    pub fn move_step_to(
        mut self: Pin<&mut Self>,
        from_set: i32,
        from_action: i32,
        from_step: i32,
        to_set: i32,
        to_action: i32,
        to_index: i32,
    ) -> bool {
        let changed = {
            let this = self.as_mut().rust_mut().get_mut();
            let (Some(from_set), Some(from_action), Some(from_step), Some(to_set), Some(to_action)) = (
                index(from_set),
                index(from_action),
                index(from_step),
                index(to_set),
                index(to_action),
            ) else {
                return false;
            };
            let to_index = usize::try_from(to_index).unwrap_or_else(|_| {
                this.library
                    .action(to_set, to_action)
                    .map_or(0, |action| action.steps.len())
            });
            match this.library.move_step_to(
                from_set,
                from_action,
                from_step,
                to_set,
                to_action,
                to_index,
            ) {
                Ok(changed) => changed,
                Err(err) => {
                    this.last_error = err.to_string();
                    false
                }
            }
        };
        if changed {
            self.as_mut().changed();
        }
        changed
    }

    pub fn record_start(
        mut self: Pin<&mut Self>,
        scope: i32,
        set: i32,
        action: i32,
        step: i32,
    ) -> bool {
        let started = {
            let this = self.as_mut().rust_mut().get_mut();
            if this
                .player
                .as_ref()
                .is_some_and(|player| !player.is_finished())
            {
                this.last_error = ActionError::PlaybackActive.to_string();
                return false;
            }
            let selection = match (scope, index(set), index(action), index(step)) {
                (1, Some(set), _, _) => Selection::Set { set },
                (2, Some(set), Some(action), _) => Selection::Action { set, action },
                (3, Some(set), Some(action), Some(step)) => Selection::Step { set, action, step },
                _ => Selection::None,
            };
            match this.recorder.start(&this.library, selection) {
                Ok(()) => {
                    this.last_error.clear();
                    true
                }
                Err(err) => {
                    this.last_error = err.to_string();
                    false
                }
            }
        };
        if started {
            self.as_mut().changed();
        }
        started
    }

    pub fn record_stop(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().get_mut().recorder.stop();
        self.as_mut().changed();
    }

    pub fn record_command(mut self: Pin<&mut Self>, id: &QString, params_json: &QString) -> bool {
        let recorded = {
            let this = self.as_mut().rust_mut().get_mut();
            let params = match CommandParams::from_json(&params_json.to_string()) {
                Ok(params) => params,
                Err(err) => {
                    this.last_error = err.to_string();
                    return false;
                }
            };
            match this
                .recorder
                .record(&mut this.library, &CommandId::new(id.to_string()), &params)
            {
                Ok(recorded) => {
                    if recorded {
                        this.last_error.clear();
                    }
                    recorded
                }
                Err(err) => {
                    this.last_error = err.to_string();
                    false
                }
            }
        };
        if recorded {
            self.as_mut().changed();
        }
        recorded
    }

    pub fn record_tool_stroke(mut self: Pin<&mut Self>, payload_json: &QString) -> bool {
        let recorded = {
            let this = self.as_mut().rust_mut().get_mut();
            let stroke = match ToolStroke::from_json(&payload_json.to_string()) {
                Ok(stroke) => stroke,
                Err(err) => {
                    this.last_error = err;
                    return false;
                }
            };
            match this.recorder.record_tool_stroke(
                &mut this.library,
                this.allow_tool_recording,
                stroke,
            ) {
                Ok(recorded) => recorded,
                Err(err) => {
                    this.last_error = err.to_string();
                    false
                }
            }
        };
        if recorded {
            self.as_mut().changed();
        }
        recorded
    }

    pub fn play_start(
        mut self: Pin<&mut Self>,
        scope: i32,
        set: i32,
        action: i32,
        step: i32,
    ) -> bool {
        let started = {
            let this = self.as_mut().rust_mut().get_mut();
            if this.recorder.is_recording() {
                this.last_error = ActionError::RecordingActive.to_string();
                return false;
            }
            if this
                .player
                .as_ref()
                .is_some_and(|player| !player.is_finished())
            {
                this.last_error = ActionError::PlaybackActive.to_string();
                return false;
            }
            let selection = match (scope, index(set), index(action), index(step)) {
                (1, Some(set), _, _) => PlaybackSelection::Set { set },
                (2, Some(set), Some(action), _) => PlaybackSelection::Action { set, action },
                (3, Some(set), Some(action), Some(step)) => {
                    PlaybackSelection::FromStep { set, action, step }
                }
                (4, Some(set), Some(action), Some(step)) => {
                    PlaybackSelection::SingleStep { set, action, step }
                }
                _ => {
                    this.last_error = ActionError::NoTarget.to_string();
                    return false;
                }
            };
            let facts = this.facts.clone();
            let options = PlaybackOptions { speed: this.speed };
            match Player::start(&this.library, selection, &facts, options) {
                Ok(player) => {
                    this.player = Some(player);
                    this.last_error.clear();
                    true
                }
                Err(err) => {
                    this.last_error = err.to_string();
                    this.player = None;
                    false
                }
            }
        };
        if started {
            self.as_mut().changed();
        }
        started
    }

    pub fn play_next(mut self: Pin<&mut Self>) -> ScriptPlaybackRow {
        let (row, notify) = {
            let this = self.as_mut().rust_mut().get_mut();
            let Some(player) = this.player.as_mut() else {
                return playback_row("finished");
            };
            match player.next() {
                Ok(PlaybackCommand::Execute {
                    set,
                    action,
                    step,
                    command,
                    params,
                    modal,
                }) => (
                    ScriptPlaybackRow {
                        kind: q("execute"),
                        set: set as i32,
                        action: action as i32,
                        step: step as i32,
                        command: q(command.as_str()),
                        params_json: q(&params.to_json()),
                        modal,
                        message: q(""),
                        allow_continue: false,
                        path: q(""),
                        error: q(""),
                    },
                    true,
                ),
                Ok(PlaybackCommand::ToolStroke {
                    set,
                    action,
                    step,
                    stroke,
                }) => (
                    ScriptPlaybackRow {
                        kind: q("tool"),
                        set: set as i32,
                        action: action as i32,
                        step: step as i32,
                        command: q("pictura.toolStroke"),
                        params_json: q(&stroke.to_json()),
                        modal: false,
                        message: q(""),
                        allow_continue: false,
                        path: q(""),
                        error: q(""),
                    },
                    true,
                ),
                Ok(PlaybackCommand::Stop {
                    set,
                    action,
                    step,
                    message,
                    allow_continue,
                }) => (
                    ScriptPlaybackRow {
                        kind: q("stop"),
                        set: set as i32,
                        action: action as i32,
                        step: step as i32,
                        command: q(""),
                        params_json: q(""),
                        modal: false,
                        message: q(&message),
                        allow_continue,
                        path: q(""),
                        error: q(""),
                    },
                    true,
                ),
                Ok(PlaybackCommand::InsertPath {
                    set,
                    action,
                    step,
                    path,
                }) => (
                    ScriptPlaybackRow {
                        kind: q("path"),
                        set: set as i32,
                        action: action as i32,
                        step: step as i32,
                        command: q(""),
                        params_json: q(""),
                        modal: false,
                        message: q(""),
                        allow_continue: false,
                        path: q(&path),
                        error: q(""),
                    },
                    true,
                ),
                Ok(PlaybackCommand::Waiting) => (playback_row("waiting"), false),
                Ok(PlaybackCommand::Finished) => {
                    this.player = None;
                    (playback_row("finished"), true)
                }
                Err(err) => {
                    this.last_error = err.to_string();
                    this.player = None;
                    let mut row = playback_row("error");
                    row.error = q(&err.to_string());
                    (row, true)
                }
            }
        };
        if notify {
            self.as_mut().changed();
        }
        row
    }

    pub fn play_continue(mut self: Pin<&mut Self>) {
        if let Some(player) = self.as_mut().rust_mut().get_mut().player.as_mut() {
            player.continue_stop();
        }
    }

    pub fn play_cancel(mut self: Pin<&mut Self>) {
        {
            let this = self.as_mut().rust_mut().get_mut();
            if let Some(player) = this.player.as_mut() {
                player.cancel();
            }
            this.player = None;
        }
        self.as_mut().changed();
    }

    pub fn set_document_facts(
        mut self: Pin<&mut Self>,
        present: bool,
        square: bool,
        mode: &QString,
        has_selection: bool,
    ) {
        let this = self.as_mut().rust_mut().get_mut();
        this.facts = DocumentFacts {
            present,
            square,
            mode: mode.to_string(),
            has_selection,
        };
        let facts = this.facts.clone();
        if let Some(player) = this.player.as_mut() {
            player.set_facts(facts);
        }
    }

    pub fn conditional_branch(&self, set: i32, action: i32, step: i32) -> QString {
        let (Some(set), Some(action), Some(step)) = (index(set), index(action), index(step)) else {
            return q("");
        };
        let Some(branch) = self
            .library
            .conditional_branch(set, action, step, &self.facts)
        else {
            return q("");
        };
        self.library
            .set(set)
            .and_then(|target| target.action_index(&branch))
            .map_or(q(""), |index| q(&format!("s{set}a{index}")))
    }

    pub fn set_playback_options(mut self: Pin<&mut Self>, speed: i32, pause_seconds: f64) {
        let this = self.as_mut().rust_mut().get_mut();
        this.speed = match speed {
            1 => PlaybackSpeed::StepByStep,
            2 => PlaybackSpeed::PauseSeconds(pause_seconds),
            _ => PlaybackSpeed::Accelerated,
        };
        this.pause_seconds = pause_seconds;
        self.as_mut().changed();
    }

    pub fn playback_speed(&self) -> i32 {
        match self.speed {
            PlaybackSpeed::Accelerated => 0,
            PlaybackSpeed::StepByStep => 1,
            PlaybackSpeed::PauseSeconds(_) => 2,
        }
    }

    pub fn playback_pause_seconds(&self) -> f64 {
        self.pause_seconds
    }

    pub fn palette_bytes(&self) -> Vec<u8> {
        self.library.save_bytes()
    }

    pub fn set_bytes(&self, set: i32) -> Vec<u8> {
        index(set)
            .and_then(|set| self.library.set_bytes(set))
            .unwrap_or_default()
    }

    pub fn load_palette(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let loaded = {
            let this = self.as_mut().rust_mut().get_mut();
            this.end_sessions();
            this.last_error.clear();
            let source = path.to_string();
            let bytes = match std::fs::read(source.as_str()) {
                Ok(bytes) => bytes,
                Err(err) => {
                    this.last_error = err.to_string();
                    return false;
                }
            };
            let (count, error) = this.library.load_bytes_partial(&source, &bytes);
            if let Some(err) = error {
                this.last_error = err.to_string();
            }
            count > 0 || this.last_error.is_empty()
        };
        if loaded {
            self.as_mut().changed();
        }
        loaded
    }

    pub fn save_palette(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let this = self.as_mut().rust_mut().get_mut();
        match this.library.save_path(path.to_string()) {
            Ok(()) => true,
            Err(err) => {
                this.last_error = err.to_string();
                false
            }
        }
    }

    pub fn load_palette_bytes(
        mut self: Pin<&mut Self>,
        bytes: &[u8],
        source: &QString,
        replace: bool,
    ) -> bool {
        let loaded = {
            let this = self.as_mut().rust_mut().get_mut();
            this.end_sessions();
            this.last_error.clear();
            let source = source.to_string();
            if replace {
                let (loaded, error) = this.library.load_bytes_partial(&source, bytes);
                if let Some(err) = error {
                    this.last_error = err.to_string();
                }
                loaded > 0 || this.last_error.is_empty()
            } else {
                match this.library.append_bytes(&source, bytes) {
                    Ok(()) => true,
                    Err(err) => {
                        this.last_error = err.to_string();
                        false
                    }
                }
            }
        };
        if loaded {
            self.as_mut().changed();
        }
        loaded
    }

    pub fn save_set(mut self: Pin<&mut Self>, path: &QString, set: i32) -> bool {
        let this = self.as_mut().rust_mut().get_mut();
        let Some(bytes) = index(set).and_then(|set| this.library.set_bytes(set)) else {
            return false;
        };
        match std::fs::write(path.to_string().as_str(), bytes) {
            Ok(()) => true,
            Err(err) => {
                this.last_error = err.to_string();
                false
            }
        }
    }

    pub fn reset_actions(mut self: Pin<&mut Self>, append: bool) {
        let this = self.as_mut().rust_mut().get_mut();
        this.end_sessions();
        this.library.reset(if append {
            ResetMode::Append
        } else {
            ResetMode::Replace
        });
        self.as_mut().changed();
    }

    pub fn clear_actions(mut self: Pin<&mut Self>) {
        let this = self.as_mut().rust_mut().get_mut();
        this.end_sessions();
        this.library.clear();
        self.as_mut().changed();
    }
}

fn playback_row(kind: &str) -> ScriptPlaybackRow {
    ScriptPlaybackRow {
        kind: q(kind),
        set: -1,
        action: -1,
        step: -1,
        command: q(""),
        params_json: q(""),
        modal: false,
        message: q(""),
        allow_continue: false,
        path: q(""),
        error: q(""),
    }
}
