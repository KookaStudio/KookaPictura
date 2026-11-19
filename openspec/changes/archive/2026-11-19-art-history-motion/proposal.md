# Proposal

## Why

Art History Brush stalls during pointer motion on large documents. Issue
[#1238](https://github.com/KookaStudio/KookaPictura/issues/1238) measured a
motion p95 of 107–128 ms at 8000×6000. The press and source work tracked in
#1094 and #1070 does not explain the delay. Measured on this branch, the stall
belongs to motion: on a document where every dab paints nothing, the stroke
still spent about 20 ms per move in the dab math. Presenting each move added
more, through serial passes in the tile repair and a GPU round trip for small
regions.

## What Changes

- **Art History strokes paint in one pass.** Each scattered stroke stamps its
  tips in one row-parallel pass, with the tip profile built once and each row
  limited to the tip's reach. Every pixel sees the same blends in the same
  order, so output stays byte-identical.
- **A stroke saves a tile with one read.** The stroke's undo save copies each
  64 px tile it reaches with one plane read, not one read per row.
- **Small region presents composite on the CPU.** The display's region refresh
  uses the GPU only for regions of at least 4 MP. Below that, the per-layer
  upload and readback cost more than the CPU composite.
- **Pyramid repair does less.** A repair builds level 0's straight rows only
  when a level-0 tile is cached, halves without copying its input, and
  interleaves rows in parallel.
- **The profiler gains a reproducible Art History case.** Brush options are
  pinned. A textured scenario paints from a textured source onto a layer that
  has drifted from it, and a History Brush scenario on the same workload
  serves as the budget reference. The control server's `history` method gains
  `brush_source`, which sets the History Brush source state.

## Capabilities

### New Capabilities

- `tools/art-history-motion`: one-pass Art History strokes and the motion budget.
- `tools/stroke-tile-save`: a stroke's undo save reads each tile once.
- `compositing/region-present-backend`: the size floor for GPU region refreshes.
- `compositing/pyramid-repair`: what a tile-pyramid repair builds and copies.
- `verification/art-history-motion-profile`: the profiler's Art History
  scenarios and the `history` `brush_source` op.

## Impact

- **Engine**: `pictura-paint` (`art_history.rs`, `stroke.rs`) and
  `pictura-render` (`tile_pyramid.rs`, `region_level.rs`, `tile_build.rs`).
- **App Rust**: `cxxqt_object/tiles.rs` (GPU floor).
- **App C++**: `control_server_commands.cpp` (`brush_source`).
- **MCP**: the `pictura_history` tool schema lists `brush_source`.
- **Scripts**: `profile-tools.py` scenarios and self-check.
- **Not changed**: press-time source construction (#1094, #1070), the hold
  timer shared by every airbrush tool, and the render crate's GPU policy and
  parity tests.
