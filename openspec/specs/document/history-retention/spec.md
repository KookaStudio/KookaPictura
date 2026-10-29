# document/history-retention Specification

## Purpose
Bounded undo/redo that stores each state's changed 64×64 region tiles against
the previous state, so retained history memory scales with the edited area
rather than the document area.

## Requirements

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

### Requirement: Byte-identical state materialization

`undo`, `redo`, and `jump` SHALL return a `Snapshot` whose `Document` and
selection are byte-identical to the state that was captured, including states
reachable only across a full anchor. The label order, cursor movement, depth
bound, and redo truncation SHALL be unchanged.

#### Scenario: Every state round-trips exactly [hr_exact_materialize]

- **WHEN** undo, redo, or jump restores any captured state
- **THEN** the restored document equals the captured document field for field,
  including pixels stored as tiles and pixels left unchanged since the base

#### Scenario: A paint commit undoes to the prior pixels [hr_paint_undo]

- **WHEN** a paint commit is captured and then undone
- **THEN** the edited layer channel plane and the composite equal their bytes
  before the commit, and redo restores the committed bytes

#### Scenario: Undo across a geometry change is exact [hr_geometry_undo]

- **WHEN** a geometry-changing state is captured and then undone
- **THEN** the restored document equals the pre-change document exactly

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

### Requirement: Untracked stores are shared across history states

The history SHALL share, while unchanged, every bulk byte or sample store that
is not a tracked plane: retained native sample stores, smart-object payloads and
the raw linked-section bytes, preserved byte blocks, and the selection mask. It
SHALL share the layer tree's nodes' bulk stores. The layer-tree container (the
`Arc<Vec<Layer>>` allocation) MAY be forked once per capture path (once per
materialized state) while its nodes' bulk stores stay shared, and small
container vectors such as a layer's channel or block lists MAY be copied. No
bulk byte or sample buffer may be copied by a capture that does not write it. A
write to a shared bulk store SHALL fork only that store, leaving every captured
state byte-identical.

#### Scenario: Unchanged native samples are retained once

- **WHEN** many captures are made on a 16-bit document whose native samples do not change
- **THEN** every state shares one sample allocation per store and retained sample bytes do not grow with the number of captures

#### Scenario: An unchanged selection is retained once

- **WHEN** many captures are made and the selection is not written
- **THEN** every state shares one selection allocation

#### Scenario: Unchanged payloads and byte blocks are retained once

- **WHEN** many captures are made without editing a smart-object payload, the raw linked section, or a preserved byte block
- **THEN** every state shares those allocations and their retained bytes do not grow with the number of captures

#### Scenario: A capture forks the tree container but shares its nodes

- **WHEN** many captures are made without a structural edit
- **THEN** a capture may allocate a new layer-tree container, but every node's bulk stores remain shared across the states

#### Scenario: A native edit retains only the changed plane

- **WHEN** a capture writes one channel plane of a per-channel native store
- **THEN** only that plane is copied and the other planes remain shared with the prior state

#### Scenario: Undo of a native edit is byte-exact

- **WHEN** a capture changes a native sample store, the smart-object payload, the raw linked section, or the selection, and the state is undone
- **THEN** the restored document and selection equal the captured pre-edit bytes
