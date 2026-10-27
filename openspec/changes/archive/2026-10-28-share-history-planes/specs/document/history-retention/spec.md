# Spec Delta

## ADDED Requirements

### Requirement: Structural anchors share unchanged tracked planes

Tracked planes are the composite, the document channels, the layer channels, and
the layer masks. When a capture starts a new geometry segment because the
tracked-plane count, size, or order changed, the history SHALL build the new
private current state from the previous current state's tracked planes, matching
them to the live tracked planes by nonzero stamp, and SHALL copy only tracked
planes that are new or whose stamp no longer matches. Tracked planes shared this
way SHALL be copy-on-write, so a later write forks only the plane it touches.
The live document SHALL NOT share a tracked plane with any history state after a
capture or an in-place restore (`undo_live`, `redo_live`, or `jump_live`); the
legacy `undo`, `redo`, and `jump` return a shared snapshot by design.

Restoring a state into the live document SHALL likewise keep the live
document's own tracked planes where their stamps match the target and copy only
tracked planes that are new or whose live stamp no longer matches the target's;
an unstamped tracked plane is copied even when its bytes match. Retained
tracked-plane bytes after a series of structural captures SHALL be bounded by
the base state plus the tracked planes those captures changed and their tile
deltas, not by the number of captures times the document's tracked-plane size.

#### Scenario: Twenty added layers retain one copy of the base [hr_structural_sharing]

- **WHEN** twenty New Layer captures are made on an N-layer document and no
  other pixels change
- **THEN** the history retains one copy of the base document's planes, one copy
  of each added layer's planes, and the changed tile deltas, rather than twenty
  full documents

#### Scenario: Structural states stay byte-exact [hr_structural_exact]

- **WHEN** layer additions, deletions, and reorders are interleaved with paint
  captures and then undone, redone, and jumped across segment anchors
- **THEN** every restored state is byte-identical to its captured document and
  selection

#### Scenario: Live stays private through structural captures [hr_live_private]

- **WHEN** any structural capture or any in-place restore (`undo_live`,
  `redo_live`, or `jump_live`) completes
- **THEN** no tracked plane of the live document shares an allocation with a
  tracked plane of any retained history state

#### Scenario: Undo of an add keeps the live planes [hr_restore_reuses_live]

- **WHEN** a state that added a layer is undone and the target state still
  contains the other layers and the composite
- **THEN** the live document keeps its own allocations for those planes and
  copies only the planes that the target changes or removes
