# document/memory-gate-admission Specification

## Purpose
Gives every memory-gated edit one admission call and one typed outcome, so a
command cannot forget to reset the protocol, cannot ask the wrong cost, and
cannot present a refusal for an edit that succeeded.

## Requirements

### Requirement: A gated command makes one admission call

A memory-gated command SHALL test whether it fits by making one admission call
that supplies its own declared cost, and SHALL receive one outcome. The
admission SHALL itself reset the previous refusal, judge the cost against the
governor's room, judge spillable output against that room plus the free space on
the usable scratch volumes, and record a refusal reason when it refuses. A
command MUST NOT reset refusal state, select an admission function, or ask the
governor's room separately.

#### Scenario: A fitting edit is admitted [mga_admits]

- **WHEN** a command supplies a cost within the governor's room and the scratch
  disks' free space
- **THEN** the admission yields an admitted outcome and no refusal is recorded

#### Scenario: An edit beyond the room is refused [mga_refuses]

- **WHEN** a command supplies a cost whose resident bytes exceed the governor's
  room
- **THEN** the admission yields a refusal naming a memory shortfall, and the
  command leaves the document, its history and its modified flag unchanged

#### Scenario: Spillable output beyond room plus scratch is refused [mga_scratch_full]

- **WHEN** a command's resident bytes fit the room but its spillable bytes
  exceed the room plus the free space on the usable scratch volumes
- **THEN** the admission yields a refusal naming full scratch disks, and no
  memory shortfall reason is presented for it

### Requirement: An admitted outcome reports whether the edit must spill

An admitted outcome for a commit SHALL report whether the commit can fit only by
packing or spilling tiles, so a caller asks admission once instead of asking
whether the edit fits and then separately whether it spills.

#### Scenario: A commit that exceeds the room is marked to spill [mga_spill_marked]

- **WHEN** a commit whose resident and spillable bytes together exceed the
  governor's room is admitted because that room plus the scratch disks' free
  space holds it
- **THEN** the admitted outcome reports that the commit must spill, and a commit
  the room holds on its own does not

### Requirement: No refusal outlives the edit that produced it

A refusal SHALL belong to the edit that recorded it. An edit that makes no
admission call MUST NOT present a refusal recorded by an earlier edit, and an
edit that is admitted MUST NOT present one. A command that reuses a gate helper
SHALL reach admission through the seam rather than asking the governor directly.

#### Scenario: A successful edit after a refusal presents no refusal [mga_no_stale]

- **WHEN** an edit is refused, the room is restored, and an edit that makes no
  admission call then succeeds
- **THEN** presenting that edit's outcome reports no refusal

#### Scenario: A Quick Mask stroke after a refusal presents no refusal [mga_quick_mask_clean]

- **WHEN** an edit is refused, the room is restored, and a Quick Mask stroke or
  Quick Mask filter then succeeds
- **THEN** its outcome reports no refusal and no memory message is shown

### Requirement: Every refusal presents through one path

The application SHALL present a refusal reason through one path that maps the
typed reason to the status-bar message. Per-command presenters MAY supply their
own wording but MUST NOT re-derive which reason applies. Every reason SHALL be
distinguishable on presentation, including full scratch disks.

#### Scenario: Every command family presents its own wording [mga_one_path]

- **WHEN** a fill, a Move, a Free Transform, an Image Size, an undo, an add
  layer and a save are each refused for a memory shortfall
- **THEN** each shows its own action wording from the same presentation path

#### Scenario: A scratch-full refusal names the scratch disks [mga_scratch_full_presented]

- **WHEN** a fill, a Move, a Free Transform or an Image Size is refused because
  the scratch disks cannot hold its spillable output, and a save is refused for
  the same reason
- **THEN** each presented message says the scratch disks are full rather than
  that there is not enough memory

### Requirement: A refusal recorded after an edit commits is still typed

An edit that commits and then rolls back because it read tiles lost from the
scratch disk SHALL record the unreadable-tiles reason through the same typed
refusal, so its presentation does not depend on a separate predicate.

#### Scenario: A rolled-back edit reports unreadable tiles [mga_rollback]

- **WHEN** an edit reads an unreadable scratch tile and is rolled back to the
  history's current state
- **THEN** its outcome reports the unreadable-tiles reason and the presented
  message says image data could not be read back from the scratch disk
