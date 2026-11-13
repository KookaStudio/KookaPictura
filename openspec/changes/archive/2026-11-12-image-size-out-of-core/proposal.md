# Proposal

## Why

Issue #1293, split from #1290. `Image > Image Size` (`resize_document`)
resamples every layer channel, mask, alpha channel and retained 16/32-bit store
as a whole dense plane. `resample_plane` copies the source into a
`PixelBuffer` (materializing a tiled source first) and `pictura_ops::resize`
allocates the whole destination. None of it can be packed or spilled by the
#1217 evictor. The gate (`resize_budget_allows`) therefore refuses any target
whose `width × height × 4` bytes exceed the governor's room, so a 17000²
document cannot be resized under `MemoryMax=2G` or `4G` at all. #1290 and #1292
already stream the transform and warp commits through spillable tiles.

## What Changes

- **Tile-streamed resize.** Image Size resamples one strip of 256 destination
  rows at a time into tiled planes the evictor can pack or spill. The resample
  is separable: each block of 16 rows by 256 columns reads only the source rows
  its vertical taps reach and the column span its horizontal taps reach,
  splitting until that window is at most 1 Mpx. Uniform tiles hold no
  allocation. Every plane goes through it: layer channels, layer masks,
  document alpha channels, the retained per-layer store (its `-2` mask plane
  at the mask's rect) and the document-level retained store. The math is
  unchanged: the per-axis taps move into `pictura_ops::resize` and both the
  dense path and the tiled path use them, the 8-bit planes keep that
  resampler's rounding, and the retained stores keep their native-precision
  unit scale. The output is byte-identical to the dense resamplers, which stay
  as test oracles.
- **Gate.** An Image Size is gated as a spilling commit: its working set (one
  output strip and one source window per worker thread, plus the dense
  composite of a document whose composite is authoritative) must fit the
  governor's room, and its resampled planes must fit the room plus the free
  space on the scratch disks. The side limit of `ImageBudget` still applies.
  A refusal for full scratch disks gets its own message. The dialog, the
  command replay, the control server's `image.image.size` and the
  `resize_image` bridge share the gate; Canvas Size keeps its RAM-only gate
  until #1294.
- **Benchmark.** The memory benchmark gains M15, Image Size of a 17000²
  full-layer gradient to 150 %, so the acceptance runs under `MemoryMax=2G`
  and `4G` are scripted.

## Capabilities

- **ADDED** `document/out-of-core-image-size`
- **MODIFIED** `document/image-resize-memory-budget`: the gate is RAM plus
  scratch, the refusal names full scratch disks, and Image Size uses the shared
  predicate for its sides only.
- **MODIFIED** `verification/memory-benchmarks`: the catalogue is M1–M15.

## Impact

- `pictura-ops`: `resize.rs` (public per-axis taps, `OpsError::Memory`).
- `pictura-render`: new `document_ops/resize_tiled.rs`; `document_ops/resize.rs`
  (every plane through it, rows counted for the stores);
  `document_ops/mod.rs` and `native_store.rs` (the dense resamplers become
  test-only).
- `pictura-app`: `helpers/memory_gate.rs` (`image_size_cost`),
  `image_adjust/image_size*` (gate, bridge), `impl_transform/dispatch.rs`
  (`resize_image`), `cpp/image_size_dialog.cpp`, `cpp/frame_menus_image.cpp`,
  `cpp/control_server_commands.cpp`.
- `scripts/memory_scenarios.py`, `scripts/profile-tools.py` (M15).
