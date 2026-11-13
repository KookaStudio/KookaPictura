# Test plan

This plan covers `transform-acceptance-scenarios` (#1290). It adds to
`interop/agentic-control`, `document/out-of-core-transform`,
`document/history-away-tiles` and `compositing/gpu-region-memory`, and modifies
`verification/memory-benchmarks`.

**Fixtures.**
- The Qt case reuses `tst_control_editing`'s 32² document with Layer 1 active
  and `paintCentreSquare`.
- The render test uses `crate::test_support::scratch_guard` and a per-process
  scratch volume, as `transformed_tiles_spill_and_read_back` does.
- The profile-tools checks run in `--self-check`. The catalogue runs at 512².

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ce_layer_digest` | Paint the centre square; digest Layer 1 twice; fill one pixel outside the square; digest again; digest path `"99"` | The first two digests are equal and the third differs; `rect` is the layer's; `"99"` is `invalid_param` | `tst_control_editing::layerDigestSeesEveryPixel` |
| `ce_key_inactive_window` | Paint the centre square; begin a Free Transform; show the main window and focus its canvas; show and activate a second top-level window; send `key` `Escape` | The application has no focus widget; the key reaches the canvas and the session ends | `tst_control_editing::keyReachesTheCanvasOfAnInactiveWindow` |
| `oct_progress_spill` | Scratch volume set; 1024 × 2304 seeded plane rotated 10° under a job with one stage of output rows; the map records permille and runs `run_pass(u64::MAX)` on the first call of each strip | At least four increasing values strictly between 0 and 1000 are recorded; spilled tiles are counted before the resample returns; the output equals the dense resampler's | `pictura-render::progress_climbs_while_tiles_spill` |
| `gpu_region_estimate` | Estimate an 8 × 8 region of two plain layers, then a group of two layers | 64 × (16 + 2 × 5) bytes; the group's per-pixel bytes count both children | `pictura-render::the_region_estimate_counts_every_upload` |
| `gpu_region_memory` | Two-layer 8 × 8 scene on a GPU device; pin the host reading to 0 available; call `composite_gpu_region_raw` | `Err(GpuError::Memory)` after the device limits and before any region-sized buffer; no refusal recorded | `pictura-render::a_region_over_the_memory_room_declines_to_the_cpu` |
| `gpu_region_reserve` | Pin 64 GiB available; `reserve_if_room(1 GiB)`; then pin 0 available and try again | The first reservation is held (`reserved` ≥ 1 GiB) until dropped; the second is `None` and records no refusal | `pictura-core::reserve_if_room_neither_evicts_nor_records` |
| `hist_away_tile` | Noisy 300² tiled plane and a clone with one sample flipped at (5, 7); compare tile 0; spill both to a per-process scratch volume with `run_pass`; compare again | `(5, 7, 1, 1)` while resident; the whole tile rect once away; both tiles still away afterwards | `pictura_app::an_away_tile_is_wholly_changed_without_a_restore` |
| `mb_catalogue` | `profile-tools.py --self-check` with a built app | Every M1–M23 at 512² has `outcome: ok` and a valid row; M22 and M23 each land one history state with no open session | `profile-tools::self_check_catalogue` |
| `mb_digest` | Compare two in-memory runs whose digests agree in M19 and differ in M20; self-check M19 with `--digest` | `same` for M19 and `differs` for M20; the M19 row carries digests for Background and Layer 1 | `profile-tools::check_digest_compare`, `profile-tools::self_check_digest` |
| `mb_transform_cgroup` | At 17000² with `--digest` and scratch on disk, run M19–M23: uncapped, `--memory-max 2G`, `--memory-max 4G`; compare each capped run with the uncapped one. Then run M21 with a small tmpfs as the only scratch volume | Every capped row is `ok` with one history state, and the pixels are `same`; the tmpfs run is refused with the full-scratch message | manual: needs a 289 MP document, cgroup limits and tens of GB of scratch, which is too large and host-dependent for CI |
