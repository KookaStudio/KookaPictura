## ADDED Requirements

### Requirement: A Move tool commit is gated on RAM plus scratch

A Move tool commit of pixel layers SHALL be gated as a whole-pixel transform
is. Its working set (the composite's strip buffers across the changed region's
width) SHALL fit the governor's room. The tiled composite of the changed region
(4 bytes a pixel) SHALL fit that room plus the free space on the usable scratch
volumes. It SHALL NOT be charged for the moved layers' planes. When the
composite does not fit, the move SHALL be refused with a scratch-full refusal,
the document and history SHALL be unchanged, and the move refusal message
SHALL name the scratch disks.

#### Scenario: A large move spills instead of refusing [oct_move_gate_spills]

- **WHEN** the commit cost of moving a 17000 × 17000 full-canvas layer by (5, 5) is gated with the governor's room pinned at 1.75 GiB and 64 GiB of scratch available
- **THEN** the commit is admitted, although 12 bytes a pixel of the changed region exceeds the room

#### Scenario: A move needs room only for its tiled composite [oct_move_commits]

- **WHEN** a 4100 × 4100 full-canvas layer is moved by (5, 5) with room for 4 bytes a pixel of the changed region but not 12, and no scratch available
- **THEN** the move is recorded in history and no memory refusal is recorded

#### Scenario: Full scratch disks refuse a move with their own message [oct_move_scratch_full]

- **WHEN** the same move is committed with room for its working set but not its composite, and no scratch available
- **THEN** it is refused, the scratch-full refusal is recorded, history is unchanged, and the move refusal message names the scratch disks
