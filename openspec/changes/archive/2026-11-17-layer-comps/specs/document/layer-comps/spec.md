# Spec Delta

## Purpose

A document-level store of named layer-state snapshots that can be captured,
applied, updated, and validated without changing layer pixels.

## ADDED Requirements

### Requirement: A document owns an ordered list of layer comps

A document SHALL own an ordered list of layer comps plus the implicit Last
Document State. Each comp SHALL carry a stable comp id, a name, a comment, a
recorded-class set, and per-layer captured state. The store SHALL be document
metadata: it SHALL ride history snapshots, document clones, recovery, and PSD
saves. A clone of a document SHALL share the comp store until one side edits it.
A document with no comps SHALL have an empty list and no captured Last Document
State; the panel's pseudo-row remains visible but unavailable.

#### Scenario: A new document has no comps [lc_store_empty]

- **WHEN** a document is created or opened without layer comps
- **THEN** its comp list is empty, no Last Document State is captured, and the panel's Last Document State row is unavailable

#### Scenario: Clones share the comp store [lc_store_clone]

- **WHEN** a document with comps is cloned and the clone adds or renames a comp
- **THEN** the original document's comps are unchanged

#### Scenario: History restores the comp store [lc_store_history]

- **WHEN** a comp is created, the document is edited, and the edit is undone
- **THEN** the comp list matches its pre-edit state

### Requirement: A comp records visibility, position, and appearance

Each comp SHALL record any combination of three classes: layer visibility,
layer position, and layer appearance. Appearance SHALL mean the layer's blend
mode — including a preserved unknown 4-byte blend key — and, when the layer
carries an `lfx2` layer style, whether that style's master switch is on. A
class that is not recorded SHALL NOT be captured for that comp and SHALL NOT be
applied by it. Smart Filter settings MUST NOT be part of a comp.

#### Scenario: A visibility-only comp stores no position or appearance [lc_record_visibility_only]

- **WHEN** a comp is created with only Visibility checked
- **THEN** changing a layer's position, blend mode, or style and applying the comp leaves that field unchanged

#### Scenario: An all-class comp stores every class [lc_record_all]

- **WHEN** a comp is created with Visibility, Position, and Appearance checked
- **THEN** applying it restores visibility, position, blend mode, and style state

#### Scenario: Smart Filters do not vary by comp [lc_record_smart_filter]

- **WHEN** a layer has a Smart Filter and different comps are applied
- **THEN** the Smart Filter settings and rendered effect stay the same in every comp

### Requirement: Capture creates a comp from the current layer state

Creating a comp SHALL capture the current state of every layer present at that
moment for the comp's recorded classes. The comp SHALL receive the requested
name and comment; an empty name SHALL fall back to the next default
`Layer Comp N`. Creation SHALL assign a stable identity to any layer that lacks
one before capturing. Creation SHALL record exactly one undoable metadata state
and SHALL NOT change pixels or the composite. Cancelling the New Layer Comp
dialog SHALL leave the document, history, and modified state unchanged.

#### Scenario: Capture reflects the current visibility [lc_capture_visibility]

- **WHEN** layers A, B, and C are visible or hidden in some combination and a visibility-recording comp is created
- **THEN** applying that comp later restores exactly that combination

#### Scenario: Cancelling creation changes nothing [lc_capture_cancel]

- **WHEN** the New Layer Comp dialog is opened and cancelled
- **THEN** the comp list, pixels, modified state, and history are unchanged

#### Scenario: Creation is one undo state [lc_capture_undo]

- **WHEN** a comp is created and undo is invoked
- **THEN** the comp disappears and the document is otherwise unchanged

### Requirement: Applying a comp restores only its recorded classes

Applying a comp SHALL write the captured values back to the layers that still
exist, addressed by stable layer identity. A captured layer that no longer
exists SHALL be skipped and SHALL mark the comp with a warning. Applying SHALL
recomposite and record exactly one undoable state. Layer pixel data SHALL NOT
change. A comp whose entries are all missing SHALL change nothing, record no
state, and SHALL NOT replace Last Document State.

#### Scenario: Visibility is restored [lc_apply_visibility]

- **WHEN** a document has `[A visible, B hidden, C visible]`, a visibility comp is created, A is hidden and B shown, and the comp is applied
- **THEN** the document is `[A visible, B hidden, C visible]` and one history state is added

#### Scenario: Position is restored [lc_apply_position]

- **WHEN** a position-recording comp is created, a layer is moved, and the comp is applied
- **THEN** the layer returns to its captured position within one document pixel

#### Scenario: Appearance is restored [lc_apply_appearance]

