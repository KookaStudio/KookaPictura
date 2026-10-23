//! Playback: a step cursor over actions, speeds, modal decisions, conditional
//! branches, nested Play, cancellation, and errors. C++ owns the loop, timers,
//! dialogs, and redraws; this engine only says what to do next.

use crate::action::{DocumentFacts, StepKind, ToolStroke};
use crate::command::{self, CommandId, CommandParams, DialogMode};
use crate::error::ActionError;
use crate::library::ActionLibrary;

/// The documented nested Play/Conditional depth limit.
pub const MAX_PLAY_DEPTH: usize = 64;

/// What a Play command starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackSelection {
    Set {
        set: usize,
    },
    Action {
        set: usize,
        action: usize,
    },
    FromStep {
        set: usize,
        action: usize,
        step: usize,
    },
    SingleStep {
        set: usize,
        action: usize,
        step: usize,
    },
}

/// The three CS6 playback speeds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlaybackSpeed {
    Accelerated,
    StepByStep,
    PauseSeconds(f64),
}

/// Playback options fixed when playback starts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaybackOptions {
    pub speed: PlaybackSpeed,
}

impl Default for PlaybackOptions {
    fn default() -> Self {
        Self {
            speed: PlaybackSpeed::Accelerated,
        }
    }
}

/// One instruction the C++ loop executes or presents.
#[derive(Clone, Debug, PartialEq)]
pub enum PlaybackCommand {
    Execute {
        set: usize,
        action: usize,
        step: usize,
        command: CommandId,
        params: CommandParams,
        /// Open the command's dialog and wait for input.
        modal: bool,
    },
    ToolStroke {
        set: usize,
        action: usize,
        step: usize,
        stroke: ToolStroke,
    },
    Stop {
        set: usize,
        action: usize,
        step: usize,
        message: String,
        allow_continue: bool,
    },
    InsertPath {
        set: usize,
        action: usize,
        step: usize,
        path: String,
    },
    /// A Stop with Allow Continue is waiting for the user.
    Waiting,
    Finished,
}

#[derive(Clone, Debug)]
struct Frame {
    set: usize,
    action: usize,
    next_step: usize,
    /// Pop after one step (Ctrl/Cmd-click Play on a single command).
    single: bool,
    /// A nested Play/Conditional frame counts toward the depth limit.
    nested: bool,
}

/// A started playback run. Conditionals evaluate when the cursor reaches them,
/// against the latest facts the shell supplied.
#[derive(Clone, Debug)]
pub struct Player {
    library: ActionLibrary,
    frames: Vec<Frame>,
    waiting: bool,
    stopped: bool,
    cancelled: bool,
    failed: Option<ActionError>,
    options: PlaybackOptions,
    facts: DocumentFacts,
    last_step: Option<usize>,
    depth: usize,
}

impl Player {
    pub fn start(
        library: &ActionLibrary,
        selection: PlaybackSelection,
        facts: &DocumentFacts,
        options: PlaybackOptions,
    ) -> Result<Self, ActionError> {
        let mut frames = Vec::new();
        match selection {
            PlaybackSelection::Set { set } => {
                let target = library.set(set).ok_or(ActionError::NoTarget)?;
                for action in (0..target.actions.len()).rev() {
                    frames.push(Frame {
                        set,
                        action,
                        next_step: 0,
                        single: false,
                        nested: false,
                    });
                }
            }
            PlaybackSelection::Action { set, action } => {
                library.action(set, action).ok_or(ActionError::NoTarget)?;
                frames.push(Frame {
                    set,
                    action,
                    next_step: 0,
                    single: false,
                    nested: false,
                });
            }
            PlaybackSelection::FromStep { set, action, step } => {
                library.action(set, action).ok_or(ActionError::NoTarget)?;
                frames.push(Frame {
                    set,
                    action,
                    next_step: step,
                    single: false,
                    nested: false,
                });
            }
            PlaybackSelection::SingleStep { set, action, step } => {
                library.action(set, action).ok_or(ActionError::NoTarget)?;
                frames.push(Frame {
                    set,
                    action,
                    next_step: step,
                    single: true,
                    nested: false,
                });
            }
        }
        Ok(Self {
            library: library.clone(),
            frames,
            waiting: false,
            stopped: false,
            cancelled: false,
            failed: None,
            options,
            facts: facts.clone(),
            last_step: None,
            depth: 0,
        })
    }

    pub fn speed(&self) -> PlaybackSpeed {
        self.options.speed
    }

    /// Update the facts conditionals see from the next step onward.
    pub fn set_facts(&mut self, facts: DocumentFacts) {
        self.facts = facts;
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn is_finished(&self) -> bool {
        self.cancelled
            || self.stopped
            || self.failed.is_some()
            || (self.frames.is_empty() && !self.waiting)
    }

    pub fn remaining(&self) -> usize {
        if self.cancelled || self.stopped || self.failed.is_some() {
            return 0;
        }
        self.frames.len()
    }

    /// Cancel after the current step. Completed steps stay applied.
    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.waiting = false;
    }

