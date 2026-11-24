//! Batch options: the serialized model, validation, and override paths.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::naming::{BatchDate, FileNaming};
use crate::action::StepKind;
use crate::library::ActionLibrary;

pub const SAVE_AS_COMMAND: &str = "file.saveAs";
pub const OPEN_COMMAND: &str = "file.open";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BatchSource {
    #[default]
    Folder,
    OpenedFiles,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BatchDestination {
    #[default]
    None,
    SaveAndClose,
    Folder,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorPolicy {
    #[default]
    Stop,
    Log,
}

/// Everything the Batch dialog (and later Create Droplet) configures.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BatchOptions {
    pub set: String,
    pub action: String,
    pub source: BatchSource,
    pub source_folder: String,
    pub include_subfolders: bool,
    pub override_open: bool,
    pub suppress_open: bool,
    pub suppress_profile: bool,
    pub destination: BatchDestination,
    pub destination_folder: String,
    pub override_save: bool,
    pub naming: FileNaming,
    pub errors: ErrorPolicy,
    pub error_log: String,
}

/// Why a batch is refused before any input is processed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum BatchError {
    #[error("The action \u{201c}{action}\u{201d} in set \u{201c}{set}\u{201d} is not loaded.")]
    MissingAction { set: String, action: String },
    #[error("The source folder \u{201c}{0}\u{201d} does not exist.")]
    MissingSourceFolder(String),
    #[error("The destination folder \u{201c}{0}\u{201d} does not exist.")]
    MissingDestinationFolder(String),
    #[error(
        "The file naming must include a document name, serial number, or serial letter so files are not overwritten."
    )]
    NoUniqueField,
    #[error("The extension must be the last File Naming field.")]
    ExtensionNotLast,
    #[error("Choose a file to log errors to.")]
    MissingErrorLog,
    #[error(
        "Override Action \u{201c}Save As\u{201d} Commands is on, but the action contains no Save As step."
    )]
    NoSaveAsStep,
}

/// Refuse a batch whose options cannot run.
pub fn validate(options: &BatchOptions, library: &ActionLibrary) -> Result<(), BatchError> {
    let Some((set, action)) = find_action(library, &options.set, &options.action) else {
        return Err(BatchError::MissingAction {
            set: options.set.clone(),
            action: options.action.clone(),
        });
    };
    if options.source == BatchSource::Folder && !is_dir(&options.source_folder) {
        return Err(BatchError::MissingSourceFolder(
            options.source_folder.clone(),
        ));
    }
    if options.destination == BatchDestination::Folder {
        if !is_dir(&options.destination_folder) {
            return Err(BatchError::MissingDestinationFolder(
                options.destination_folder.clone(),
            ));
        }
        if !options.naming.has_unique_field() {
            return Err(BatchError::NoUniqueField);
        }
        // ponytail: saves pick the format from the file suffix, so a name like
        // `a.png001` cannot be written; CS6 saves it in the step's format.
        if !options.naming.extension_is_last() {
            return Err(BatchError::ExtensionNotLast);
        }
    }
    if options.errors == ErrorPolicy::Log && options.error_log.trim().is_empty() {
        return Err(BatchError::MissingErrorLog);
    }
    if options.override_save
        && options.destination != BatchDestination::None
        && !action_reaches_command(library, set, action, SAVE_AS_COMMAND)
    {
        return Err(BatchError::NoSaveAsStep);
    }
    Ok(())
}

fn is_dir(path: &str) -> bool {
    !path.is_empty() && Path::new(path).is_dir()
}

/// The (set, action) indices for names.
pub fn find_action(library: &ActionLibrary, set: &str, action: &str) -> Option<(usize, usize)> {
    let set_index = library.sets().iter().position(|s| s.name == set)?;
    let action_index = library.set(set_index)?.action_index(action)?;
    Some((set_index, action_index))
}

/// Whether the action, or an action it reaches through Play or Conditional
/// steps in its set, holds an enabled step running `command`.
pub fn action_reaches_command(
    library: &ActionLibrary,
    set: usize,
    action: usize,
    command: &str,
) -> bool {
    let mut visited = HashSet::new();
    reaches(library, set, action, command, &mut visited)
}

fn reaches(
    library: &ActionLibrary,
    set: usize,
    action: usize,
    command: &str,
    visited: &mut HashSet<usize>,
) -> bool {
    if !visited.insert(action) {
        return false;
    }
    let Some(set_ref) = library.set(set) else {
        return false;
    };
    let Some(action_ref) = set_ref.actions.get(action) else {
        return false;
    };
    for step in action_ref.steps.iter().filter(|step| step.flags.enabled) {
        let nested: Vec<&str> = match step.kind() {
            StepKind::Command { command: id, .. } if id.as_str() == command => return true,
            StepKind::Play { action } => vec![action.as_str()],
            StepKind::Conditional {
                then_action,
                else_action,
                ..
            } => [then_action, else_action]
                .into_iter()
                .flatten()
                .map(String::as_str)
                .collect(),
            _ => Vec::new(),
        };
        for name in nested {
            if let Some(next) = set_ref.action_index(name) {
                if reaches(library, set, next, command, visited) {
                    return true;
                }
            }
        }
    }
    false
}

/// Where an overridden Save As step writes, or `None` for a destination that
/// does not redirect it. `input` is the input's path (or document title for
/// an untitled document) and `ext` the step's format extension.
pub fn override_save_path(
    options: &BatchOptions,
    input: &Path,
    ext: &str,
    position: usize,
    date: BatchDate,
) -> Option<PathBuf> {
    let file_name = input.file_name()?.to_string_lossy().into_owned();
    match options.destination {
        BatchDestination::None => None,
        BatchDestination::Folder => Some(
            Path::new(&options.destination_folder)
                .join(options.naming.render(&file_name, ext, position, date)),
        ),
        BatchDestination::SaveAndClose => {
            let stem = input.file_stem()?.to_string_lossy().into_owned();
            Some(input.with_file_name(format!("{stem}.{}", ext.to_lowercase())))
        }
    }
}

/// One error-log line: `<input path>: <message>` with line breaks flattened.
pub fn error_log_line(input: &str, message: &str) -> String {
    let flat: String = message
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    format!("{input}: {}\n", flat.trim())
}