- **WHEN** an appearance-recording comp is created, a layer's blend mode or style switch is changed, and the comp is applied
- **THEN** the captured blend mode and style state return

#### Scenario: An unknown blend key is restored [lc_apply_unknown_blend]

- **WHEN** an appearance-recording comp captures a layer whose blend mode is an unrecognized 4-byte key, the key changes, and the comp is applied
- **THEN** the captured raw blend key returns

#### Scenario: Unrecorded position is untouched [lc_apply_unrecorded]

- **WHEN** a comp without Position recorded is applied after a layer move
- **THEN** the layer keeps its current position

#### Scenario: Applying changes no pixels [lc_apply_pixels]

- **WHEN** a comp recording only visibility is applied
- **THEN** every layer's pixel data is unchanged and the composite equals a fresh render of the restored document

#### Scenario: A comp with no resolvable entries is a no-op [lc_apply_empty]

- **WHEN** every layer referenced by a comp has been deleted and the comp is applied
- **THEN** the document and Last Document State are unchanged and no history state is added

#### Scenario: Repeated apply and undo are lossless [lc_apply_repeat]

- **WHEN** a position-recording comp is applied and undone ten times
- **THEN** after each pair every affected layer returns to the same position and no fractional drift accumulates

### Requirement: Update recaptures a comp

Updating a comp SHALL re-capture its recorded classes from the current document,
drop entries for layers that no longer exist, clear its warning state, and
record exactly one undoable metadata state. Update SHALL NOT change pixels.

#### Scenario: Update reflects the current state [lc_update]

- **WHEN** a comp is created, a recorded property changes, and Update is chosen
- **THEN** applying the comp later restores the changed value and one history state was added by Update

#### Scenario: Update clears a warning [lc_update_warning]

- **WHEN** a comp has a cannot-be-fully-restored warning and Update is chosen
- **THEN** the warning is gone and applying the comp no longer reports missing layers

### Requirement: Comp metadata is editable and undoable

The system SHALL support renaming a comp, editing its comment, changing its
recorded classes, duplicating it, and deleting it. Each SHALL record exactly one
undoable metadata state and SHALL NOT change pixels. Changing recorded classes
SHALL NOT recapture the comp until Update is invoked; the new record set governs
the next apply. Duplicating SHALL assign a new comp id and SHALL NOT reuse a
deleted comp's id. Deleting the last comp SHALL remove the comp list and its
persisted data.

#### Scenario: Delete removes the comp [lc_delete]

- **WHEN** a comp is deleted
- **THEN** it disappears from the list, one history state is added, and the remaining comps are unchanged

#### Scenario: Rename keeps the captured state [lc_rename]

- **WHEN** a comp is renamed
- **THEN** its captured state and entries are unchanged

#### Scenario: Options change the record set only [lc_options]

- **WHEN** Appearance is enabled in Layer Comp Options without Update
- **THEN** the existing captured state is unchanged until Update runs, and the next Apply uses the enabled class only if an entry holds it

#### Scenario: Duplicate copies the comp [lc_duplicate]

- **WHEN** a comp is duplicated
- **THEN** a new comp with a new id and the same captured state appears next to it

#### Scenario: Comp ids are not reused [lc_comp_id_reuse]

- **WHEN** the last comp is deleted, a new comp is created, and the document is saved and reopened
- **THEN** the new comp's id differs from every earlier comp's id

### Requirement: Last Document State restores the pre-apply state

Before each apply that changes the document, the system SHALL capture the
document's layer state across the three comp classes and keep it as Last
Document State. Restoring it SHALL return every layer that still exists to that
state and SHALL record exactly one undoable state. A no-op apply SHALL NOT
replace Last Document State. Last Document State SHALL be unavailable until a
change-making comp apply has happened in the session. It SHALL be a pseudo-row,
not a comp, and SHALL NOT be created, renamed, updated, or deleted as one.

#### Scenario: Restoring after an apply [lc_last_restore]

- **WHEN** a comp is applied and Last Document State is then applied
- **THEN** the document returns to its pre-comp-apply layer state and one history state is added

#### Scenario: Undo returns to the comp state [lc_last_undo]

- **WHEN** the Last Document State restore is undone
- **THEN** the document returns to the state the comp produced

#### Scenario: No state before the first apply [lc_last_unavailable]

- **WHEN** no comp has been applied in the session
- **THEN** Last Document State cannot be applied

#### Scenario: A no-op apply does not become Last Document State [lc_last_noop]

- **WHEN** a comp whose entries are all missing is applied
- **THEN** Last Document State still holds the state from the previous real apply, or remains unavailable if there was none

### Requirement: Missing layers create a clearable warning

