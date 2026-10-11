## ADDED Requirements

### Requirement: History keeps each changed tile once

History SHALL store each changed 64×64 tile of a delta once, holding the side
that the materialized state walked from does not hold. For a state at or below
the cursor that is the bytes before the change; above it, the bytes after.
Capture SHALL compare the live document with the private current state, keep
the current bytes of each changed tile, and copy the live bytes in. A walk that
moves the current state or a full anchor across a delta SHALL exchange the
delta's bytes with the state's. A walk that only rebuilds a state (the History
Brush source, a dropped anchor) SHALL read the deltas without changing them.
Every state SHALL rebuild byte-exactly after any sequence of captures, undos,
redos and jumps.

#### Scenario: Ten one-tile edits retain ten tiles

- **WHEN** ten states each change only the first 64×64 tile of the composite
- **THEN** history retains at most ten tiles' bytes and every state jumps back exactly

#### Scenario: Every state rebuilds after random walks

- **WHEN** a seeded sequence of edits, layer changes, undos, redos and jumps runs against a full-snapshot reference
- **THEN** after every step each retained state rebuilds equal to its reference, without moving the cursor

#### Scenario: A full-layer filter costs one copy of what it changed

- **WHEN** a Radial Blur is committed to an 8000×8000 layer
- **THEN** the new delta holds one side of the changed tiles: half the bytes a before-and-after delta held
