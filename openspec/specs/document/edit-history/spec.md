# edit-history Specification

## Purpose
Snapshot capture before each mutating command, undo and redo to bit-identical state, bounded to 20 states.

## Requirements

### Requirement: Snapshot capture on mutating commands

The system SHALL capture a snapshot of the full editing state — the loaded `Document` and the active `Option<Selection>` — before each successful mutating command. The commands that capture SHALL include: `apply_filter`, `add_adjustment`, `remove_layer`, `set_layer_visible`, `resize_image`, `resize_canvas`, `rotate_doc`, `flip_doc`, `select_all`, `deselect`, `magic_wand`, `trim_image`, and `revert`. A command that fails validation or whose engine call errors SHALL NOT capture, leaving the history stack unchanged. A listed command that records a state on a document that already has history SHALL append it to that history and SHALL NOT replace or clear the history, its cursor, or its named snapshots.

#### Scenario: Successful command adds a history state

- **WHEN** any listed command completes successfully
- **THEN** the pre-command document and selection are recoverable via undo and the history depth increased by one

#### Scenario: Failed command leaves history untouched

- **WHEN** a command is rejected (invalid parameters, unknown kind, no document loaded)
- **THEN** the history depth and stack contents are unchanged

#### Scenario: A document-preserving command appends [eh_append_existing]

- **WHEN** a listed command records on a document that already has history
- **THEN** the prior states and named snapshots remain, the new state is current, and undo returns to the pre-command state

### Requirement: Undo and redo restore the captured state

The system SHALL provide `undo() -> bool` and `redo() -> bool`. `undo()` SHALL
restore the most recent captured snapshot (document and selection), decrement the
history position, and return `true`; with no state to undo it SHALL return
`false` and leave the document untouched. `redo()` SHALL re-apply the state that
was undone and return `true`; with nothing to redo it SHALL return `false`. Both
SHALL leave the document bit-identical to the snapshot being restored. Both SHALL
restore the displayed image from the restored document's `composite` buffer
without running a full composite, unconditionally — there is no colour-plane or
layer-count fallback. The restored display MUST be byte-identical to a full
recomposite of the restored document; across a GPU/CPU backend change it MAY
differ by at most 1 LSB per channel, consistent with the existing GPU parity
contract.

#### Scenario: Undo reverts a filter bit-exactly

- **WHEN** a filter is applied to a document and `undo()` is called
- **THEN** it returns `true` and every document field, including the composite and the selection, equals its pre-filter value

#### Scenario: Redo re-applies the undone op bit-exactly

- **WHEN** `undo()` succeeded and `redo()` is called
- **THEN** the document equals its post-filter value

#### Scenario: Undo and redo at the stack ends are no-ops

- **WHEN** `undo()` is called with an empty history, or `redo()` is called with no undone state
- **THEN** each returns `false` and the document is unchanged

#### Scenario: Undo restores the display without a full composite

- **WHEN** any document is edited and `undo()` is called
- **THEN** the displayed image is restored from the snapshot's composite buffer, no full composite runs, and the display equals a full recomposite of the restored document

### Requirement: Bounded depth of 20 states

The history SHALL retain at most 20 captured states, matching the Photoshop CS6 default. When a capture occurs on a full stack, the oldest state SHALL be dropped. Redo information SHALL be discarded when a new successful mutating command captures after one or more undos.

#### Scenario: Oldest state falls off the bounded stack

- **WHEN** 21 successful mutating commands have captured snapshots in sequence
- **THEN** the first command's state is no longer recoverable via undo, and `undo()` returns the state before the second command

#### Scenario: New op invalidates redo

- **WHEN** after one or more `undo()` calls a new successful mutating command runs
- **THEN** `redo()` returns `false` for the previously undone state

### Requirement: History resets on document open

Opening a document (`PictureView::open`) SHALL clear the history stack regardless of whether the load succeeds, so undo never crosses a file boundary.

#### Scenario: Undo is unavailable after opening

- **WHEN** a document is opened after any number of commands
- **THEN** `can_undo()` is `false` and `undo()` returns `false`

### Requirement: App controls and accessors

The system SHALL expose `can_undo() -> bool`, `can_redo() -> bool`, and `history_depth() -> i32` on `PictureView`, and the Qt shell SHALL provide Undo and Redo controls wired to `undo()`/`redo()` with Ctrl+Z and Ctrl+Y shortcuts, disabled when the corresponding availability accessor is `false`, refreshing after every state change.

#### Scenario: Controls track availability

- **WHEN** the stack is empty, after a captured op, and after an undo
- **THEN** the Undo control is disabled, enabled, and enabled-with-Redo-enabled respectively

### Requirement: Headless self-test coverage

The app `--self-test` SHALL exercise undo/redo end-to-end: capture the image before a mutating command, run it, undo, and assert the displayed image is bit-identical to the pre-command image; redo and assert bit-identical to the post-command image; and assert a following command invalidates redo.

#### Scenario: Self-test proves undo and redo in the app

- **WHEN** the app runs `--self-test` with the layered fixture
- **THEN** the undo/redo assertions pass and the self-test exits successfully

### Requirement: Composite kept current on the canvas rebuild

