# Spec Delta

## MODIFIED Requirements

### Requirement: Save and Save As

The system SHALL write the active document in its current output format, chosen
by the save path's extension, writing atomically so a failed write never
truncates an existing file. A `psd` path SHALL write the document's composite
and layer tree through the PSD codec; a `psb` path SHALL force a version-2 PSB
container through the codec. A supported raster path (PNG, JPEG, TIFF, WebP, or
BMP) SHALL write the flattened composite through the Qt-boundary raster encode
edge. Save As SHALL default its format to PSD unless the document is a single
pixel or background layer imported from a supported raster, in which case it
SHALL default to that source format. When the document's recorded path is a
raster format and the document has more than one layer, a group, an adjustment,
or a type layer, Save SHALL behave as Save As rather than silently flattening
into that raster file. Saving a document that has more than one layer, a group,
an adjustment, or a type layer to a format that cannot hold them SHALL warn
before writing and then write the flattened composite. The `composite`
serialized to PSD SHALL be the active document's current rendered composite, so
a save after any edit round-trips to the edited pixels rather than to a pre-edit
merged image. The `write_psd` byte layout and its use of `doc.composite.channels`
to derive the PSD colour-plane count MUST NOT change; an RGB document's merged
composite is RGBA (four planes) after a canvas rebuild, while a grayscale
document keeps its composite colour-plane count. Save on an untitled document
SHALL behave as Save As. On success the save SHALL record the file path. It
SHALL clear the modified state only when the document is the same document that
was snapshotted when the write started — the same history identity, content
revision, and document generation — and that snapshot is still current; a
document changed after the snapshot SHALL keep the recorded path and remain
modified. When the document generation changed during the save (the document was
replaced), the replacement document's path and modified state SHALL be
unchanged. On failure the path and modified state SHALL be unchanged and no
temporary file SHALL remain.

#### Scenario: Save As writes a file

- **WHEN** the user runs Save As and chooses a path
- **THEN** a valid file of the chosen path's format is written at that path and the document is no longer modified

#### Scenario: Save uses the recorded path

- **WHEN** a document has a path and the user runs Save
- **THEN** the file at that path is overwritten atomically, in that path's format

#### Scenario: Save on an untitled document prompts for a path

- **WHEN** Save is invoked on an untitled document
- **THEN** a Save As path is requested before any write

#### Scenario: Save after an edit writes the edited composite

- **WHEN** an edit changes the canvas and the document is saved and read back
- **THEN** the read-back document's composite equals the edited rendered composite, not the pre-edit merged image, and its colour-plane count matches the saved document's colour-plane count

#### Scenario: Save As defaults to the import source format

- **WHEN** a single-layer PNG imported through `File > Open` is saved with Save As
- **THEN** the Save As dialog preselects PNG as the format

#### Scenario: A raster save clears modified and re-saves the same format

- **WHEN** a document is saved to `photo.png` and then edited and saved again
- **THEN** the second save writes a PNG to `photo.png` and clears the modified state

#### Scenario: A layered document saved to a flat format warns and flattens

- **WHEN** the user saves a document with more than one layer to a PNG
- **THEN** a warning is shown that the format cannot hold the document's features, and on confirmation the flattened composite is written as a PNG

#### Scenario: A layered document defaults Save As to PSD

- **WHEN** a PNG imported through `File > Open` has a layer or an adjustment added and Save As is invoked
- **THEN** the Save As dialog preselects PSD, not PNG

#### Scenario: Save on a layered raster path behaves as Save As

- **WHEN** a layered document recorded at a `.png` path is saved
- **THEN** the format-aware Save As dialog opens (defaulting to PSD) instead of overwriting the PNG with a flattened composite

#### Scenario: A PSB path writes a PSB container

- **WHEN** a document is saved to a `.psb` path
- **THEN** the written file is a version-2 PSB

#### Scenario: A completed save writes the snapshot and clears modified [dl_save_current]

- **WHEN** a modified document is saved through the worker and the document does not change while it runs
- **THEN** the file reopens with the snapshot's layers and composite, the recorded path is the saved path, and the document is unmodified

#### Scenario: An edit during a save leaves the document modified [dl_save_stale]

- **WHEN** the document's history or content revision changes while the save worker runs
- **THEN** the file holds the saved snapshot, the recorded path is the saved path, and the document remains modified

#### Scenario: A failed write leaves the document untouched [dl_save_failed]

- **WHEN** the target cannot be written
- **THEN** the save returns false, the path and modified state are unchanged, and no temporary file remains

#### Scenario: A replaced document keeps its path and dirty state [dl_save_replaced]

- **WHEN** the document is replaced while the save worker runs
- **THEN** the file holds the original document's snapshot, and the replacement document keeps its own path and modified state

### Requirement: Modified state tracking

The system SHALL expose `is_dirty()` for the active document. Every successful
mutating command SHALL mark the document modified. Saving SHALL establish the
document as unmodified at the current history state only when the saved
document's history identity and its recorded content revision are still current;
a save that sees a later edit SHALL leave the document modified, and a save
whose document was replaced SHALL leave the replacement unchanged. Opening, New, and a successful Revert
SHALL establish the document as unmodified at the current history state. Undo,
redo, history jump, and named-snapshot restore SHALL mark the document modified
unless the resulting history state is the state recorded as saved. Returning to
the saved state SHALL clear the modified state. When the saved state is no
longer retained by the bounded history, the document SHALL report modified. A
rejected command SHALL NOT change the modified state. A view with no document
SHALL report unmodified.

#### Scenario: A mutating command marks modified

- **WHEN** a filter, adjustment, document op, or selection command succeeds
- **THEN** the document reports modified

#### Scenario: Save clears modified

- **WHEN** a modified document is saved successfully and does not change while the save runs
- **THEN** the document reports unmodified

#### Scenario: Open clears modified

- **WHEN** a document is opened
- **THEN** it reports unmodified

#### Scenario: History navigation marks modified [ml_history_navigation]

- **WHEN** a saved document is edited and then navigated by undo, redo, history jump, or named-snapshot restore to a state other than the saved state
- **THEN** the document reports modified

#### Scenario: Returning to the saved state clears modified [ml_return_to_saved]

- **WHEN** a saved document is edited and history navigation returns to the saved state
- **THEN** the document reports unmodified

#### Scenario: A dropped saved state stays modified [ml_saved_state_dropped]

- **WHEN** enough new edits or bounded-history drops remove the saved state from the retained history
- **THEN** the document reports modified and no navigation can return it to unmodified

## ADDED Requirements

### Requirement: Native saves run off the GUI thread

After the destination and format are chosen, a native `.psd` or `.psb` save
SHALL clone the document and perform the composite refresh, encode, and atomic
write on a worker thread. The GUI thread SHALL keep processing events and SHALL
NOT accept user input until the save lands; a busy indicator SHALL appear when
the write outlasts 250 ms. The worker SHALL NOT touch the live document. A save
through the synchronous API SHALL write the same bytes as the worker path.

#### Scenario: The event loop stays live during a save [dl_save_worker]

- **WHEN** a native save is held in flight by the test seam
- **THEN** a repeating timer keeps firing with no gap over 50 ms and the canvas keeps repainting

#### Scenario: The synchronous API writes the worker bytes [dl_save_sync_bytes]

- **WHEN** two identical documents are saved, one through the synchronous API and one through a held worker save
- **THEN** the two files are byte-identical
