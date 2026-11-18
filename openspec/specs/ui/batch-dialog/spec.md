# ui/batch-dialog Specification

## Purpose
The `File > Automate > Batch…` dialog, its progress dialog, and its end-of-run
reporting.

## Requirements

### Requirement: Batch menu command

`File > Automate > Batch…` (`file.automate.batch`) SHALL be implemented and
SHALL open the modal Batch dialog. The command SHALL be enabled when the
action library holds at least one action and no playback, recording, or batch
is running.

#### Scenario: The command opens the dialog [bd_menu]
- **WHEN** the library holds an action and `file.automate.batch` is dispatched
- **THEN** the Batch dialog opens

#### Scenario: The command is disabled without actions [bd_menu_disabled]
- **WHEN** the library holds no action
- **THEN** `file.automate.batch` is disabled

### Requirement: Batch dialog controls

The dialog SHALL embed one `BatchOptionsWidget` with these groups:

- **Play:** `Set` and `Action` combo boxes listing the loaded sets and the
  chosen set's actions, defaulting to the Actions panel's selected set and
  action.
- **Source:** a combo box with exactly `Folder` and `Opened Files`, a
  `Choose…` button and folder label, and the `Override Action "Open" Commands`,
  `Include All Subfolders`, `Suppress File Open Options Dialogs`, and
  `Suppress Color Profile Warnings` checkboxes.
- **Destination:** a combo box with exactly `None`, `Save and Close`, and
  `Folder`, a `Choose…` button and folder label, and the
  `Override Action "Save As" Commands` checkbox.
- **File Naming:** a `FileNamingWidget` with six editable field combo boxes
  offering the File Naming tokens and accepting custom text, an `Example:`
  label, a `Starting serial#` spin box (minimum 1), and `Windows`, `Mac OS`,
  and `Unix` compatibility checkboxes, with `Unix` checked and disabled.
- **Errors:** a combo box with `Stop For Errors` and `Log Errors To File`, and
  a `Save As…` button and log path label.

`Choose…` and `Include All Subfolders` SHALL be enabled only for the `Folder`
source. The destination `Choose…` button and the File Naming group SHALL be
enabled only for the `Folder` destination, and
`Override Action "Save As" Commands` only for `Save and Close` and `Folder`.
The error log `Save As…` button SHALL be enabled only for
`Log Errors To File`. The default naming fields SHALL be `Document Name` then
`extension`.

#### Scenario: The dialog shows every group [bd_controls]
- **WHEN** the Batch dialog opens
- **THEN** it contains the Play, Source, Destination, File Naming, and Errors controls with the listed choices

#### Scenario: Controls follow the source and destination [bd_enablement]
- **WHEN** the source is set to `Opened Files` and the destination to `None`
- **THEN** the source `Choose…`, `Include All Subfolders`, destination `Choose…`, `Override Action "Save As" Commands`, and File Naming controls are disabled

#### Scenario: The example follows the scheme [bd_naming_example]
- **WHEN** the naming fields are `DOCUMENT NAME`, `1 Digit Serial Number`, `extension` with starting serial 3
- **THEN** the example reads `MYFILE3.gif`

### Requirement: Batch dialog acceptance

Pressing OK SHALL validate the options with the batch engine. A validation
failure SHALL show the diagnostic and keep the dialog open with no input
processed. When the options are valid but the chosen action contains an Open
step and `Override Action "Open" Commands` is off, the dialog SHALL warn that
the batch inputs will not be processed and SHALL start the batch only when the
user confirms. An accepted batch SHALL persist its options as the next
dialog's defaults.

#### Scenario: Invalid options keep the dialog open [bd_invalid]
- **WHEN** OK is pressed with a `Folder` destination and no destination folder chosen
- **THEN** the diagnostic is shown, the dialog stays open, and no batch starts

#### Scenario: A baked-in Open warns before running [bd_open_warning]
- **WHEN** OK is pressed for an action containing an Open step with `Override Action "Open" Commands` off
- **THEN** a warning is shown, and declining it starts no batch

#### Scenario: Options persist between runs [bd_persist]
- **WHEN** a batch is accepted with a source folder, `Include All Subfolders`, and starting serial 5, and the dialog is opened again
- **THEN** the dialog shows that folder, `Include All Subfolders` on, and starting serial 5

### Requirement: Batch progress and reporting

A running batch SHALL show a progress dialog naming the current input and its
position (`n of N`), advancing once per input, with a `Cancel` button that
cancels the batch. When a batch with `Log Errors To File` ends after logging at
least one error, the system SHALL show a message giving the error count and log
path. When `Stop For Errors` stops a batch, the system SHALL show the failing
input and the failure.

#### Scenario: Progress advances per input and Cancel stops [bd_progress]
- **WHEN** a batch over three inputs is running and Cancel is pressed during the second
- **THEN** the progress dialog had shown `1 of 3` and `2 of 3`, and the batch stops without processing the third input

#### Scenario: Logged errors are reported at the end [bd_log_message]
- **WHEN** a batch with `Log Errors To File` logs one error
- **THEN** the end message states one error and the log path