The system SHALL persist the rendered result into the document's `composite`
buffer whenever the canvas is rebuilt — a full recomposite or a region refresh.
An RGB document's stored composite SHALL be the rendered 4-plane RGBA frame, so
its merged composite is RGBA after any rebuild. A non-RGB mode
(Grayscale/Bitmap/Duotone/CMYK/Lab) SHALL keep its existing composite
colour-plane count, copying `min(rendered.channels, composite.channels)` planes,
so a 1-plane grayscale composite stays 1-plane. When the document's composite
dimensions no longer match the document — after a resize, crop, rotate, flip, or
canvas-size change — the system SHALL replace the composite with the rendered
result rather than patching it. A region refresh MUST leave every pixel outside
its region with its previous value. The stored composite MUST be derived from the
same rendered buffer that is converted for display, so the document composite and
the displayed image cannot diverge.

#### Scenario: A full recomposite stores the rendered result

- **WHEN** a filter, adjustment, blend, opacity, visibility, reorder, or delete mutation rebuilds the canvas in full
- **THEN** `doc.composite` equals the rendered composite that was displayed

#### Scenario: An RGB composite is stored as RGBA

- **WHEN** an RGB document's canvas is rebuilt
- **THEN** its composite has four colour planes and equals the rendered RGBA frame

#### Scenario: A region refresh patches only its region

- **WHEN** a region refresh composites a clamped rectangle
- **THEN** the pixels inside the rectangle are written from the rendered region, every pixel outside it keeps its previous value, and the composite's colour-plane count is unchanged

#### Scenario: A dimension change replaces the composite

- **WHEN** a resize, crop, rotate, flip, or canvas-size change rebuilds the canvas and the composite's dimensions no longer match the document
- **THEN** the composite is replaced with the rendered result at the new document dimensions

#### Scenario: A grayscale composite stays one plane

- **WHEN** a grayscale document is edited and its canvas rebuilt
- **THEN** its composite remains a single colour plane and the stored values are the rendered colour plane

### Requirement: Current composite at history capture

The history snapshot captured for a mutating command SHALL carry a document whose
`composite` buffer is current — equal to the rendered result of that document.
A mutating operation SHALL apply its region refresh or full recomposite before it
records the snapshot. The history order, labels, cursor movement, depth bound, and
redo truncation MUST otherwise be unchanged, and the snapshot SHALL continue to
carry the post-command document state.

#### Scenario: A captured snapshot carries a current composite

- **WHEN** any mutating command completes and its state is captured
- **THEN** the captured document's composite equals a full recomposite of the captured document

#### Scenario: Capture ordering does not change history order

- **WHEN** a mutating command runs after one or more undos
- **THEN** the captured state is still the post-command state, redo is discarded as before, and the labels and depth bound are unchanged

### Requirement: Restoring a named snapshot records a history state

Restoring named snapshot `i` SHALL install that snapshot's document and selection, restore the layer target captured with it, and append exactly one current history state labelled with the snapshot's label. The pre-restore document SHALL remain the previous state, so `undo()` SHALL return to the pre-restore document and `redo()` SHALL return to the snapshot. The new state SHALL follow the normal history rules: the cursor SHALL move to it, a later capture after undo SHALL discard redo, and the 20-state bound SHALL apply. Restoring a snapshot SHALL cancel a live session before installing the snapshot.

#### Scenario: Undo returns to the pre-restore state [eh_snapshot_undo]

- **WHEN** a named snapshot is restored while the document has a later state
- **THEN** exactly one state is appended, the history cursor is the appended state, and undo restores the pre-restore document and selection

#### Scenario: Redo returns to the snapshot [eh_snapshot_redo]

- **WHEN** the undo of a snapshot restore succeeds and redo is called
- **THEN** the document, selection, and layer target equal the restored snapshot

#### Scenario: Restoring after undo discards redo [eh_snapshot_after_undo]

- **WHEN** a document is edited past a snapshot, undone once, and the snapshot is restored
- **THEN** exactly one state is appended after the undone states are discarded, undo returns to the state that was current before the restore, and redo returns to the snapshot

### Requirement: Trim records one state on the existing history

`Image > Trim` SHALL crop the live document in place and record exactly one
history state labelled `Trim` on the existing history. It SHALL refuse without
mutating while a live edit session or a pending opacity, fill, or mask-refine
preview is active. When the visible content already fills the canvas or the
composite is empty, Trim SHALL record nothing and leave the document, selection,
history, and dirty state unchanged.
 It SHALL keep the
active layer target when it still resolves and otherwise select the top layer.
On success it SHALL clear the selection because its coordinates no longer map.
It SHALL NOT replace the document, reset the history, or discard named
snapshots. Undo SHALL restore the pre-Trim document, selection, and history
depth.

#### Scenario: Trim is undoable [eh_trim_undoable]

- **WHEN** a document with prior edits, a named snapshot, an active layer target, and a selection is trimmed and undo is called
- **THEN** the pre-Trim document, selection, history depth, and layer target are restored, the prior states and snapshot remain, and the Trim state is no longer current

#### Scenario: A trim with nothing to remove records nothing [eh_trim_noop]

- **WHEN** Trim runs on a document whose visible content already fills the canvas, or on an empty composite
- **THEN** no history state is added and the document, selection, dirty state, and layer target are unchanged
