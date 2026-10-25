//! Kooka Pictura's automation engine: the command catalogue, action library,
//! recorder, and player. Qt-free, so the engine is testable headless.
//!
//! The `.atn`/`.psp` container codec lives in [`pictura_codec::action_file`];
//! [`atn`] maps it to the action model used here.

pub mod action;
pub mod atn;
pub mod command;
pub mod command_catalogue;
pub mod error;
pub mod library;
pub mod player;
pub mod recorder;
pub mod tool_catalogue;

pub use action::{
    Action, ActionColor, ActionSet, ActionStep, Condition, DialogOptions, DocumentFacts, EventKey,
    FunctionKey, ImportedStepForm, ModalState, Platform, StepFlags, StepKind, ToolSample,
    ToolStroke,
};
pub use command::{
    CommandClass, CommandDefinition, CommandError, CommandId, CommandParams, DialogMode, EventId,
    ParameterSpec,
};
pub use error::ActionError;
pub use library::{ActionLibrary, ResetMode};
pub use player::{PlaybackCommand, PlaybackOptions, PlaybackSelection, PlaybackSpeed, Player};
pub use recorder::{Recorder, Selection};
