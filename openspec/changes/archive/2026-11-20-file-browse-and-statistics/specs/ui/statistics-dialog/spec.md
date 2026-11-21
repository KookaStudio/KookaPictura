# Spec Delta

## Purpose

The Image Statistics dialog collects the sources, stack mode, and alignment for
`File > Scripts > Statistics…`.

## ADDED Requirements

### Requirement: Image Statistics controls

The dialog SHALL be titled `Image Statistics` and SHALL contain a stack-mode menu
labelled `Choose Stack Mode`, a Source Files group with a `Use` menu offering
exactly `Files` and `Folder`, a source list, the buttons `Browse…`, `Remove`, and
`Add Open Files`, the checkbox `Attempt to Automatically Align Source Images`,
and the buttons `OK` and `Cancel`. The stack-mode menu SHALL offer Mean,
Summation, Minimum, Maximum, Median, Variance, Standard Deviation, Skewness,
Kurtosis, Range, and Entropy, in that order. The dialog SHALL NOT offer a
smart-object option.

#### Scenario: The dialog presents the Statistics controls [sd_controls]
- **WHEN** `File > Scripts > Statistics…` opens the dialog
- **THEN** it shows the stack-mode menu, Source Files, Use, the list, Browse…,
  Remove, Add Open Files, the align checkbox, OK, and Cancel, and no smart-object
  checkbox

### Requirement: Dialog defaults and two-file gate

The dialog SHALL default to `Use: Files`, `Choose Stack Mode: Mean`, align off,
and an empty list when no remembered choice is supplied. `OK` and the align
checkbox SHALL be disabled until the list holds at least two files. `Remove`
SHALL be enabled only with a selection; `Add Open Files` SHALL be enabled when at
least one document is open when the dialog opens.

#### Scenario: The defaults match CS6 [sd_defaults]
- **WHEN** the dialog opens with no remembered choice
- **THEN** Use is Files, the stack mode is Mean, align is off, the list is empty,
  and OK is disabled

#### Scenario: Two files unlock OK and align [sd_two_files]
- **WHEN** files are added one at a time
- **THEN** OK and the align checkbox stay disabled at one file and become
  enabled at two

#### Scenario: Buttons follow the selection and open documents [sd_buttons]
- **WHEN** the dialog opens with no selected row and no open document
- **THEN** Remove and Add Open Files are disabled; selecting a row enables
  Remove, and constructing the dialog with a saved open document enables
  Add Open Files

### Requirement: Source selection

`Browse…` in Files mode SHALL add the chosen image files to the list. `Browse…`
in Folder mode SHALL add the chosen folder's supported image files to the list,
without descending into subfolders. `Remove` SHALL remove the selected rows.
`Add Open Files` SHALL append open documents that are saved and unmodified; when
an open document has unsaved changes, it SHALL warn that documents must be saved
before they can be merged and SHALL add only the eligible documents. A path SHALL
appear at most once. The list SHALL preserve the order in which files were added
and SHALL display file names.

#### Scenario: Browse adds files in order [sd_browse_files]
- **WHEN** two image files are added through the file browser
- **THEN** both appear in that order

#### Scenario: Folder Browse adds the folder's images [sd_folder]
- **WHEN** Use is Folder and a folder containing images and a text file is chosen
- **THEN** the folder's supported images are added and the text file is not

#### Scenario: Remove drops the selection [sd_remove]
- **WHEN** one row is selected and Remove is pressed
- **THEN** only that row leaves the list

#### Scenario: Add Open Files takes saved clean documents [sd_add_open]
- **WHEN** Add Open Files is pressed with one saved clean document and one
  unsaved document open
- **THEN** the saved clean document is added, the unsaved one is not, and a
  warning says documents must be saved

#### Scenario: Duplicates are ignored [sd_dedupe]
- **WHEN** the same path is added twice
- **THEN** the list contains it once

### Requirement: Remembered choices seed the dialog

The dialog SHALL preselect the stack mode and align flag it is given from the
session's remembered choice.

#### Scenario: Remembered values are preselected [sd_remembered]
- **WHEN** the dialog is constructed with a remembered Skewness mode and align on
- **THEN** the stack-mode menu shows Skewness and align is checked

### Requirement: Cancel leaves the application unchanged

Opening and cancelling the dialog SHALL leave every open document's pixels, path,
modified state, and history unchanged.

#### Scenario: Cancel changes nothing [sd_cancel]
- **WHEN** the dialog is opened and cancelled
- **THEN** every open document is unchanged
