# Proposal

## Why

Issue #1292, split from #1290. The Free Transform warp commit (`warp_planes`,
the triangle-mesh rasterizer) converts the whole source to dense premultiplied
`f32` planes and rasterizes the whole destination into dense `f32` planes before
it narrows them to the layer's sample type. A 17000² RGBA layer needs about
4.6 GB for each side. The #1217 evictor cannot pack or spill any of it, so on a
memory-limited host a large warp is refused or holds the full layer in RAM.
#1290 already streams the similarity and quad paths through spillable tiles.

## What Changes

- **Tile-streamed warp.** The warp commit rasterizes one strip of 256
  destination rows at a time, in blocks of 64 rows by 256 columns, into tiled
  planes the evictor can pack or spill. Each block keeps only the triangles
  that reach it, in depth order, and reads only the source window its samples'
  kernel taps and shrink footprints reach, converted and premultiplied for that
  window alone. The block splits until the window fits a fixed byte budget.
  Uniform tiles hold no allocation. The layer's channels, its mask and the
  retained 16/32-bit store all go through it. The output is byte-identical to
  the dense rasterizer, which is kept as the test oracle.
- **Gate.** A warp commit's working set also counts its output strips.
- **Control and benchmark.** `layer_op` `free_transform` accepts an optional
  warp preset and bend, and the memory benchmark gains M14, an arc warp of a
  17000² full-layer gradient, so the acceptance run under `MemoryMax=4G` is
  scripted.

## Capabilities

- **ADDED** `document/out-of-core-warp`
- **MODIFIED** `interop/agentic-control`: `free_transform` takes `warp` and `bend`.
- **MODIFIED** `verification/memory-benchmarks`: the catalogue is M1–M14.

## Impact

- `pictura-render`: `layer_ops/warp_raster.rs` (tiled rasterizer),
  `layer_ops/warp.rs` (layer, mask and store passes), new test-only
  `warp_oracle.rs` (the dense rasterizer, unchanged).
- `pictura-app`: `impl_transform/group.rs` (`warp_strip_bytes`),
  `impl_transform/session.rs` (gate), `cpp/control_server_actions.cpp`.
- `scripts/memory_scenarios.py`, `scripts/profile-tools.py` (M14).
