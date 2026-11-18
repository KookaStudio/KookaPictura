# batch-processing Specification

## Purpose
Running one action over a set of files: source resolution, the per-input
open/play/save/close pipeline, the CS6 Open and Save As overrides, destinations,
error handling, and cancellation.

## ADDED Requirements

### Requirement: Batch options validation

A batch SHALL be described by: the action (set and action name), the source
(`Folder` with a path and `Include All Subfolders`, or `Opened Files`),
`Override Action "Open" Commands`, `Suppress File Open Options Dialogs`,
`Suppress Color Profile Warnings`, the destination (`None`, `Save And Close`, or
`Folder` with a path), `Override Action "Save As" Commands`, the File Naming
scheme, and the error policy (`Stop For Errors` or `Log Errors To File` with a
path). Before any input is processed, the system SHALL refuse the batch with a
named diagnostic when the action does not exist in the loaded library, when a
Folder source path is not an existing directory, when a Folder destination path
is not an existing directory, when a Folder destination's scheme fails the
unique-field rule, when `Log Errors To File` has no path, or when
`Override Action "Save As" Commands` is on and neither the action nor any action
it reaches through Play steps contains an enabled `Save As` step.

#### Scenario: A missing action refuses the batch [bp_validate_action]
- **WHEN** the batch names an action that is not in the library
- **THEN** validation fails naming the action and no input is processed

#### Scenario: Override Save As needs a Save As step [bp_validate_save_step]
- **WHEN** `Override Action "Save As" Commands` is on and the action and its played actions contain no enabled Save As step
- **THEN** validation fails with the Save As diagnostic

#### Scenario: A nested Save As step satisfies the override [bp_validate_nested_save]
- **WHEN** the action contains only a Play step whose target action contains an enabled Save As step
- **THEN** validation passes the Save As check

#### Scenario: Folder paths must exist [bp_validate_folders]
- **WHEN** the source folder or the destination folder does not exist
- **THEN** validation fails naming that folder

### Requirement: Folder source resolution

A `Folder` source SHALL resolve to the regular files directly in the folder
whose extension the application can open, skipping hidden entries (names that
begin with `.`). With `Include All Subfolders`, the system SHALL also include
such files in every non-hidden subfolder, following symbolic links to folders
(CS6 documents batching several folders through aliases in one folder) and
visiting each canonical folder at most once. Inputs SHALL be ordered by their
path relative to the source folder, compared case-insensitively; the order is
an inferred app decision.

#### Scenario: Top-level files only without subfolders [bp_source_folder]
- **WHEN** a folder holds `b.jpg`, `A.png`, `notes.txt`, `.hidden.jpg`, and a subfolder with `c.jpg`, and `Include All Subfolders` is off
- **THEN** the inputs are `A.png` then `b.jpg`

#### Scenario: Subfolders are included when asked [bp_source_recurse]
- **WHEN** the same folder is resolved with `Include All Subfolders` on
- **THEN** the inputs are `A.png`, `b.jpg`, and `sub/c.jpg`

#### Scenario: A folder link cycle terminates [bp_source_cycle]
- **WHEN** a subfolder contains a symbolic link back to the source folder and `Include All Subfolders` is on
- **THEN** resolution terminates and each file appears once

### Requirement: Opened Files source

An `Opened Files` source SHALL resolve to the documents open when the batch
starts, in tab order. A document opened while the batch runs SHALL NOT become
an input. A document closed by the user before its turn SHALL be skipped
without an error.

#### Scenario: Open documents are processed in tab order [bp_source_opened]
- **WHEN** two documents are open and a batch with an `Opened Files` source runs an action that rotates the canvas 90°
- **THEN** both documents are rotated, first tab first

### Requirement: Per-input pipeline

For each input in order, the system SHALL open a Folder-source file (or
activate an Opened-Files document), make it the active document, and play the
action on it with playback dialogs suppressed. When the action finishes, the
system SHALL apply the destination to the active document and then advance to
the next input. Opening SHALL skip the open-options dialogs when
`Suppress File Open Options Dialogs` is on and SHALL keep the embedded profile
without asking when `Suppress Color Profile Warnings` is on. With destination
`None`, documents SHALL be left open and unsaved unless the action saved them.
With destination `Save And Close` or `Folder`, the document SHALL be closed
after it is saved. The batch SHALL NOT add a batch-level history state.

#### Scenario: Destination None leaves documents open [bp_dest_none]
- **WHEN** a Folder source with two files runs an action with no Save step and destination `None`
- **THEN** both files are open as modified documents and the source files are unchanged on disk

#### Scenario: Save And Close overwrites in place [bp_dest_save_close]
- **WHEN** a Folder source with two PNG files runs an action that rotates the image 90° with destination `Save And Close`
- **THEN** each source PNG on disk holds the rotated image and neither document remains open

