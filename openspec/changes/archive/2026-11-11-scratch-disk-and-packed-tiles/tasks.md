# Tasks

## 1. Tests named by the plan

- [x] 1.1 `pictura-core` scratch: `scratch_round_trip`, `scratch_corrupted_extent_is_unreadable`, `scratch_frees_and_reuses_extents`, `scratch_full_volume_falls_through`, `scratch_all_full_is_typed`, `scratch_removed_volume_falls_through`, `scratch_default_volume_and_tmpfs_flag`, `scratch_sigkill_leaves_no_file`, `scratch_efficiency_accounting`, `scratch_sizes_follow_extents`.
- [x] 1.2 `pictura-core` eviction: `evict_restore_identity`, `a_pinned_tile_is_not_evicted`, `a_write_after_restore_survives_eviction`, `history_tiles_are_evicted_before_live_ones`, `eviction_starts_above_the_ceiling_and_stops_at_ninety`, `second_chance_and_hot_holds`, `urgent_pressure_evicts_on_the_allocating_thread`, `a_pass_asks_the_display_to_drop_first`, `a_clean_tile_drops_without_a_write`, `reclaim_turns_a_refusal_into_a_grant`, `no_pressure_no_eviction`.
- [x] 1.3 `pictura-app`: `the_status_json_carries_scratch_readouts`, `the_display_cache_drops_on_request`, `an_edit_on_unreadable_tiles_changes_nothing`, `save_refuses_unreadable_tiles`.
- [x] 1.4 Qt: `tst_memory_governor::statusCarriesScratchReadouts`.
- [x] 1.5 Ignored benchmarks: `pictura-codec` `pack_codec_benchmark`, `pictura-core` `scratch_io_benchmark`.

## 2. Core

- [x] 2.1 `scratch`: volumes, unnamed file, extent allocator, CRC-32, typed errors, fault injection, sizes, efficiency.
- [x] 2.2 Tile bodies: residency, pins, restore on touch, pack and spill, clean extents, unreadable state; access paths in `plane/access.rs` and `tiled_plane`.
- [x] 2.3 `memory::eviction`: registry, clock, hot holds, history holds, thread, thresholds, urgent path, display-drop epoch, reclaim before refusal; `cgroup_file` in the host reading.

## 3. App

- [x] 3.1 Start eviction; session `scratchVolumes`; bridge readouts; `status` fields.
- [x] 3.2 History holds on delta tiles; hot holds for strokes and view builds; display drop on request.
- [x] 3.3 Edit rollback on unreadable reads; save refusal.

## 4. Verification and close-out

- [x] 4.1 R7/R8 benchmarks; dependency decision recorded in `design.md` and the commit.
- [x] 4.2 fmt, clippy, nextest, CMake build, Qt suites (8409/8410; `tst_open_save::rasterSaveKeepsFileInfo` needs the Qt TIFF plugin, absent locally).
- [x] 4.3 M1–M13 under `MemoryMax=2G`/`4G` and unconstrained; results in `design.md` (M12 refused at the open, #1273; dab p99 under pressure not reachable by any M scenario, left manual).
- [x] 4.4 `scratch-disks-and-memory.md` (`TASK-ALLOWS-DOCS` commit); `ROADMAP.md`; `STATE.md`.
- [x] 4.5 `openspec validate --all --strict`; archive.
