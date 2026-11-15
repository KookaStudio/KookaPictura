# Spec Delta

## Purpose

The `File > Save a Copy` command: writing a copy in any supported save format
without re-pointing, marking, or otherwise changing the open document.

## ADDED Requirements

### Requirement: Save a Copy is a File menu command

`File > Save a Copy…` SHALL be a command in the File menu, enabled when a
document is open and disabled when none is. It SHALL open a save-path dialog
titled "Save a Copy" that offers the same format filters and default-format
rules as Save As, followed by the same per-format options dialog and the same
Embed Color Profile dialog that format would open for Save As. A document
already in Indexed Color mode saved as GIF SHALL open no dialog beyond the path
dialog; a document not already in Indexed Color mode SHALL open the Indexed
Color dialog. Cancelling the path dialog or any options dialog SHALL write
nothing and leave the document unchanged.

#### Scenario: The command follows the document [copy-command-enabled]

- **WHEN** a document is open and then all documents are closed
- **THEN** Save a Copy is enabled with a document and disabled without one

#### Scenario: The dialog matches the Save As matrix [copy-dialog-matrix]

- **WHEN** Save a Copy is opened for a single-layer PNG-imported document
- **THEN** the format list matches Save As and preselects PNG

#### Scenario: Cancelling writes nothing [copy-cancel]

- **WHEN** the Save a Copy path dialog or a format options dialog is cancelled
- **THEN** no file is written and the document is unchanged

#### Scenario: The profile dialog matches Save As [copy-profile-dialog]

- **WHEN** Save a Copy is run for a `.psd`, `.psb`, `.jpg`, or `.tif` destination and the Embed Color Profile dialog is cancelled
- **THEN** no file is written and the document is unchanged

### Requirement: A copy does not change the open document

A successful Save a Copy SHALL write the chosen file atomically and SHALL NOT
change the active document's recorded path, output format, modified state,
history (including adding an undo state), recovery snapshot, recent-files list,
tab title, or window title. The copy SHALL NOT become the active document or its
recorded path. A copy SHALL NOT record an action step. The same feature-loss
warning as Save As SHALL apply before a layered document is written to a format
that cannot hold it. The command SHALL work for an untitled document, requesting
a destination path first and leaving the document untitled. A failed write SHALL
report failure and leave the document and any previous file unchanged, with no
partial file.

#### Scenario: A modified document stays modified [copy-nonmutating-raster]

- **WHEN** a modified document recorded at `a.psd` is saved as a copy to `b.png`
- **THEN** `b.png` holds the flattened composite, the document's path is still `a.psd`, and it remains modified with the same history

#### Scenario: A PSD copy keeps the document in place [copy-psd]

- **WHEN** the user saves a copy of an open document to `backup.psd`
- **THEN** the copy is a full layered PSD and the open document's path and format are unchanged

#### Scenario: A PDF copy uses the PDF dialog [copy-pdf]

- **WHEN** the user saves a copy to `print.pdf`
- **THEN** the Save Adobe PDF dialog is shown, the PDF is written, and the document's path and modified state are unchanged

#### Scenario: An untitled document stays untitled [copy-untitled]

- **WHEN** Save a Copy is run on an untitled document
- **THEN** a path is requested, the copy is written, and the document remains untitled

#### Scenario: A GIF copy leaves the document RGB [copy-gif]

- **WHEN** an RGB document is saved as a copy to `web.gif`
- **THEN** a GIF89a file is written and the document's mode, path, modified state, and history are unchanged

#### Scenario: A copy records no action step [copy-no-recorder]

- **WHEN** an action is recording and a copy is written
- **THEN** the action gains no step for the copy

#### Scenario: A failed copy leaves everything intact [copy-failed]

- **WHEN** the copy destination cannot be written and a file already exists there
- **THEN** the save reports failure, the existing file's bytes are unchanged, the document is unchanged, and no partial file remains
