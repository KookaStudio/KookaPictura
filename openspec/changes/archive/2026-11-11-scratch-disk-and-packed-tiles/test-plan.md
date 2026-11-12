# Test plan

This plan covers `document/scratch-disk` and `document/tile-eviction` for
`scratch-disk-and-packed-tiles` (#1217).

**Fixtures.**

- **Scratch tests** open a store on a fresh directory under the test's
  temporary directory (never `/tmp`, which is tmpfs here) and drive its faults
  with `scratch::inject`.
- **Eviction tests** pin the governor's host reading (`set_host_for_test`) and
  run passes explicitly through `eviction::run_pass`, rather than waiting on the
  background thread. Their planes are seeded random data. All of them hold a
  process-wide test lock, because the registry and fault switches are global.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `sd_round_trip` | Write seeded payloads of 1 B to 300 KiB to a store; read each back | Every read equals the bytes written | `pictura-core::scratch_round_trip` |
| `sd_corrupt` | Arm `Corrupt` and write; arm `ReadError` and read | Both reads return `ScratchError::Unreadable` | `pictura-core::scratch_corrupted_extent_is_unreadable` |
| `sd_free_reuse` | Write ten payloads, free them, write ten of the same sizes | The file does not grow; used returns to the payloads' total | `pictura-core::scratch_frees_and_reuses_extents` |
| `sd_fallthrough` | Configure two volumes; arm `Full` on the first; write | The write lands on volume 1 and reads back | `pictura-core::scratch_full_volume_falls_through` |
| `sd_all_full` | Arm `Full` on every volume; write | `ScratchError::Full`; earlier extents still read | `pictura-core::scratch_all_full_is_typed` |
| `sd_removed_volume` | Two volumes; write to the first; arm `RemoveVolume(0)`; write and read | The new write lands on volume 1; the old extent reads `Unreadable` | `pictura-core::scratch_removed_volume_falls_through` |
| `sd_default_volume` | Resolve the default with `XDG_CACHE_HOME` set; probe a `/dev/shm` volume | The default is `$XDG_CACHE_HOME/kooka-pictura/scratch`; the `/dev/shm` row is flagged tmpfs | `pictura-core::scratch_default_volume_and_tmpfs_flag` |
| `sd_crash_clean` | Re-run the test binary as a child that spills and blocks; SIGKILL it; open the volume again | No file left in the volume directory; the new store works | `pictura-core::scratch_sigkill_leaves_no_file` |
| `sd_efficiency` | Run an operation with 20 ms of work and 10 ms of recorded I/O; read before and after | Both counters monotonic; compute ≈ 20 ms, I/O ≈ 10 ms, Efficiency ≈ 67 % | `pictura-core::scratch_efficiency_accounting` |
| `sd_sizes` | Write payloads, read `sizes()`, free, read again | Used grows by their aligned sizes, then returns to 0 | `pictura-core::scratch_sizes_follow_extents` |
| `sd_status` | Ask the control server for `status` | `memory.scratch.volumes`, `used`, `available`, `efficiency.percent`, and the tile counts are present | `pictura-app::the_status_json_carries_scratch_readouts`, `tst_memory_governor::statusCarriesScratchReadouts` |
| `te_identity` | Seeded 8/16/32-bit planes; pack every dense tile, then spill; read through tile, row, rect, and sample paths | Bit-identical to the original | `pictura-core::evict_restore_identity` |
| `te_pinned` | Hold a `tile_at` borrow; run a pass that must evict everything | The borrowed tile stays resident; the borrow reads the original | `pictura-core::a_pinned_tile_is_not_evicted` |
| `te_write_restore` | Spill a tile, write it through `tiles_mut`, evict it again, read it | The written samples come back | `pictura-core::a_write_after_restore_survives_eviction` |
| `te_order` | History-held and live bodies; run a pass needing less than the history bodies hold | Only history-held bodies are packed | `pictura-core::history_tiles_are_evicted_before_live_ones` |
| `te_thresholds` | Pin the host at 99 %, then 101 % of the ceiling; run a pass each time | Nothing at 99 %; at 101 %, frees down to ≤ 90 % | `pictura-core::eviction_starts_above_the_ceiling_and_stops_at_ninety` |
| `te_clock_hot` | Touch tile A; touch tile B under a hot hold; run two passes; drop the hold; run a pass | A survives pass 1 and goes in pass 2; B goes only after the hold drops | `pictura-core::second_chance_and_hot_holds` |
| `te_urgent` | Pin the host at 130 %; allocate a new dense tile | Cold tiles are packed on the allocating thread before it returns | `pictura-core::urgent_pressure_evicts_on_the_allocating_thread` |
| `te_display_drop` | Pin the host over the ceiling; run a pass | The display-drop epoch advances before any tile is packed | `pictura-core::a_pass_asks_the_display_to_drop_first`, `pictura-app::the_display_cache_drops_on_request` |
| `te_clean` | Spill a tile; read it; evict it; then write it and evict | The second eviction writes 0 bytes; the third writes an extent | `pictura-core::a_clean_tile_drops_without_a_write` |
| `te_reclaim` | Pin a room short of a reservation by less than the cold tiles hold; reserve | Cold tiles are packed; the reservation is granted | `pictura-core::reclaim_turns_a_refusal_into_a_grant` |
| `te_edit_unreadable` | Spill a layer's tiles; arm `ReadError`; apply Invert to the layer | The document and history are unchanged; `edit_refused_for_scratch` | `pictura-app::an_edit_on_unreadable_tiles_changes_nothing` |
| `te_save_unreadable` | Make a tile unreadable; save as PSD | `UnreadableTiles` error; no file written | `pictura-app::save_refuses_unreadable_tiles` |
| `te_idle` | Pin the host at 50 %; edit a document; run passes | Packed and spilled counts stay 0 | `pictura-core::no_pressure_no_eviction` |
| `te_memorymax` | `profile-tools.py --scenarios M1..M13` under `systemd-run --user -p MemoryMax=2G` and `4G`, and unconstrained against master `b017d378`; brush dab latency p50/p99 | No OOM kill or refusal (M12: #1273); unconstrained within 3 %; p99 reported | manual: needs the reference machine, a systemd user scope, and 17000² fixtures; CI runners are too small and noisy for the timing bounds |
| R7 codec benchmark | `PICTURA_PACK_BENCH_PSD=… cargo test --release -p pictura-codec --test pack_codec_benchmark -- --ignored --nocapture` | Throughput and ratio for `lz4_flex` and `flate2`, recorded in `design.md` | manual: a timing measurement on the reference machine, not a pass/fail check |
| R8 I/O benchmark | `cargo test -p pictura-core --release -- scratch_io_benchmark --ignored --nocapture` | `pread`/`pwrite` against `mmap` throughput, recorded in `design.md` | manual: a timing measurement on a real disk |
| Slow disk | Point scratch at a USB stick or a throttled loop device; paint and pan a 17000² document under `MemoryMax=2G` | It stays responsive; Efficiency drops below 95 % and recovers | manual: needs real slow hardware |
