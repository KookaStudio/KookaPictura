# document/document-session Specification

## Purpose
Define how one document session owns the open document, its history, and every
in-progress edit gesture, so replacing or restoring the document, navigating
history, or switching tools always leaves a coherent document with no stale
session state.

## Requirements

### Requirement: At most one live edit session

The app SHALL have at most one Rust-owned live edit session at a time. A live
edit session is an in-progress gesture that can change the document or selection
on commit: paint or mask stroke, healing stroke, lasso or magnetic selection,
selection move, transform, path gesture, filter preview, or mode preview. The
Type tool's editing session remains C++-owned and is reset by the in-place
restore hook; it is not part of this slot.

A begin of a different kind while a session is live SHALL be refused without
changing the document or the active session. A repeated begin of a stroke,
healing, selection move, or path gesture while that same session is live SHALL
be refused. Kinds whose contract is an update SHALL update the existing session
instead of creating a second one: a filter preview or mode preview call changes
the live preview's parameters, `magnetic_begin` creates the selection gesture
and a following `begin_lasso` attaches its points to it, and `begin_paint`
resolves the layer or mask target before acquiring the slot rather than nesting
a second stroke begin.

A transform on the same path and mode SHALL be a no-op; a transform on the same
path with a different mode, or on a different target, SHALL cancel the live
transform and start the new one as `document/free-transform` requires. A refused
begin SHALL leave the document bit-identical to its state before the call.

#### Scenario: A second session is refused [ds_one_slot]

- **WHEN** a healing gesture is live and a paint stroke is begun
- **THEN** the paint stroke returns false, the healing gesture remains active, and the document and history are unchanged

#### Scenario: Update begins keep one session [ds_update_begin]

- **WHEN** a filter preview or mode preview is updated repeatedly, or a magnetic lasso begins and a lasso begin follows to attach its points
- **THEN** exactly one session remains live, its parameters or points are updated, and no second session is created

#### Scenario: A transform on another target replaces the live transform [ds_replace_target]

- **WHEN** a Free Transform session is live on one layer and a transform is begun on a different transformable layer
- **THEN** the first session ends without adding history, exactly one transform session is live on the second layer, and the second layer's pre-trim state is the only document change

#### Scenario: A transform mode switch replaces the live transform [ds_transform_mode_switch]

- **WHEN** a Free Transform session is live on a path and a Skew, Distort, or Perspective session is begun on the same path
- **THEN** the first session ends, exactly one session is live in the new mode, and no history state is added

#### Scenario: A failed transform begin leaves the layer unchanged [ds_failed_transform_begin]

- **WHEN** a transform begin trims the target and then fails to build its preview or read its rectangle
- **THEN** the layer's pre-trim rectangle is restored, no session is live, and no history state is added

### Requirement: Document replacement cancels first and installs coherent incoming state

When Revert, recovery restore, Open, New, Open Image, Open As Smart Object, or
Duplicate installs a document, the app SHALL cancel the live session before
installing it, so cancellation can only affect the outgoing document. It SHALL
clear the outgoing selection, the prior-selection memory used by Reselect, the
outgoing history, the mask edit target, the active path selection, any path
gesture, pending lasso points and edge map, filter and mode previews, the ruler,
pending opacity/fill/mask-refine previews, the canvas proxy, and the stored PDF
options. It SHALL bump the document generation so document-derived caches cannot
serve the incoming document.

The incoming PDF password SHALL be installed when the incoming document is a
protected PDF opened or reverted, and cleared for New, Duplicate, or a non-PDF
incoming document. The installed document's history SHALL be the incoming Open
or Revert state, exactly one New state, or exactly one Duplicate state. An
in-flight open or filter result belonging to an outgoing document SHALL be
discarded by request or generation identity.

#### Scenario: A live session cannot leak into the new document [ds_replace_cancels]

- **WHEN** a paint stroke, mask stroke, healing gesture, lifting transform, filter preview, or mode preview is live and Revert, New, or Open installs a document
- **THEN** the installed document contains none of the outgoing session's pixels, no edit session is live, and the outgoing handler state cannot commit into it

#### Scenario: Outgoing pointers, previews, and caches are cleared [ds_replace_clears]

