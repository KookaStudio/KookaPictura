# Document lifecycle delta

## MODIFIED Requirements

### Requirement: Save and Save As

The system SHALL write the active document in its current output format, chosen
by the save path's extension, writing atomically so a failed write never
truncates an existing file. A `psd` path SHALL write the document's composite
and layer tree through the PSD codec; a `psb` path SHALL force a version-2 PSB
container through the codec. A supported raster path (PNG, JPEG, TIFF, WebP,
BMP, or GIF) SHALL write the flattened composite: PNG, JPEG, TIFF, WebP, and BMP
through the Qt-boundary raster encode edge, and GIF through the engine GIF89a
encoder (`interop/gif-save`). Save As SHALL default its format to PSD unless the
document is a single pixel or background layer imported from a supported raster,
in which case it SHALL default to that source format. When the document's
recorded path is a raster format and the document has more than one layer, a
group, an adjustment, or a type layer, Save SHALL behave as Save As rather than
silently flattening into that raster file. Saving a document that has more than
one layer, a group, an adjustment, or a type layer to a format that cannot hold
them SHALL warn before writing and then write the flattened composite. The
`composite` serialized to PSD SHALL be the active document's current rendered
composite, so a save after any edit round-trips to the edited pixels rather than
to a pre-edit merged image. The `write_psd` byte layout and its use of
`doc.composite.channels` to derive the PSD colour-plane count MUST NOT change; an
RGB document's merged composite is RGBA (four planes) after a canvas rebuild,
while a grayscale document keeps its composite colour-plane count. Save on an
untitled document SHALL behave as Save As. On success the save SHALL record the
file path. It SHALL clear the modified state only when the document is the same
document that was snapshotted when the write started — the same history
identity, content revision, native persisted-state identity, and document
generation — and that snapshot is still current. A document changed after the
snapshot SHALL keep the recorded path and remain modified. When the document generation changed during the save (the
document was replaced), the replacement document's path and modified state SHALL
be unchanged. On a pre-publication failure the path and modified state SHALL
be unchanged and no temporary file SHALL remain. An unconfirmed publication
SHALL retain the path and establish the protection obligation specified below.

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
mutating command SHALL mark the document modified. A native saved-state identity
SHALL include both history state and persisted session metadata, including
Vanishing Point planes, even where a metadata change creates no history entry
or content revision. Save completion SHALL also compare the snapshot content
revision to the live content revision before clearing modified. Saving records
the snapshot identity; history navigation
SHALL clear modified only if both the resulting history state and current
persisted metadata match the saved identity and there is no outstanding
publication protection obligation. A later metadata-only edit remains modified
across navigation. Opening, New, and successful Revert establish an unmodified
state. A dropped saved history state remains modified. A rejected command SHALL
NOT change modified state. A view with no document SHALL report unmodified.

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

### Requirement: Unsaved-changes prompt
The system SHALL prompt Save / Discard / Cancel before closing, reverting, or
replacing a modified document or one with an outstanding publication protection obligation. Cancel SHALL abort the operation and leave the
document open and modified.

#### Scenario: Discard closes without saving
- **WHEN** the user chooses Discard in the prompt
- **THEN** the document closes and no file is written

#### Scenario: Save then close
- **WHEN** the user chooses Save in the prompt
- **THEN** the document is saved and then closed

#### Scenario: Cancel aborts
- **WHEN** the user chooses Cancel
- **THEN** the document remains open and modified


### Requirement: Native saves run off the GUI thread

After the destination and format are chosen, a native `.psd` or `.psb` save
SHALL capture an immutable document snapshot and perform the composite refresh,
encode, and atomic write on a worker thread. The worker SHALL NOT touch the live
document. The GUI thread SHALL keep processing events. Interactive Save and
Save As SHALL accept editing input while Save in Background is enabled.
Synchronous callers SHALL wait for completion and retain their completion
result and input exclusion. With Save in Background disabled, interactive
saves SHALL also wait, exclude input, and show a busy indicator for a write
that outlasts 250 ms. Synchronous and background saves of identical inputs
SHALL write identical bytes.

#### Scenario: The event loop stays live during a save [dl_save_worker]

- **WHEN** a native save is held in flight by the test seam
- **THEN** a repeating timer keeps firing with no gap over 50 ms and the canvas keeps repainting

#### Scenario: The synchronous API writes the worker bytes [dl_save_sync_bytes]

- **WHEN** two identical documents are saved, one through the synchronous API and one through a held worker save
- **THEN** the two files are byte-identical

## ADDED Requirements

### Requirement: Background save preference and progress

File Handling SHALL expose Save in Background, enabled by default and persisted
across launches. Changing it SHALL apply to subsequent saves. Automatically
Save Recovery Information SHALL operate independently. During an interactive
background save the originating document tab SHALL display a percentage and
its lower-left status area SHALL show a percentage and progress bar when that
document is visible. Progress SHALL reflect completed phases, stay below 100
until the write succeeds, and clear on completion or failure. Switching tabs
SHALL keep each document's progress associated with that document.

#### Scenario: The background save preference persists [bs_preference]

