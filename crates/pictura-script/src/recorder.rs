//! The recording session: where steps land, what is recordable, and the
//! success-only append contract.

use crate::action::{ActionStep, DialogOptions, StepFlags, StepKind, ToolStroke};
use crate::command::{self, CommandClass, CommandId, CommandParams, DialogMode};
use crate::error::ActionError;
use crate::library::ActionLibrary;

/// The panel selection recording starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    None,
    Set {
        set: usize,
    },
    Action {
        set: usize,
        action: usize,
    },
    Step {
        set: usize,
        action: usize,
        step: usize,
    },
}

/// Insertion point for recorded steps.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Target {
    set: usize,
    action: usize,
    insert_at: usize,
    /// The action name at `action`; a move/delete invalidates the raw index.
    action_name: String,
    /// The step just before `insert_at`; a delete/move invalidates the target.
    anchor: Option<Anchor>,
}

/// A cheap identity for the step a recording inserts after.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Anchor {
    kind: &'static str,
    display_name: String,
}

fn anchor_of(step: &ActionStep) -> Anchor {
    Anchor {
        kind: match &step.kind {
            StepKind::Command { .. } => "command",
            StepKind::Stop { .. } => "stop",
            StepKind::MenuItem { .. } => "menuItem",
            StepKind::InsertPath { .. } => "path",
            StepKind::Play { .. } => "play",
            StepKind::Conditional { .. } => "conditional",
            StepKind::ToolStroke { .. } => "toolStroke",
            StepKind::Unknown { .. } => "unknown",
        },
        display_name: step.display_name.clone(),
    }
}

fn make_target(
    library: &ActionLibrary,
    set: usize,
    action: usize,
    insert_at: usize,
) -> Option<Target> {
    let action_ref = library.action(set, action)?;
    let insert_at = insert_at.min(action_ref.steps.len());
    Some(Target {
        set,
        action,
        insert_at,
        action_name: action_ref.name.clone(),
        anchor: insert_at
            .checked_sub(1)
            .and_then(|index| action_ref.steps.get(index))
            .map(anchor_of),
    })
}

/// The active recording session. The library owns the data; the recorder owns
/// only the insertion point.
#[derive(Clone, Debug, Default)]
pub struct Recorder {
    target: Option<Target>,
}