- **WHEN** a document with a mask edit target, a selected saved path, a ruler measurement, pending opacity/fill/mask-refine previews, a canvas proxy, and PDF options is replaced by Revert or New
- **THEN** the mask edit target and active saved path are unset, the ruler and pending previews are gone, the document generation changed, and the canvas proxy cannot serve the incoming document

#### Scenario: PDF password follows the incoming document [ds_pdf_password]

- **WHEN** New or Duplicate follows a protected PDF, or a protected PDF is opened or reverted
- **THEN** New and Duplicate clear the stored password, while the PDF open or Revert installs the incoming password so a later Revert can reopen the file

#### Scenario: Reselect is unavailable after a same-size replacement [ds_reselect_after_replace]

- **WHEN** a selection is made and deselected, then a replacement document of the same dimensions is installed, and Reselect is queried or applied
- **THEN** the query reports no prior selection, applying Reselect changes nothing, and the installed document has no selection

#### Scenario: Replacement installs the incoming history [ds_replace_history]

- **WHEN** Open/Revert, New, or Duplicate installs a document
- **THEN** the history contains exactly the incoming Open/Revert state, one New state, or one Duplicate state respectively, and that single initial state is the only undo target

#### Scenario: A stale async install is discarded [ds_stale_async]

- **WHEN** an open or filter installation carries a generation or request id that no longer matches the current document
- **THEN** the late result is dropped, the installed document and history are unchanged, and no session state is created

#### Scenario: Replacement keeps the entry point's signal policy [ds_replace_signals]

- **WHEN** Open/Revert, New, or Duplicate installs a document with no live session
- **THEN** the entry point emits the same full-change or blit signal it emits today, and the replacement itself adds no second history capture or notification

### Requirement: History navigation cancels first and resets tool handlers

Undo, redo, history jump, and snapshot restore SHALL cancel the live session and
clear the pending preview flags before restoring state. The restored document and
selection SHALL equal the target history state exactly; a cancellation's
document restoration MUST NOT survive the restore. If cancellation reports a
display change, the history restore SHALL request a full display rebuild for the
restored state rather than a partial region. In-place replacement and history
restore SHALL also reset every C++ tool handler's local gesture, press view,
polygon, content-move, whole-layer move/clone, and typing state, so a later
pointer release cannot commit into the restored document.

#### Scenario: Undo during any session restores the history state [ds_undo_cancels]

- **WHEN** Undo runs while a paint stroke, lifting transform, filter preview, or mode preview is live, or a pending opacity/fill/mask-refine preview exists
- **THEN** the session and pending previews are cleared, the document and selection equal the undo target, the history index is that target, and no session state remains

#### Scenario: Redo cancels the active session [ds_redo_cancels]

- **WHEN** Redo runs while a live session is active
- **THEN** the session ends, the redo target is installed, and no session state remains

#### Scenario: Jump cancels the active session [ds_jump_cancels]

- **WHEN** a history jump target is selected while a filter preview is open
- **THEN** the preview and pending preview flags are discarded against the outgoing state, the jumped-to state is installed, and the preview baseline is gone

#### Scenario: Snapshot restore cancels the active session [ds_snapshot_cancels]

- **WHEN** a named snapshot is restored while a paint stroke or transform is live
- **THEN** the session ends, the snapshot's document and selection are installed, and no session state remains

#### Scenario: In-place restore resets C++ handler state [ds_inplace_handlers]

- **WHEN** Revert or Undo happens while a Pen anchor, Path Selection drag, content move, whole-layer move/Alt-clone, or Type edit is in progress
- **THEN** the handler drops its press view, gesture, polygon, move/clone, and typing state, and a later release commits nothing into the restored document

### Requirement: Cancelling a session restores state and records nothing

Cancelling a live session SHALL clear the session, leave the document and
selection bit-identical to their values when the session began, leave the dirty
flag and history unchanged, and add no history state. A path gesture that edits
a shape layer or a Path Selection drag SHALL restore the pre-gesture document,
not only the Work/Saved path backup. Cancelling when no session is live SHALL
be a no-op that reports no change.

#### Scenario: Cancel restores the pre-session state [ds_cancel_restores]