#### Scenario: Folder destination writes named files [bp_dest_folder]
- **WHEN** a Folder source with two files runs an action with no Save As step, destination `Folder`, and the scheme `Document Name` + `2 Digit Serial Number` with starting serial 1
- **THEN** the destination holds `<name1>01.<ext>` and `<name2>02.<ext>` in each input's own format, and neither document remains open

### Requirement: Override Action Open commands

With `Override Action "Open" Commands` on, an Open step reached during a batch
SHALL NOT open its recorded file and SHALL succeed, so later steps apply to the
batch input. With the override off, an Open step SHALL open its recorded file
as during normal playback, and later steps SHALL apply to that newly active
document (the documented CS6 behaviour, in which the batch inputs are not
processed).

#### Scenario: The override processes the batch inputs [bp_override_open]
- **WHEN** an action of Open `x.png` then Rotate 90° runs over two inputs with the override on and destination `Save And Close`
- **THEN** both inputs are rotated on disk, `x.png` is unchanged, and no document remains open

#### Scenario: Without the override the recorded file is opened [bp_no_override_open]
- **WHEN** the same action runs over one input with the override off and destination `None`
- **THEN** `x.png` is opened and rotated, and the input document is left open and not rotated

### Requirement: Override Action Save As commands

With `Override Action "Save As" Commands` on, a Save As step reached during a
batch SHALL save the active document in the step's recorded format with the
step's recorded format options, ignoring the step's recorded folder and file
name. Under a `Folder` destination, the file SHALL be written to the
destination folder under the File Naming scheme, with the step's format
extension. Under `Save And Close`, the file SHALL be written to the input's own
folder under the input's base name and the step's format extension. Without the
override, a Save As step SHALL save to its recorded path, so every input
overwrites the same file, and a `Folder` destination still saves the named copy
afterwards (the documented CS6 double save).

#### Scenario: Override Save As writes to the destination with the step's options [bp_override_save_folder]
- **WHEN** an action with a JPEG Save As step recorded at quality 3 runs over two PNG inputs with destination `Folder`, the override on, and the scheme `Document Name` + `extension`
- **THEN** the destination holds `<name1>.jpg` and `<name2>.jpg` encoded at quality 3, and the step's recorded path is not written

#### Scenario: Save And Close with the override keeps the input folder [bp_override_save_close]
- **WHEN** the same action runs over a PNG input with destination `Save And Close` and the override on
- **THEN** a JPEG with the input's base name is written beside the input, and the document is closed

#### Scenario: Without the override the recorded path is reused [bp_no_override_save]
- **WHEN** the action runs over two inputs with destination `None` and the override off
- **THEN** the step's recorded path holds the second input's result

### Requirement: Batch error policy

A failure to open an input, a failed step, or a failed save SHALL be an input
error. Under `Stop For Errors`, the first input error SHALL stop the batch and
report the input and the failure; inputs already processed SHALL keep their
results. Under `Log Errors To File`, each input error SHALL append one line
`<input path>: <message>` to the log file, the failing input's batch-opened
document SHALL be closed without saving, and the batch SHALL continue with the
next input; at the end, the batch SHALL report how many errors it logged and
the log path. When the log file cannot be opened for writing at the start, the
batch SHALL fall back to `Stop For Errors` and report the unwritable path. The
log format is an inferred app decision; CS6 does not document it.

#### Scenario: Stop For Errors halts on the first failure [bp_error_stop]
- **WHEN** the second of three inputs is not a readable image and the policy is `Stop For Errors`
- **THEN** the batch stops after the second input, reports it, and the third input is not opened

#### Scenario: Log Errors continues and records the failure [bp_error_log]
- **WHEN** the same inputs run with `Log Errors To File`
- **THEN** the first and third inputs are processed, the log holds exactly one line naming the second input, and the end report counts one error

#### Scenario: An unwritable log falls back to stopping [bp_error_log_unwritable]
- **WHEN** `Log Errors To File` names a path inside a directory that does not exist
- **THEN** the batch reports the path and stops at the first input error

### Requirement: Batch cancellation

Cancelling a batch SHALL stop after the current step, SHALL NOT save the
current input, SHALL close the current input's batch-opened document without
saving, and SHALL process no further inputs. Outputs already written SHALL be
kept. A save interrupted by cancellation SHALL NOT leave a partially written
file at the output path.

#### Scenario: Cancel stops after the current step [bp_cancel]
- **WHEN** a batch over three inputs with destination `Folder` is cancelled while the second input's action is playing
- **THEN** the destination holds only the first input's output and no batch-opened document remains open
