# Proposal

## Why

Issue #1217 is step 9 of #1209 (bounded memory for large documents). After
steps 1–8, memory is proportional to a document's real content: tiles are
shared, uniform tiles hold no allocation, history holds tile references, and
the display keeps only visible tiles. A document with enough painted content
still has to fit in RAM, though, so the governor refuses edits that a scratch
disk could have absorbed.

`docs/10-workflow-io/scratch-disks-and-memory.md` specifies RAM plus scratch
disks: a typed error when scratch is full, no corrupted state, Scratch Sizes and
Efficiency readouts, and no leftover file after a crash. This change turns
"refused" into "slow but works". It also closes #943, which tracked the same
engine work.

## What Changes

- **Packed and spilled tiles.** A dense tile's samples live in a shared tile
  body whose residency can change. The body holds its samples (resident), its
  LZ4- or deflate-compressed bytes in RAM (packed), a checksummed extent in the
  scratch file (spilled), or nothing (unreadable). Each tile-access path in
  `pictura-core::plane` pins the body and restores it on first touch, so no
  caller changes.
- **A scratch file per volume** (`pictura-core::scratch`). It uses
  `pread`/`pwrite` on an unnamed file (`O_TMPFILE`, or create-then-unlink), an
  extent allocator with coalescing, and a checksum per extent. When a volume is
  full, spilling moves to the next of up to four volumes; when all are full it
  returns `ScratchFull` and backs off. The default volume is
  `$XDG_CACHE_HOME/kooka-pictura/scratch/`, and a tmpfs volume is flagged. A
  failure-injection hook covers a full disk, a read error, and a removed volume.
- **Eviction in the governor** (`pictura-core::memory::eviction`). A background
  clock (second-chance) thread starts above 100 % of the ceiling and stops at
  90 %. It frees, in order: display tiles (a drop request), tiles only history
  holds, then cold live tiles. Each tile is packed first and spilled after.
  Pinned tiles and tiles touched under a hot hold (the stroke or edit in
  progress, the visible view) are never evicted. Above 125 %, the allocating
  thread also evicts. A reservation that does not fit evicts reclaimable tiles
  synchronously before it refuses.
- **Clean tiles are free to drop.** A restored tile keeps its extent until it
  is written, so evicting it again writes nothing.
- **Unreadable tiles.** A failed checksum or read marks the body unreadable.
  An edit that read one is rolled back to the history's current state with a
  typed error. Save refuses a document holding one.
- **Readouts.** Scratch Sizes (used / available) and Efficiency (compute time
  versus scratch I/O time) come from the engine. The bridge and the control
  server's `status` carry them, together with the volume list, for #636's
  Performance pane and status bar. The volume list is a saved session setting
  with no UI yet (#636 adds it).
- **Dependency.** `lz4_flex` (MIT, pure Rust) is added only if it wins the R7
  benchmark against `flate2`, which is already present; the numbers go in
  `design.md`.

## Capabilities

### New Capabilities

- `document/scratch-disk`: the scratch file, its volumes, extents and
  checksums, typed errors, failure injection, crash cleanliness, Scratch Sizes,
  and Efficiency.
- `document/tile-eviction`: the packed, spilled and unreadable tile states;
  restore on touch; hot holds and pins; clock eviction and its priority order
  and thresholds; reclaim before refusal; clean-extent reuse; edit rollback and
  save refusal on unreadable tiles.

The issue names them `platform/scratch-disk` and `platform/tile-eviction`.
`openspec/config.yaml` has no `platform` domain and forbids adding one, so they
sit in `document/` next to `document/memory-governor` and
`document/tiled-planes`.

## Impact

- `pictura-core`: `tiled_plane` (tile bodies), `plane/access`, new `scratch`
  module, `memory::eviction`, governor reclaim. `libc` becomes a Unix
  dependency of core (it is already in the lock file, used by codec and mcp).
- `pictura-app`: the edit rollback, the save refusal, history holds on delta
  tiles, the display drop request, the hot hold around strokes, the bridge and
  `status` fields, and the session's scratch volume list.
- Out of scope: M12 under `MemoryMax=4G`. The open peak is in the PSD decode
  (#1273), so it is reported as blocked there. Also out of scope are the
  Performance pane UI (#636) and `Backed` tiles (#1218), which have a slot in
  the eviction order but no implementation here.
