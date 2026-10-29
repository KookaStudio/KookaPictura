# Spec Delta

## REMOVED Requirements

### Requirement: Structural anchors share unchanged tracked planes

**Reason**: Sharing is now band-granular, so the requirement's "a later write
forks only the plane it touches" and "no live plane shares an allocation with
history" no longer hold: an untouched band is shared with history, and a write
forks only the touched bands.

**Migration**: None for users. Structural capture and in-place restore keep the
same byte-exact result and the same retained-memory bound, measured in bands
rather than whole planes.

## ADDED Requirements

### Requirement: Structural anchors share unchanged tracked bands

Tracked planes are the composite, the document channels, the layer channels, and
the layer masks; smart-object filter masks and native-depth `Samples` are not
tracked. When a capture starts a new geometry segment because the tracked-plane
count, size, or order changed, the history SHALL build the new current state
from the previous current state's tracked planes, matching them to the live
tracked planes by nonzero stamp or by shared band, and SHALL copy only the
tracked planes or bands that are new or no longer shared. Tracked planes shared
this way SHALL be band-granular copy-on-write, so a later write forks only the
bands it touches. The live document SHALL NOT share a written band with any
history state after a capture or an in-place restore (`undo_live`, `redo_live`,
or `jump_live`); untouched bands MAY remain shared. The legacy `undo`, `redo`,
and `jump` return a shared snapshot by design.

Restoring a state into the live document SHALL likewise keep the live
document's own tracked planes and bands where their allocations match the target
and copy only the tracked planes or bands that are new or no longer match; an
unstamped tracked plane has only its unshared bands copied even when its bytes
match. Retained tracked-plane bytes after a series of structural captures SHALL
be bounded by the base state plus the bands those captures changed and their
tile deltas, not by the number of captures times the document's tracked-plane
size.

#### Scenario: Twenty added layers retain one copy of the base [hr_structural_sharing]

- **WHEN** twenty New Layer captures are made on an N-layer document and no
  other pixels change
- **THEN** the history retains one copy of the base document's bands, one copy
  of each added layer's bands, and the changed tile deltas, rather than twenty
  full documents

#### Scenario: Structural states stay byte-exact [hr_structural_exact]

- **WHEN** layer additions, deletions, and reorders are interleaved with paint
  captures and then undone, redone, and jumped across segment anchors
- **THEN** every restored state is byte-identical to its captured document and
  selection

#### Scenario: Live writes stay private through structural captures [hr_live_private]

- **WHEN** a band of the live document is written after any structural capture
  or any in-place restore (`undo_live`, `redo_live`, or `jump_live`)
- **THEN** that band does not share an allocation with any retained history
  state, while bands the live document did not write may stay shared

#### Scenario: Undo of an add keeps the live bands [hr_restore_reuses_live]

- **WHEN** a state that added a layer is undone and the target state still
  contains the other layers and the composite
- **THEN** the live document keeps its own allocations for those bands and
  copies only the bands that the target changes or removes

## MODIFIED Requirements

### Requirement: Region-delta undo retention

The history SHALL store the oldest state as a materialized base and each later
state as the 64×64 tiles whose bytes differ from the previous state, for the
layer channel planes, the document composite, and the layer mask planes, plus
the state's selection and non-pixel metadata. The retained history memory SHALL
scale with the edited area rather than with the document area. The single
materialized base MAY share unchanged bands with the live document; bands that
differ SHALL be copied. Smart-object filter masks and native-depth `Samples` are
not tracked planes and SHALL remain outside this contract. The public `History`
API (`capture`, `undo`, `redo`, `jump`, `can_undo`/`can_redo`, the
label/index/depth/count accessors, and the named-snapshot accessors) and the
20-state and 10-snapshot bounds SHALL be unchanged.

#### Scenario: Small edits retain small deltas [hr_bounded_retention]

- **WHEN** many small edits are captured on a large document
- **THEN** the retained delta bytes are bounded by the changed tile area and the
  history keeps a single materialized base, not one full state per edit

#### Scenario: A geometry change stores a full state [hr_geometry_anchor]

- **WHEN** a captured state changes the tracked-plane count, stride, or height
  (resize, crop, flatten, or a layer/mask structure change)
- **THEN** that state is stored as a full snapshot and becomes a new anchor, and
  tile deltas only span a stable geometry
