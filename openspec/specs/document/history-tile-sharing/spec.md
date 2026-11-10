# document/history-tile-sharing Specification

## Purpose
Undo states and named snapshots hold references to the document's 256² tiles
instead of copies of their samples, so a snapshot and its restore cost metadata
and undo or redo moves references. History's memory budget counts each tile it
keeps beyond the live document once (#1215).

## Requirements

### Requirement: States hold tile references

A history state SHALL hold references to the 256² tiles of the tracked planes
its edit changed, never copies of their samples. A tile the edit did not change
SHALL be the same allocation in that state as in its neighbour states. Undo,
redo and a history jump SHALL move tile references between the stored states
and the document, without copying samples.

#### Scenario: A delta keeps the replaced tile [hts_reference]

- **WHEN** an edit changes a few samples of one tile and the state is captured
- **THEN** the state's delta holds that tile's previous allocation, and every
  other tile of the plane is still shared with the previous state

#### Scenario: Undo and redo are byte-identical [hts_identity]

- **WHEN** a seeded sequence of paint, layer and visibility edits is captured
  and then undone to the first state and redone to the last
- **THEN** after every step the live document equals, byte for byte, the
  document captured for that state, and its plane stamps agree with history's

### Requirement: Snapshots cost metadata, not pixels

Taking a named snapshot SHALL share every tile with the live document, and
restoring one SHALL install the snapshot's tiles again by reference. Neither
SHALL copy a sample or materialize a tiled plane. A restore SHALL present the
snapshot's stored composite without compositing the layers again.

#### Scenario: Taking and restoring a snapshot copies no tile [hts_snapshot]

- **WHEN** a snapshot is taken, the document is edited, and the snapshot is
  restored
- **THEN** history's held bytes do not grow when the snapshot is taken, the
  restore swaps the edited tile into a delta instead of copying anything, and
  no plane is materialized

#### Scenario: The application round trip [hts_snapshot_gui]

- **WHEN** the user fills a marquee, takes a snapshot, fills another region
  across tile edges, and restores the snapshot
- **THEN** the canvas shows the snapshot's pixels, one state named after the
  snapshot is added, and undo brings back the edit

#### Scenario: Snapshot and restore at 17000² [hts_snapshot_large]

- **WHEN** M8 (snapshot, then restore) runs on a 17000² document
- **THEN** the history and document memory it adds is at most 16 MiB and the
  snapshot and restore take at most 100 ms. The display refresh is not counted
  (#1216).

### Requirement: The history budget counts unique tiles

History's budget SHALL be the memory governor's history share of its ceiling.
History SHALL count against it the bytes of the tiles it holds that the live
document does not hold. A tile held by several states or snapshots SHALL be
counted once. The count SHALL be kept up to date as states change, not by a
walk over every tile on each capture.

#### Scenario: A shared tile is counted once [hts_counted_once]

- **WHEN** a snapshot and a later state's delta hold the same tile
- **THEN** history's held bytes count that tile once, and match a full recount

#### Scenario: The budget is half the ceiling [hts_budget]

- **WHEN** the governor reports a ceiling
- **THEN** the history budget is half of it

### Requirement: Over budget, the oldest states go first

When history holds more than its budget, it SHALL drop the oldest states first.
It SHALL always keep the current state and one undo. A named snapshot SHALL NOT
be dropped for the budget.

#### Scenario: The oldest states go [hts_drop_order]

- **WHEN** a series of whole-plane edits exceeds the budget
- **THEN** the oldest states are dropped until history holds no more than its
  budget, and one undo is still available

#### Scenario: Strokes under a 4 GiB limit [hts_memory_max]

- **WHEN** twenty M6 brush strokes run on a 17000² document under
  `MemoryMax=4G`
- **THEN** history keeps to its budget and the process is not killed

### Requirement: Purge releases history tiles

Purge SHALL release the tiles of the states and snapshots it drops, and a purge
SHALL NOT be undoable. Purge Undo SHALL keep the named snapshots. Purge All
SHALL leave history holding no tile beyond the live document.

#### Scenario: Purge frees what it drops [hts_purge]

- **WHEN** states and a snapshot hold tiles, and then Purge Undo, Purge
  Histories and Purge All run
- **THEN** Purge Undo leaves only the snapshot's tiles, Purge Histories and
  Purge All leave none, and no undo is available after a purge
