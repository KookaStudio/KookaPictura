# Design: move-memory-recovery

## Context

`RecoveryAttachment::capture` queues `doc.clone()`, a copy-on-write snapshot, but
reserved `history::document_bytes(doc) + tile_table_bytes(doc)`. The first term
counts every plane allocation the live document already holds; the clone shares
them, so the reservation double-counted gigabytes of live pixels and left the
move's `admit_spilling` no room.

`history::planes::tile_planes` converts every flat plane to tiles with
`Plane::tile`, which copies the plane into tiles while the flat buffer is still
held. A decoded 20000² background is 1.6 GB, so the first capture needs a second
1.6 GB and can OOM or thrash on a memory-constrained host.

## Decision

Reserve `tile_table_bytes(doc) + SNAPSHOT_METADATA_BYTES` for the queued
snapshot. The tile table is the only per-snapshot structure the clone adds; the
encoder's working memory is already gated in `write_checkpoint`. The metadata
constant covers the document structs and extra blocks.

In `tile_planes`, use `governor().reserve_if_room(plane.len())` for a flat
plane. If it does not fit, leave the plane flat: `diff_into` then compares it by
shared allocation and falls back to a full state when it changed. A tiling that
does fit still converts it once. `reserve_if_room` never evicts display tiles for
this optional conversion.

## Alternatives considered

- Remove `tile_planes` entirely: the tiled history's per-tile deltas would be
  lost for every document.
- Gate only the recovery capture: the history tiling still copies 1.6 GB on the
  first move.
- Use `reserve` for the tiling: it evicts display tiles to make room for a
  conversion that is optional, hurting the view.

## Risks

- A flat plane that does not fit stays flat; the history stores a full state
  (copy-on-write) instead of a tile delta, so an undo restores more metadata but
  shares the same pixels.
- The metadata estimate is a constant; a document with unusually large
  non-plane blocks could under-reserve. The encoder remains gated.
