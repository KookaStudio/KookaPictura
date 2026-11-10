# Proposal

## Why

Issue #1215 is step 7 of #1209 (bounded memory for large documents). History
already shares untouched planes with the live document, but three costs remain:

- **A named snapshot copies every plane.** Taking one, and restoring one, both
  went through `detached_copy`, which copies every tracked plane of the
  document. At 17000² with a few layers that is gigabytes for one click. The
  #1177 audit saw M7 and M8 refused under 8 GiB.
- **A delta copies bytes.** Each state stored the 64×64 regions that changed as
  owned byte vectors, even though #1213 had already given the planes 256² tiles
  that could simply be shared.
- **The budget miscounted.** The history cap added up those byte vectors
  without noticing what states and snapshots share, and it recounted the whole
  history on every capture.

## What Changes

- **Deltas hold tile references.** A state keeps a reference (`SharedTile`) to
  each 256² tile its edit replaced. An undo or redo swaps references between the
  delta and the document. No sample is copied.
- **Snapshots share tiles.** A named snapshot is a clone of the live document,
  and its restore installs that clone again. The cost is the document's
  metadata and tile tables. The restore presents the snapshot's exact composite
  and does not composite it again.
- **The budget counts unique tiles.** A tile census in
  `pictura_core::memory` counts every tile allocation history holds, each once,
  and leaves out the ones the live document holds too. It is kept up to date as
  history changes, so a capture does not walk every tile. The history budget is
  the governor's `history_budget()`, half the ceiling as before.
- **Purge releases tiles.** Purging releases the tiles of the states and
  snapshots it drops, and it cannot be undone.
- `detached_copy` and `copy_plane` are removed.

## Capabilities

### New Capabilities

- `document/history-tile-sharing`: states, snapshots and restores as tile
  references; the unique-tile budget; purge.

### Modified Capabilities

- `document/history-retention`: "Region-delta undo retention" now stores the
  changed 256² tiles by reference instead of 64×64 byte copies.

## Impact

- `pictura-core`: `SharedTile`, `Plane::shared_tile` and `Plane::swap_tile`;
  `memory::TileCensus`, `Governor::history_budget` and `HISTORY_SHARE`.
- `pictura-app`: `history` (deltas, census, budget), `impl_history` (named
  snapshot, restore), the memory gate's snapshot estimates, and the M8 heap
  scenario.
- No new dependency and no change to the document format.
