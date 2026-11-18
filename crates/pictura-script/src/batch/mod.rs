//! File > Automate > Batch: the Qt-free rules (sources, naming, validation,
//! override paths, error log). The app's runner only sequences open, play,
//! save, and close.

pub mod naming;
pub mod options;
pub mod source;

pub use naming::{BatchDate, FileNaming, NamingField, TOKEN_LABELS};
pub use options::{
    action_reaches_command, error_log_line, find_action, override_save_path, validate,
    BatchDestination, BatchError, BatchOptions, BatchSource, ErrorPolicy, OPEN_COMMAND,
    SAVE_AS_COMMAND,
};
pub use source::resolve_folder;

#[cfg(test)]
mod tests;
