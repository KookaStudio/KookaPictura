# Tasks

## 1. Tests named by the plan

- [x] 1.1 `pictura-render`: `tiled_warp_matches_the_dense_rasterizer`, `deep_stores_warp_through_tiles`, `a_solid_layer_warps_into_uniform_tiles`, `warped_tiles_spill_and_read_back`, `a_cancelled_warp_changes_nothing`.
- [x] 1.2 `pictura-app`: `a_large_warp_is_admitted_when_scratch_holds_it`.
- [x] 1.3 Qt: `tst_control_editing::freeTransformWarpsTheLayer`.

## 2. Engine

- [x] 2.1 `warp_oracle.rs`: the dense rasterizer, moved unchanged, test-only.
- [x] 2.2 `warp_raster.rs`: windowed `Planes`, block rasterizer, strip driver into tiled `Samples`.
- [x] 2.3 `warp.rs`: layer, mask and retained-store passes through the tiled rasterizer.

## 3. App

- [x] 3.1 `warp_strip_bytes` and the warp commit's gate.
- [x] 3.2 `layer_op free_transform` takes `warp` and `bend`.

## 4. Benchmark

- [x] 4.1 M14 in `scripts/memory_scenarios.py`; docs strings name M1–M14.
- [x] 4.2 Run M14 at 17000² under `MemoryMax=4G`; record the result (ok, 64 s, peak 3954 MiB).

## 5. Close-out

- [x] 5.1 `docs/dev/STATE.md`.
- [x] 5.2 `openspec validate --all --strict`, `check-test-plan.py`, one build, `verify-fast`.
- [x] 5.3 Archive the change.