    /// Resume a Stop step that offered Continue.
    pub fn continue_stop(&mut self) {
        self.waiting = false;
    }

    /// Stop after a failed step and report it.
    pub fn fail(&mut self, message: impl Into<String>) -> ActionError {
        self.cancelled = true;
        self.waiting = false;
        ActionError::Playback {
            step: self.last_step.unwrap_or(0),
            message: message.into(),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<PlaybackCommand, ActionError> {
        if let Some(error) = &self.failed {
            return Err(error.clone());
        }
        if self.cancelled || self.stopped {
            return Ok(PlaybackCommand::Finished);
        }
        if self.waiting {
            return Ok(PlaybackCommand::Waiting);
        }
        let result = self.advance();
        if let Err(error) = &result {
            self.failed = Some(error.clone());
        }
        result
    }

    fn advance(&mut self) -> Result<PlaybackCommand, ActionError> {
        loop {
            let Some((set, action_index, step_index, single)) = self
                .frames
                .last()
                .map(|frame| (frame.set, frame.action, frame.next_step, frame.single))
            else {
                return Ok(PlaybackCommand::Finished);
            };
            let Some(step) = self
                .library
                .action(set, action_index)
                .and_then(|action| action.steps.get(step_index))
                .cloned()
            else {
                self.pop_frame();
                continue;
            };
            if let Some(frame) = self.frames.last_mut() {
                if single {
                    frame.next_step = usize::MAX;
                } else {
                    frame.next_step += 1;
                }
            }
            if !step.flags.enabled {
                continue;
            }
            self.last_step = Some(step_index);
            let opens_dialog = step.opens_dialog();
            match step.kind {
                StepKind::Command { command, params } => {
                    return Ok(PlaybackCommand::Execute {
                        set,
                        action: action_index,
                        step: step_index,
                        command,
                        params,
                        modal: opens_dialog,
                    });
                }
                StepKind::MenuItem { command, params } => {
                    let modal = command::definition(command.as_str())
                        .is_none_or(|def| def.dialog != DialogMode::None);
                    return Ok(PlaybackCommand::Execute {
                        set,
                        action: action_index,
                        step: step_index,
                        command,
                        params,
                        modal,
                    });
                }
                StepKind::Stop {
                    message,
                    allow_continue,
                } => {
                    if allow_continue {
                        self.waiting = true;
                    } else {
                        self.stopped = true;
                    }
                    return Ok(PlaybackCommand::Stop {
                        set,
                        action: action_index,
                        step: step_index,
                        message,
                        allow_continue,
                    });
                }
                StepKind::InsertPath { path } => {
                    return Ok(PlaybackCommand::InsertPath {
                        set,
                        action: action_index,
                        step: step_index,
                        path,
                    });
                }
                StepKind::ToolStroke { stroke } => {
                    return Ok(PlaybackCommand::ToolStroke {
                        set,
                        action: action_index,
                        step: step_index,
                        stroke,
                    });
                }
                StepKind::Play { action } => {
                    let nested = self.resolve(&action, step_index)?;
                    self.push(nested)?;
                }
                StepKind::Conditional {
                    condition,
                    then_action,
                    else_action,
                } => {
                    if then_action.is_none() && else_action.is_none() {
                        return Err(ActionError::Playback {
                            step: step_index,
                            message: "conditional has no Then or Else action".into(),
                        });
                    }
                    if !self.facts.present {
                        return Err(ActionError::NoDocument);
                    }
                    let branch = if condition.evaluate(&self.facts) {
                        then_action
                    } else {
                        else_action
                    };
                    if let Some(branch) = branch {
                        let nested = self.resolve(&branch, step_index)?;
                        self.push(nested)?;
                    }
                }
                StepKind::Unknown { event, .. } => {
                    return Err(ActionError::UnsupportedEvent {
                        event: event.display(),
                        step: step_index,
                    });
                }
            }
        }
    }

    fn pop_frame(&mut self) {
        if let Some(frame) = self.frames.pop() {
            if frame.nested {
                self.depth = self.depth.saturating_sub(1);
            }
        }
    }

    fn resolve(&self, name: &str, step_index: usize) -> Result<usize, ActionError> {
        let Some(frame) = self.frames.last() else {
            return Err(ActionError::Playback {
                step: step_index,
                message: format!("action '{name}' is not in this set"),
            });
        };
        let Some(set) = self.library.set(frame.set) else {
            return Err(ActionError::NoTarget);
        };
        set.action_index(name).ok_or_else(|| ActionError::Playback {
            step: step_index,
            message: format!("action '{name}' is not in this set"),
        })
    }

    fn push(&mut self, action: usize) -> Result<(), ActionError> {
        if self.depth >= MAX_PLAY_DEPTH {
            return Err(ActionError::RecursionLimit);
        }
        let Some(frame) = self.frames.last() else {
            return Err(ActionError::NoTarget);
        };
        let set = frame.set;
        self.frames.push(Frame {
            set,
            action,
            next_step: 0,
            single: false,
            nested: true,
        });
        self.depth += 1;
        Ok(())
    }
}
