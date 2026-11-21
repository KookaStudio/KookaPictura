# Save a Copy Operation Status

## MODIFIED Requirements

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
