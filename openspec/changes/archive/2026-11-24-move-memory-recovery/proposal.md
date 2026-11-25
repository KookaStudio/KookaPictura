# Proposal: move-memory-recovery

## Why

Repeated Move drags on a 20000 × 20000 recovered document end in "Could not
move: not enough memory" and repeated crash recovery. Two accounting mistakes
make the move's gate refuse work that allocates almost nothing new. The recovery
snapshot is a copy-on-write clone, but it reserves the live document's whole
plane bytes; and the history capture converts a large flat plane to tiles by
copying it, which needs a second full-size buffer.

## What Changes

- A recovery snapshot reserves only its tile tables and metadata, not the pixel
  planes it shares with the live document.
- Tiling a large flat plane in the history capture is optional and uses
  `reserve_if_room`: under pressure the plane stays flat and the history stores
  a full, copy-on-write state.
- Both are pinned by Rust tests.

## Capabilities

### Modified Capabilities

- `document/crash-recovery`: a snapshot's reservation counts only the memory it
  adds.
- `document/history-tile-sharing`: a large flat plane is tiled only when the
  room allows it.

## Impact

- `pictura-app` recovery store and history plane tiling. Rust tests only; no
  document-format change.

## Provenance

Behavior follows the existing memory-governor, recovery, and history contracts;
no external implementation is used.