- **WHEN** a fresh session opens File Handling, turns Save in Background off, and relaunches
- **THEN** the initial setting is on, it reloads as off, an interactive native save uses the existing waiting indicator, and the recovery preference and interval are unchanged

#### Scenario: Input and progress follow the document [bs_input_progress]

- **WHEN** the user starts a held native save, edits that document through the UI, and switches to and edits another document
- **THEN** both edits are accepted, progress is shown only for the originating document, and the progress indicators clear when the job finishes

### Requirement: Background save completion is bound to the saved document

A background save SHALL apply path, dirty-state, recent-file and action
recording effects to its original transaction when appropriate. A stale
success on the same document SHALL update its path and identify the written
history state as the saved state. It SHALL leave newer live work dirty,
including a live state that was clean relative to the previous file. A
completion for a replaced document SHALL leave the replacement's path and
dirty state unchanged. The application SHALL serialize writes to each
destination within the instance. A second interactive save for a document
with an outstanding job or to an outstanding job's destination SHALL be
refused with a save-in-progress notice.

A native save SHALL sync its temporary file and the destination directory
before reporting durable success. If publication fails, the live path and
dirty state SHALL stay unchanged on a pre-rename failure and a failure notice
SHALL identify the save. If rename succeeded but directory sync failed, the
notice SHALL indicate uncertain publication and the document SHALL acquire a
sticky protection obligation: it SHALL be treated as needing an unsaved-changes
prompt despite history navigation or its previous clean state. With recovery
enabled, retain or create recovery data for the latest committed live state,
including a state formerly clean, until a later durable current-state save or
an explicit Discard resolves the obligation. A successful Revert resolves it
only after the unsaved-changes decision. The path remains unchanged.

#### Scenario: Save As completes after a newer edit and a tab switch [bs_stale_save_as]

- **WHEN** the user starts native Save As for revision A, edits to B, and selects another document while the write is held
- **THEN** the destination contains A, the originating document has the new path and stays dirty at B, and the selected document is unaffected

#### Scenario: Returning to the previous saved state during a save [bs_previous_saved_state]

- **WHEN** the user saves edited revision A and undoes to the previously saved state before A lands
- **THEN** the destination contains A, the live state is dirty, and returning to the written history state clears dirty

#### Scenario: Competing saves respect document and destination [bs_competing_saves]

- **WHEN** a save is held and a user attempts another save of that document or a save to its destination from another tab
- **THEN** the conflicting requests show a save-in-progress notice and only the first snapshot is written

#### Scenario: An unsuccessful write leaves the live document intact [bs_durable_failure]

- **WHEN** a native save fails before rename or its destination directory sync fails after rename
- **THEN** the original remains intact on a pre-rename failure, the pre-rename failure preserves path, dirty state, and recovery; the post-rename failure preserves the path, reports uncertain publication, and protects the live state

#### Scenario: An uncertain publication protects the previously clean state [bs_uncertain_clean]

- **WHEN** save A is held, the user undoes to previously clean B, rename succeeds and directory sync fails, and the user later closes
- **THEN** the application retains or creates recovery for B, prompts even after history navigation, Cancel preserves B, and a later durable save of B or explicit Discard resolves the obligation

#### Scenario: A metadata-only change during a save stays modified [bs_metadata_stale]

- **WHEN** save A is held and the user commits only a Vanishing Point plane change B, then the save completes and history is navigated away and back
- **THEN** the file contains A and the live document remains modified with B until B is durably saved

### Requirement: Background save action recording preserves dispatch order

When recording is active at dispatch, a background save SHALL reserve its position
in that recording session. A successful save SHALL fill that position with the
original command and destination even if later commands were recorded or recording
was stopped or switched before completion. Failure SHALL remove the pending position
without recording a save step. Recording playback SHALL preserve dispatch order and
SHALL NOT execute an unresolved pending save step.

#### Scenario: Recording survives background completion and failure [bs_recording_order]

- **WHEN** a held save is dispatched while recording, a later command is recorded, recording is stopped or switched, and the save succeeds; repeat with a failed save
- **THEN** replay of the original recording runs the successful save before the later command with its original destination, the switched recording receives no save step, and the failed save contributes no step

### Requirement: Destructive actions resolve outstanding saves

Close, Revert, quit, and restart SHALL resolve any applicable pending save
before deciding whether to prompt for unsaved changes. After a successful
write they SHALL recheck the affected document. After a failed write they
SHALL keep it open unless the user explicitly discards the changes.
Choosing Save in an unsaved-changes prompt SHALL complete the save before
the operation proceeds and SHALL recheck the document for unsaved changes.

#### Scenario: Close rechecks work after the pending save [bs_close_recheck]

- **WHEN** the user requests close while a save of A is pending and the live document contains newer B, or the pending write fails
- **THEN** the application resolves the write before the close decision, prompts for the remaining unsaved work, and keeps the document open on Cancel

#### Scenario: Quit, restart, and Revert resolve pending writes [bs_lifecycle_recheck]

- **WHEN** the user invokes quit, restart, or Revert during an outstanding save for an affected document
- **THEN** the operation waits for the write outcome and the unsaved-changes decision, and Cancel leaves the document open with its edits
