# Tasks

## 1. Tests named by the plan

- [x] 1.1 Qt: `tst_control_editing::layerDigestSeesEveryPixel`, `keyReachesTheCanvasOfAnInactiveWindow`.
- [x] 1.2 `pictura-render`: `progress_climbs_while_tiles_spill`.
- [x] 1.3 `profile-tools`: `check_digest_compare`, `self_check_digest`; the catalogue covers M19–M23.

## 2. App

- [x] 2.1 `layer_pixel_digest` (declared in `layer_target_bridge.rs`, implemented in `layer_target_bridge/digest.rs`) and its `layer_digest` control method (Read tier).
- [x] 2.2 `key` falls back to the main window's focus child when no window is active, so a 17000² transform's Return reaches the canvas.

## 3. Engine

- [x] 3.1 `Governor::reserve_if_room`; the GPU region composite reserves its host bytes or declines with `memory` (tests `a_region_over_the_memory_room_declines_to_the_cpu`, `reserve_if_room_neither_evicts_nor_records`).

- [x] 3.2 History captures record packed or spilled tiles as wholly changed without restoring them (test `an_away_tile_is_wholly_changed_without_a_restore`).

## 4. Harness

- [x] 4.1 M19–M23 in `scripts/memory_scenarios.py`.
- [x] 4.2 `--digest` and the `pixels` compare column in `scripts/profile-tools.py`.

## 5. Close-out

- [x] 5.1 Run the 17000² acceptance runs (`mb_transform_cgroup`) and post them on #1290.
- [x] 5.2 Docs: `docs/dev/STATE.md`; ROADMAP unchanged (no roadmap item for #1290).
- [x] 5.3 `openspec validate --all --strict`, `scripts/check-test-plan.py`, one build, `verify-fast`.
- [x] 5.4 Archive the change in the PR.
