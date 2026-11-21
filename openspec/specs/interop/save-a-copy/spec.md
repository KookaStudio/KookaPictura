# interop/save-a-copy Specification

## Purpose
The `File > Save a Copy` command: writing a copy in any supported save format
without re-pointing, marking, or otherwise changing the open document.

## Requirements

### Requirement: Save a Copy is a File menu command

`File > Save a Copy…` SHALL be a command in the File menu, enabled when a
document is open and disabled when none is. For an editable document, it SHALL open a save-path dialog
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

- **WHEN** Save a Copy is opened for an editable single-layer PNG-imported document
- **THEN** the format list matches Save As and preselects PNG

#### Scenario: Cancelling writes nothing [copy-cancel]

- **WHEN** the Save a Copy path dialog or a format options dialog is cancelled
- **THEN** no file is written and the document is unchanged

#### Scenario: The profile dialog matches Save As [copy-profile-dialog]

- **WHEN** Save a Copy is run for a `.psd`, `.psb`, `.jpg`, or `.tif` destination and the Embed Color Profile dialog is cancelled
- **THEN** no file is written and the document is unchanged

### Requirement: A copy does not change the open document

For an editable document, a successful Save a Copy SHALL write the chosen file atomically and SHALL NOT
change the active document's recorded path, output format, modified state,
history (including adding an undo state), recovery snapshot, recent-files list,
tab title, or window title. The copy SHALL NOT become the active document or its
recorded path. A copy SHALL NOT record an action step. The same feature-loss
warning as Save As SHALL apply before a layered document is written to a format
that cannot hold it. The command SHALL work for an untitled document, requesting
a destination path first and leaving the document untitled. A pre-rename failed write SHALL report failure and leave the document and any previous file unchanged, with no partial file. A native copy that has been renamed but not durably acknowledged SHALL report uncertain publication with copy-specific feedback; the destination may contain the copy, and the open document and its recovery data SHALL remain unchanged.

#### Scenario: A modified document stays modified [copy-nonmutating-raster]

- **WHEN** a modified document recorded at `a.psd` is saved as a copy to `b.png`
- **THEN** `b.png` holds the flattened composite, the document's path is still `a.psd`, and it remains modified with the same history

#### Scenario: A PSD copy keeps the document in place [copy-psd]

- **WHEN** the user saves a copy of an editable open document to `backup.psd`
- **THEN** the copy is a full layered PSD and the open document's path and format are unchanged

#### Scenario: A PDF copy uses the PDF dialog [copy-pdf]

- **WHEN** the user saves a copy of an editable document to `print.pdf`
- **THEN** the Save Adobe PDF dialog is shown, the PDF is written, and the document's path and modified state are unchanged

#### Scenario: An untitled document stays untitled [copy-untitled]

- **WHEN** Save a Copy is run on an untitled document
- **THEN** a path is requested, the copy is written, and the document remains untitled

#### Scenario: A GIF copy leaves the document RGB [copy-gif]

- **WHEN** an editable RGB document is saved as a copy to `web.gif`
- **THEN** a GIF89a file is written and the document's mode, path, modified state, and history are unchanged

#### Scenario: A copy records no action step [copy-no-recorder]

- **WHEN** an action is recording and a copy is written
- **THEN** the action gains no step for the copy

#### Scenario: A failed copy leaves everything intact [copy-failed]

- **WHEN** the copy destination cannot be written and a file already exists there
- **THEN** the save reports failure, the existing file's bytes are unchanged, the document is unchanged, and no partial file remains

#### Scenario: A native copy panics after rename [os_copy_panic_after_rename]

- **WHEN** a native Save a Copy panics after rename and before durable acknowledgement
- **THEN** it reports `OP_ERR_PANIC` and uncertain publication with copy-specific feedback, the destination contains the staged copy, and the source path, modified state, history, recovery data and other session state are unchanged

### Requirement: Read-only compatibility copies use the source composite

For a document opened read-only by the version policy, Save a Copy SHALL use a restricted flow. When a real source composite is available, it SHALL offer a flattened PNG at a destination distinct from the source, after an explicit feature-loss confirmation. The copy SHALL be written atomically from the original merged image. Source decoding SHALL be fallible: corruption or source changes after open SHALL cause a reasoned refusal before replacing any destination. If the source composite is unavailable, Save a Copy SHALL explain why it cannot produce a faithful copy. These restrictions apply to the path-based application save entry points as well as the dialog. Existing nonmutating copy behavior SHALL apply to the open session.

#### Scenario: A confirmed flattened copy uses original pixels [dvp_flattened_copy]
- **WHEN** the user confirms Save a Copy for a read-only file with a real source composite and chooses a distinct PNG path
- **THEN** the PNG contains the source composite and the original file, path, modified state, and history remain unchanged

#### Scenario: An unavailable or refused copy writes nothing [dvp_copy_refused]
- **WHEN** the user cancels the warning, selects the source as destination, requests layered output, or attempts to copy a read-only file without a usable merged image
- **THEN** the application writes no file and explains any compatibility refusal
