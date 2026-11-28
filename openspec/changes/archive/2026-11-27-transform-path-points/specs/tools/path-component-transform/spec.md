## Purpose

Transform selected editable path geometry with interactive controls and repeat a committed transform on components or anchors.

## ADDED Requirements

### Requirement: Transform selected path components

Transform Path Scale, Rotate, Skew, Distort, Perspective, and Warp SHALL operate on whole selected components of the active Work, saved, or editable shape/vector-mask path, with a nondegenerate shared transform box and no conflicting modal session. They SHALL use the existing mode controls, reference point, numeric controls, live outline, commit and cancel. Each selected anchor and both direction points SHALL be mapped from the original geometry on every preview; unselected geometry, vector-mask flags, and unrelated pixels SHALL remain unchanged. Affine modes SHALL map cubic control points exactly. Projective and warp modes SHALL map control points through their respective maps as the existing shape-layer transform does, an approximation of the transformed curve. Invalid maps SHALL leave geometry and history unchanged. A nonidentity commit SHALL record one history state; cancel and identity SHALL restore the original target and add no history. Undo and redo SHALL restore and reapply the complete change.

#### Scenario: Affine modes and controls [pt_affine]

- **WHEN** selected curved components across a path are scaled, rotated and skewed using gestures, a reference point and numeric controls
- **THEN** live outlines and committed anchors and handles follow each affine map, unselected geometry stays unchanged, and each commit is one undoable and redoable state

#### Scenario: Projective and warp modes [pt_nonlinear]

- **WHEN** selected curved components are distorted, transformed with perspective and warped
- **THEN** anchors and handles follow the corresponding map under the control-point approximation, unselected geometry stays unchanged, and each commit is one undoable and redoable state

#### Scenario: Path targets and metadata [pt_targets]

- **WHEN** a Work Path, saved path, or editable shape/vector-mask path is transformed and undone and redone
- **THEN** only selected geometry changes, vector-mask flags and unrelated pixels are preserved, the display reflects the target, and a transformed live shape becomes a regular path while undo restores its live metadata

#### Scenario: Cancel, identity and invalid map [pt_cancel]

- **WHEN** a preview is cancelled, an identity is committed, or a map is invalid, including on a live shape
- **THEN** original geometry, mask flags and live-shape metadata are restored and history is unchanged

#### Scenario: Command availability and session ownership [pt_enable]

- **WHEN** no compatible whole component is selected, the shared box is degenerate, the target is not editable or is position locked, or another modal session is active
- **THEN** the path transform commands are disabled and no competing live session is created

#### Scenario: Target change during preview [pt_target_change]

- **WHEN** the target or document changes, or history invalidates a path during a preview
- **THEN** the captured target is restored before the session ends, stale selection is cleared, and no history state is added

### Requirement: Repeat a path transform

A successfully committed nonidentity repeatable similarity transform on editable path components SHALL be available to Transform Path Again and Transform Points Again in that document, using the existing Transform Again source-frame pivot-offset and translation convention. Path Again SHALL act on currently selected whole components; Points Again SHALL act on two or more selected anchors across the active path, mapping each anchor and its two direction points. The shared selection bounds SHALL define the destination frame. Unselected geometry, vector-mask flags, and unrelated pixels SHALL remain unchanged. Either repeat SHALL add exactly one history state and support undo and redo. Cancelled, identity, and non-repeatable projective or warp sessions SHALL NOT replace the previous repeatable transform. Repeat SHALL be disabled without a compatible selection or record, for an uneditable target, or during a conflicting modal session. Records belong to documents and SHALL be discarded when their document closes or is replaced.

#### Scenario: Repeat on components [pt_again_path]

- **WHEN** a repeatable rotation with an offset pivot is committed and Path Again is invoked on selected components
- **THEN** the rotation and source-frame pivot offset are applied in one undoable state with other geometry unchanged

#### Scenario: Repeat on anchors [pt_again_points]

- **WHEN** a repeatable transform exists and Points Again is invoked on anchors in different components
- **THEN** only those anchors and their handles transform, and undo and redo restore and reapply the result

#### Scenario: Repeat availability and retention [pt_again_enable]

- **WHEN** a record or compatible selection is absent, the target is uneditable, or another session is active
- **THEN** repeat is disabled
- **WHEN** a later session is cancelled, identity-committed, or commits a non-repeatable map
- **THEN** the previous repeatable record remains available

#### Scenario: Independent document records [pt_again_document]

- **WHEN** two documents have different repeat records and the user switches between them, then closes or replaces one
- **THEN** each open document uses only its own record and the closed or replaced document's record cannot be reused
