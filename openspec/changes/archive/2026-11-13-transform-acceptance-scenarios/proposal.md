# Proposal

## Why

Issue #1290. The code phases have merged (#1291, #1295, #1296, #1302, #1303),
and #1292–#1294 carried warp, Image Size and Canvas Size / Rotate. These
acceptance items are still unproven:
- the 17000² runs under `MemoryMax=4G`;
- a pixel-for-pixel match between a commit that spilled and the same commit in
  RAM;
- Perspective and Distort under a hard limit;
- progress that keeps advancing while a commit spills;
- a live refusal when the scratch disks are full.

#1303's 2G runs were driven by hand, and its "pixel-grid hash" left no
repeatable tool behind. Nothing reads a layer's pixels without flattening the
document. `export_png` builds one full-size image, which is about 1.2 GB at
17000², so a capped app would be killed by the check itself.

## What Changes

- **A `layer_digest` control method (Read tier).** It streams one layer's
  channel and mask rows through a 64-bit FNV-1a digest, reading spilled tiles
  back, and never builds a full-size image.
- **Scenarios M19–M23 in the memory benchmark.** Each is a full-layer gradient:
  Free Transform ×0.5, ×1.0 + 15°, and ×1.5, then Perspective and Distort
  through the real corner drag.
- **`profile-tools.py --digest`.** It records each layer's digest after the
  measured steps. `--compare` reports whether the digests match, so a capped
  run can be checked against an uncapped one.
- **`key` reaches the canvas when no window is active.** At 17000² the
  Return that commits Perspective and Distort was sent to the main window,
  because offscreen no window is active after a long step, so it was lost. The
  key now goes to the main window's focus child first.
- **A GPU region composite reserves its host memory or declines.** After a
  17000² Perspective, Distort or 90 % scale commit landed under
  `MemoryMax=2G`, the deferred composite took the GPU path, whose canvas,
  readback and source buffers (more than 5 GB at that size, all host memory on a
  unified-memory GPU) got the app OOM-killed. It now reserves those bytes with
  the governor first and, when they do not fit, declines to the banded CPU path.
- **A history capture no longer reads back evicted tiles.** When the
  transformed layer keeps its rect (Perspective, Distort, a 90 % drag), history
  compared every old and new tile, restoring packed tiles on both sides faster
  than eviction re-packed them. An away tile now counts as wholly changed.
- **A render test.** It checks that a tiled transform's progress climbs strip
  by strip while its tiles spill.

## Capabilities

- **ADDED** `interop/agentic-control`: "A layer's pixels can be digested" and
  "A control key reaches the canvas when no window is active".
- **MODIFIED** `verification/memory-benchmarks`: "The memory scenario
  catalogue" (M1–M23).
- **ADDED** `verification/memory-benchmarks`: "Pixel digests for out-of-core
  equivalence".
- **ADDED** `compositing/gpu-region-memory`: "A GPU region composite fits the
  memory room or declines".
- **ADDED** `document/history-away-tiles`: "A history capture does not read
  back evicted tiles".
- **ADDED** `document/out-of-core-transform`: "A spilling transform reports
  progress as it goes".

## Impact

- `pictura-app`: `layer_pixel_digest` in `layer_target_bridge.rs` (code in
  `layer_target_bridge/digest.rs`), plus
  `cpp/control_server_commands.cpp`, `cpp/control_server_input.cpp` and `control_server.{h,cpp}`.
- `pictura-render`: one test in `transform_tiled_tests.rs`; the GPU region
  gate in `gpu/mod.rs` and a `memory` decline reason in `metrics.rs`.
- `pictura-core`: `Governor::reserve_if_room`.
- `pictura-app`: `history/planes.rs` (`changed_or_away`).
- `scripts/memory_scenarios.py` and `scripts/profile-tools.py`.
- No new dependencies. The MCP tool list is unchanged: the digest is a
  benchmark aid, not an agent tool.