impl Recorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_recording(&self) -> bool {
        self.target.is_some()
    }

    pub fn target(&self) -> Option<(usize, usize, usize)> {
        self.target
            .as_ref()
            .map(|target| (target.set, target.action, target.insert_at))
    }

    pub fn start(
        &mut self,
        library: &ActionLibrary,
        selection: Selection,
    ) -> Result<(), ActionError> {
        if self.is_recording() {
            return Err(ActionError::RecordingActive);
        }
        let target = match selection {
            Selection::None => return Err(ActionError::NoTarget),
            Selection::Set { set } => {
                let set_ref = library.set(set).ok_or(ActionError::NoTarget)?;
                let action = set_ref
                    .actions
                    .len()
                    .checked_sub(1)
                    .ok_or(ActionError::EmptySet { set })?;
                let steps = set_ref.actions[action].steps.len();
                make_target(library, set, action, steps).ok_or(ActionError::NoTarget)?
            }
            Selection::Action { set, action } => {
                let steps = library
                    .action(set, action)
                    .ok_or(ActionError::NoTarget)?
                    .steps
                    .len();
                make_target(library, set, action, steps).ok_or(ActionError::NoTarget)?
            }
            Selection::Step { set, action, step } => {
                library.action(set, action).ok_or(ActionError::NoTarget)?;
                make_target(library, set, action, step + 1).ok_or(ActionError::NoTarget)?
            }
        };
        self.target = Some(target);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.target = None;
    }

    /// Record only when the command reported success; a refused command leaves
    /// the recording active and the action unchanged.
    pub fn record_after(
        &mut self,
        success: bool,
        library: &mut ActionLibrary,
        command: &CommandId,
        params: &CommandParams,
    ) -> Result<bool, ActionError> {
        if !success {
            return Ok(false);
        }
        self.record(library, command, params)
    }

    /// Append a recordable command. Non-recordable commands return `false` and
    /// keep recording; an unclassified command is a named error.
    pub fn record(
        &mut self,
        library: &mut ActionLibrary,
        command: &CommandId,
        params: &CommandParams,
    ) -> Result<bool, ActionError> {
        let target = self.checked_target(library)?;
        let definition =
            command::definition(command.as_str()).ok_or_else(|| ActionError::Unclassified {
                command: command.to_string(),
            })?;
        if definition.class == CommandClass::NonRecordable {
            return Ok(false);
        }
        let (dialog_options, with_dialog) = match definition.dialog {
            DialogMode::None => (DialogOptions::NONE, false),
            DialogMode::Optional => (DialogOptions::OPTIONAL, false),
            DialogMode::Required => (DialogOptions::REQUIRED, false),
            DialogMode::Specific => (DialogOptions::SPECIFIC, false),
        };
        let step = ActionStep {
            flags: StepFlags {
                with_dialog,
                ..StepFlags::default()
            },
            dialog_options,
            display_name: definition.label.to_string(),
            kind: StepKind::Command {
                command: command.clone(),
                params: params.clone(),
            },
            imported: None,
        };
        let anchor = anchor_of(&step);
        insert(library, target, step)?;
        if let Some(target) = &mut self.target {
            target.insert_at += 1;
            target.anchor = Some(anchor);
        }
        Ok(true)
    }

    /// Append a completed tool stroke when Allow Tool Recording is on. The
    /// stroke's settings are captured here, not read at playback time.
    pub fn record_tool_stroke(
        &mut self,
        library: &mut ActionLibrary,
        allow_tool_recording: bool,
        stroke: ToolStroke,
    ) -> Result<bool, ActionError> {
        if !allow_tool_recording {
            return Ok(false);
        }
        let target = self.checked_target(library)?;
        if stroke.version != ToolStroke::VERSION {
            return Err(ActionError::Invalid(format!(
                "unsupported tool-stroke version {}",
                stroke.version
            )));
        }
        if let Some(kind) = stroke.settings().get_str("replay_kind") {
            if kind != "paint" {
                return Err(ActionError::Invalid(format!(
                    "tool stroke replay kind '{kind}' is not supported"
                )));
            }
        }
        if stroke.samples.len() > MAX_TOOL_SAMPLES || stroke.payload_len() > MAX_TOOL_PAYLOAD_BYTES
        {
            return Err(ActionError::ToolStrokeTooLarge {
                limit: MAX_TOOL_SAMPLES,
            });
        }
        let step = ActionStep {
            flags: StepFlags::default(),
            dialog_options: crate::action::DialogOptions::OPTIONAL,
            display_name: format!("Tool Stroke: {}", stroke.tool),
            kind: StepKind::ToolStroke { stroke },
            imported: None,
        };
        let anchor = anchor_of(&step);
        insert(library, target, step)?;
        if let Some(target) = &mut self.target {
            target.insert_at += 1;
            target.anchor = Some(anchor);
        }
        Ok(true)
    }

    /// Revalidate the raw target indices against the library. A vanished or
    /// moved action clears the session instead of recording into a stranger.
    fn checked_target(&mut self, library: &ActionLibrary) -> Result<Target, ActionError> {
        let target = self
            .target
            .as_ref()
            .ok_or(ActionError::NotRecording)?
            .clone();
        let valid = library
            .action(target.set, target.action)
            .is_some_and(|action| {
                let anchor_matches = match target.anchor.as_ref() {
                    Some(anchor) => {
                        target
                            .insert_at
                            .checked_sub(1)
                            .and_then(|index| action.steps.get(index))
                            .map(anchor_of)
                            .as_ref()
                            == Some(anchor)
                    }
                    None => true,
                };
                action.name == target.action_name
                    && target.insert_at <= action.steps.len()
                    && anchor_matches
            });
        if !valid {
            self.target = None;
            return Err(ActionError::NoTarget);
        }
        Ok(target)
    }
}

/// The smallest stroke that the app-native payload accepts.
pub const MAX_TOOL_SAMPLES: usize = 20_000;
/// The payload bound for a tool stroke.
pub const MAX_TOOL_PAYLOAD_BYTES: usize = 1_000_000;

fn insert(
    library: &mut ActionLibrary,
    target: Target,
    step: ActionStep,
) -> Result<(), ActionError> {
    let action = library
        .action_mut(target.set, target.action)
        .ok_or(ActionError::NoTarget)?;
    let index = target.insert_at.min(action.steps.len());
    action.steps.insert(index, step);
    Ok(())
}