- **WHEN** a paint stroke, mask stroke, selection move, transform (including a lifted selection), path gesture on a shape layer, Path Selection drag, filter preview, or mode preview is cancelled
- **THEN** the document and selection equal their pre-session values, no session is live, and the history count and index are unchanged

#### Scenario: Cancel records no history [ds_cancel_no_history]

- **WHEN** any live session is cancelled after changing document pixels or selection coverage
- **THEN** the document is dirty only if it was dirty before the session, and no history label is added

#### Scenario: Cancel without a session is a no-op [ds_cancel_noop]

- **WHEN** cancel is invoked with no live session and the document already at rest
- **THEN** the document, selection, history, dirty flag, and display report are unchanged

### Requirement: Switching tools ends the live session according to its contract

Changing the active tool SHALL end the live session before the new tool receives
input, and the new tool SHALL begin with no Rust session slot. A cancellable
gesture (healing, lasso, magnetic, transform, selection move) SHALL be
cancelled and the document restored. A gesture whose tool contract commits on
deactivate (Brush, Pen, Path Selection, Type) SHALL be committed by the outgoing
handler before the new tool starts, preserving the documented behaviour.

#### Scenario: Switching tools cancels a cancellable gesture [ds_tool_switch_cancel]

- **WHEN** a healing gesture, lasso, magnetic gesture, transform, or selection move is live and the user selects another tool
- **THEN** the gesture is cancelled and the document is restored, and the new tool has no live session

#### Scenario: Switching tools commits a commit-on-deactivate gesture [ds_tool_switch_commit]

- **WHEN** a Brush, Pen, Path Selection, or Type gesture is in progress and the user selects another tool
- **THEN** the outgoing handler commits exactly the state it documents, the Rust slot is cleared, and the new tool starts with no session

### Requirement: The change report drives display and notification

Session and edit operations SHALL report what changed: no display change, one or
more bounded document regions, or a full rebuild, plus whether a full change
notification is required. A bounded region MAY carry the straight-sRGB buffer
produced by the session, or leave it absent for the adapter to composite. The
Qt adapter SHALL emit the region blit(s) for bounded regions, a full change
notification for a rebuild, and a notification only when the flag is set. The
report MUST preserve the existing signal counts: a bounded region does not by
itself imply a full change notification, a bounded region MAY require one
(healing, patch, red-eye, and batch-property commits do), and a metadata,
selection, or Work/Saved path restoration can notify without a pixel change.
An empty region list with `Blit` emits no blit.

#### Scenario: A bounded change blits without a full notification [ds_report_region]

- **WHEN** a paint or mask stroke is cancelled or committed with bounded affected rectangles and no notification flag
- **THEN** the adapter emits the region blit(s) and does not emit a full change notification

#### Scenario: A bounded change can also notify [ds_report_region_notify]

- **WHEN** a healing commit, patch, red-eye, or a layer-opacity/fill batch commit reports bounded regions and the notification flag
- **THEN** the adapter emits the region blit(s) and exactly one full change notification

#### Scenario: A notification-only change emits no blit [ds_report_notify]

- **WHEN** a Work or Saved path gesture is aborted, restoring the path and selection without changing document pixels
- **THEN** the report carries a notification but no display region, and the adapter emits a full change notification without a region blit

#### Scenario: A shape-layer path abort rebuilds [ds_report_shape_path]

- **WHEN** a path gesture that edited a shape layer's path is aborted and the pre-gesture document is restored
- **THEN** the report requests a rebuild or a bounded region and a notification, and the adapter does not treat it as notification-only

#### Scenario: A full change rebuilds and notifies [ds_report_rebuild]

- **WHEN** a mode preview is cancelled and the pre-preview document is restored
- **THEN** the report requests a full rebuild, the adapter rebuilds from the restored composite, and a full change notification is emitted

### Requirement: The document session is headless

The document session SHALL be constructible and drivable without the Qt object.
A Rust test MUST be able to begin a session, cancel it, replace the document,
and navigate history without a QObject, and the resulting document, selection,
and history SHALL be observable.

#### Scenario: The session runs without Qt [ds_headless_session]

- **WHEN** a test constructs a session, replaces its document with a known history, and undoes within that incoming history
- **THEN** every step completes without a QObject and the document, selection, and history match the expected values
