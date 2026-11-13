# Proposal

## Why

Issue #1299, with #1273 folded in. A 17000 × 17000 RGB/8 document that was
open when Pictura was killed does not come back. Autosave writes a valid
snapshot. At the next launch, `recovery_open` → `PictureView::open` →
`decode_psd` refuses the decode for memory. `recoverOrphans` treats that the
same as a corrupt file: it moves the snapshot to `quarantine/` and shows only a
status-bar message. The document is never offered again.

The open is refused because the reader decodes every layer channel, in
parallel, into a dense buffer before tiling it (`read.rs` `tiled()`), and reads
the whole file into the heap (`fs::read`). So the whole decoded document has to
fit in RAM at once. Edits can spill tiles to scratch since #1217 and #1290, so
autosave can write a document that the same machine cannot read back. #1273
measured the same decode peak on M12: 2800 MiB above the start at 12000², and
refused at 17000².

## What Changes

- **Recovery keeps a memory refusal.** A snapshot whose open is refused for
  memory is not quarantined. Its snapshot and manifest stay in the orphaned
  session, and the session directory is kept, so the next launch offers it
  again. The other documents in that session recover, and their files are
  removed as before.
- **A recovery failure gets a dialog.** After Recover, a non-blocking dialog
  lists every document that did not come back. It names the snapshot that was
  kept, for lack of memory, or the quarantine copy, for an unreadable file.
- **Layer channels decode straight into tiles.** RLE, ZIP, ZIP with prediction,
  and raw rows of 8-, 16- and 32-bit layer and mask channels are decoded a row
  at a time into tiled planes (`Plane::build_tiled`). One 256-row strip per
  channel task is held contiguously, and uniform tiles hold no allocation. The
  16- and 32-bit native planes and their 8-bit narrowing are built in the same
  pass. The output is byte-identical to the dense decode, which stays as a
  test oracle.
- **A tile budget.** `ImageBudget` gains `max_tiled_bytes`. Channels decoded
  into tiles are charged against it, while contiguous allocations stay on
  `max_document_bytes`. The per-allocation cap still applies to both. The
  default is `max_document_bytes`, so readers that do not set it behave as
  before.
- **The file is mapped.** The app opens a PSD/PSB through a read-only memory
  map instead of reading it into the heap.
- **Open admits through room plus scratch.** `open_budget` keeps contiguous
  allocations at two thirds of the governor's room. It lets tiles use that plus
  the scratch disks' free space, as commits do since #1290.

## Capabilities

- **MODIFIED** `document/crash-recovery`: Recover after an unclean exit.
- **ADDED** `codec/streamed-layer-decode`
- **ADDED** `document/out-of-core-open`

## Impact

- `pictura-codec`: `read/decode.rs` and a new `read/stream.rs` (row decoders
  and the tile sink), `read.rs` (assembly, charges), `budget.rs` and `probe.rs`
  (`max_tiled_bytes`), a new `mapped.rs` (`MappedFile`, through `libc`, which
  is already a dependency).
- `pictura-app`: `open_jobs/imp.rs` (`open_budget`, the mapped read),
  `helpers.rs` (`bounded_budget`), the C++ `frame_recovery.cpp`, and
  `tst_recovery`.
- The `open_psd` heap guard baseline is lowered to the new measurement.
