//! Typed engine errors. Every refusal names what failed so the panel can show
//! a diagnostic instead of silently doing nothing.

/// An engine, library, recorder, or playback failure.
#[derive(Clone, Debug, thiserror::Error)]
pub enum ActionError {
    #[error("failed to load action library from {source_name}: {message}")]
    Load {
        source_name: String,
        offset: Option<usize>,
        message: String,
    },
    #[error("failed to save action library to {source_name}: {message}")]
    Save {
        source_name: String,
        message: String,
    },
    #[error("no action set or action is selected")]
    NoTarget,
    #[error("action set {set} has no action to record into")]
    EmptySet { set: usize },
    #[error("invalid selection")]
    InvalidSelection,
    #[error("conditional branches must name actions in the same set")]
    ConditionalCrossSet,
    #[error("a conditional needs a Then or Else action")]
    ConditionalEmpty,
    #[error("conditional cycle: {0}")]
    ConditionalCycle(String),
    #[error("action '{action}' is referenced by a conditional")]
    ActionReferenced { action: String },
    #[error("action '{name}' already exists in this set")]
    DuplicateActionName { name: String },
    #[error("another recording is already active")]
    RecordingActive,
    #[error("no recording is active")]
    NotRecording,
    #[error("playback is already active")]
    PlaybackActive,
    #[error("command '{command}' has no classification")]
    Unclassified { command: String },
    #[error("command '{command}' is not recordable")]
    NotRecordable { command: String },
    #[error("unsupported event '{event}' at step {step}")]
    UnsupportedEvent { event: String, step: usize },
    #[error("playback failed at step {step}: {message}")]
    Playback { step: usize, message: String },
    #[error("playback depth limit of 64 invocations exceeded")]
    RecursionLimit,
    #[error("playback cancelled")]
    Cancelled,
    #[error("no active document")]
    NoDocument,
    #[error("function key {0} is already assigned to another action")]
    FunctionKeyTaken(String),
    #[error("function key {0} is not available on this platform")]
    FunctionKeyUnavailable(String),
    #[error("tool stroke exceeds the {limit}-sample limit")]
    ToolStrokeTooLarge { limit: usize },
    #[error("invalid dialog options {0}")]
    InvalidDialogOptions(u8),
    #[error("action file error: {0}")]
    Atn(String),
    #[error("{0}")]
    Invalid(String),
}
