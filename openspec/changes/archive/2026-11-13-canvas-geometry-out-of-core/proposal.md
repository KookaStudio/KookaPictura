# Proposal

## Why

Issue #1294, split from #1290. Canvas Size and Rotate Canvas build dense,
whole-plane copies that the #1217 evictor can neither pack nor spill:

- **Canvas Size** (`resize_canvas_document`, `extend_background`) only moves
  the layer rects. But it copies every document alpha channel and the retained
  document store into new dense planes (`extend_channel`, `extend_samples`).
  `extend_background` also writes a dense canvas-sized plane for every channel
  of the Background.
- **90°/180°/270° turns and flips** (`transform_document`) copy each layer
  channel and mask into a `PixelBuffer` and remap it whole (`remap_plane`). Each
  retained store goes through `native_store::remap_samples` the same way.
- **Arbitrary Rotate Canvas** (`rotate_in.rs`) builds each rotated plane as a
  dense `Vec` with its bilinear inverse map. It runs on the GUI thread with no
  job, no progress and no cancel.

None of these commands passes a memory gate. The menu commands run ungated. The
control server checks Canvas Size against the RAM-only
`document_resize_budget_allows`. The rotations were never checked. A
17000 × 17000 document therefore cannot be turned, flipped or rotated under
`MemoryMax=2G` or `4G`. #1292 and #1293 already stream the warp and Image Size
commits through spillable tiles.

## What Changes

- **Tiled geometry.** One driver, `geometry_tiled`, builds each output plane a
  strip of 256 rows at a time into tiled planes, using
  `Plane::build_tiled_strips`. Blocks of a strip run in parallel. Each block
  reads only the source window it needs. Three kernels share the driver:
  - **Shift**, for Canvas Size's alpha channels and retained store and for
    `extend_background`. It copies source row spans and fills the rest.
  - **Orient**, for the five exact remaps. It reads the block's source window
    and permutes it.
  - **Rotate**, for an arbitrary angle. It reads the bounding box of the
    block's inverse-mapped corners and applies `rotate_in`'s bilinear sample.
    That sample is refactored to read through a closure, so the dense path and
    the tiled path share the same arithmetic.

  The output is byte-identical to the dense paths, which stay as
  `#[cfg(test)]` oracles. Canvas Size still moves layer rects and does not copy
  layer planes. Uniform tiles hold no allocation. Each command works on a copy
  that shares the document's planes, so a strip refusal (`OpsError::Memory`)
  leaves the document unchanged.
- **Gate.** Canvas Size, the quarter turns, the flips and arbitrary Rotate are
  gated as spilling commits, as Image Size is (#1293). Their working set is one
  output strip of the widest plane, one source window per worker thread, and
  the dense composite when the document's composite is authoritative. That
  working set must fit the governor's room. The planes they write must fit that
  room plus the scratch disks' free space. Canvas Size keeps only
  `ImageBudget`'s side limit from the old predicate. The control server's
  `image.canvas.size` uses the same gate. A refusal shows a status-bar message
  that names memory, or the scratch disks when they are full.
- **Jobs.** Arbitrary Rotate becomes a document job, as the quarter turns
  already are. On a large document it runs on a worker behind the progress
  window, counts strips, and can be cancelled.
- **Control.** `run_command` accepts the five orientation ids with no
  parameters, so the benchmark can drive them.
- **Benchmark.** M16 (Canvas Size to 150 %, centred), M17 (Rotate 90° CW) and
  M18 (Rotate 15°), each on a 17000² full-layer gradient, script the acceptance
  runs under `MemoryMax=2G` and `4G`.

## Capabilities

- **ADDED** `document/out-of-core-canvas-geometry`
- **MODIFIED** `document/image-resize-memory-budget`: Canvas Size takes only
  the side limit from the shared predicate.
- **MODIFIED** `document/long-operation-progress`: arbitrary Rotate is one of
  the whole-document commands that run as jobs.
- **ADDED** to `interop/agentic-control`: the orientation commands are
  runnable.
- **MODIFIED** `verification/memory-benchmarks`: the catalogue is M1–M18.

## Impact

- `pictura-render`: new `document_ops/geometry_tiled.rs` and its tests.
  `canvas.rs`, `orient.rs`, `rotate_in.rs` and `native_store.rs` move to it,
  and their dense helpers become test-only. New plane-byte estimators sit
  beside `resized_plane_bytes`. `flip_document` and `extend_background` now
  return a `Result`.
- `pictura-app`: `helpers/memory_gate.rs` (a shared geometry cost),
  `helpers.rs` (the admits), `image_size/imp.rs` (Canvas Size),
  `image_ops/imp.rs` (Rotate becomes a job), `impl_transform/dispatch.rs`, and
  the C++ menus, the control server and `memory_refusal`.
- `scripts/memory_scenarios.py` and `scripts/profile-tools.py` (M16–M18).