A comp SHALL report a cannot-be-fully-restored warning when any captured layer
identity no longer resolves, including after a layer is deleted, merged, or
converted to Background. The warning SHALL be shown on the comp and SHALL be
clearable for the currently missing identities without changing the comp, the
remaining layers, or the document. Clearing all warnings SHALL clear every
comp's current warning and SHALL record exactly one undoable metadata state. A
cleared identity that later resolves again SHALL be pruned, so deleting it once
more raises the warning again. Warning state SHALL survive save and reopen.

#### Scenario: Deleting a captured layer warns [lc_warning_delete]

- **WHEN** a comp captures layer L and L is deleted
- **THEN** the comp shows a warning and its other entries remain intact

#### Scenario: Merging a captured layer warns [lc_warning_merge]

- **WHEN** a captured layer is merged into another layer
- **THEN** the comp shows a warning

#### Scenario: Converting to Background warns [lc_warning_background]

- **WHEN** a captured layer is converted to a Background
- **THEN** the comp shows a warning

#### Scenario: Clearing keeps the remaining entries [lc_warning_clear]

- **WHEN** the warning is cleared
- **THEN** the warning disappears and applying the comp still restores its resolvable entries

#### Scenario: A new loss warns again [lc_warning_new_loss]

- **WHEN** a warning is cleared, another captured layer is deleted, and the panel refreshes
- **THEN** the comp shows the warning again

#### Scenario: A relost identity warns again [lc_warning_relost]

- **WHEN** a warning is cleared, the layer is restored by undo, and that same layer is deleted again
- **THEN** the comp shows the warning again

#### Scenario: Clear All clears every comp [lc_warning_clear_all]

- **WHEN** two comps have warnings and Clear All Layer Comp Warnings is chosen
- **THEN** neither comp shows a warning and one history state is added

#### Scenario: Warnings survive save and reopen [lc_warning_persist]

- **WHEN** a warned comp is saved and the document is reopened
- **THEN** the comp still reports the same warning state and cleared identities

### Requirement: Layer identity is stable across edits

A comp SHALL address captured layers by a stable per-layer identity that
survives renames, reordering, and document clones. The identity SHALL be
assigned to a layer when a comp first needs it and SHALL be preserved by PSD
round-trips. A duplicated or pasted layer SHALL receive a new identity and
SHALL NOT inherit the source's captured comp state, so it is not captured by an
existing comp.

#### Scenario: Renaming a layer keeps its comp entry [lc_id_rename]

- **WHEN** a captured layer is renamed and the comp is applied
- **THEN** the renamed layer receives the captured state and no warning appears

#### Scenario: Reordering keeps comp entries [lc_id_reorder]

- **WHEN** captured layers are reordered and the comp is applied
- **THEN** the same layers receive their captured state

#### Scenario: A missing identity is assigned [lc_id_missing]

- **WHEN** a comp is created on a layer that has no stable identity
- **THEN** the layer receives an identity that does not collide with another layer's and the comp captures it

#### Scenario: Duplicating a layer isolates the copy [lc_id_duplicate]

- **WHEN** a captured layer is duplicated and the comp is applied
- **THEN** only the original layer is treated as captured; the copy keeps its current state

#### Scenario: A duplicate's copied state does not persist [lc_id_duplicate_save]

- **WHEN** a captured layer is duplicated, the document is saved and reopened, and the comp is applied
- **THEN** the copy is not captured and the original still is

#### Scenario: Identity survives save and reopen [lc_id_psd]

- **WHEN** a document with comps is saved and reopened
- **THEN** the comps still resolve the same layers and apply without warnings

### Requirement: Groups and adjustment layers take part in position capture

A group's captured position SHALL move its descendants together. An adjustment
layer's captured position SHALL move its mask. A shape layer's captured position
SHALL move its outline. Nested groups SHALL move once, not once per level.

#### Scenario: A group restores its children [lc_group_position]

- **WHEN** a position-recording comp captures a group, the group is moved, and the comp is applied
- **THEN** every descendant moves back by the same delta exactly once

#### Scenario: A nested group moves once [lc_group_nested]

- **WHEN** a position-recording comp captures a group containing another captured group, the outer group is moved, and the comp is applied
- **THEN** every layer below both groups moves by the outer delta exactly once

#### Scenario: An adjustment mask moves [lc_adjustment_position]

- **WHEN** a position-recording comp captures an adjustment layer with a mask, the layer is moved, and the comp is applied
- **THEN** the adjustment mask returns to its captured position

#### Scenario: A shape outline moves [lc_shape_position]

- **WHEN** a position-recording comp captures a shape layer, the shape is moved, and the comp is applied
- **THEN** the shape outline and its rendered content return to the captured position
