# Tasks

## 1. Tests named by the plan

- [x] 1.1 `pictura-render`: `tiled_canvas_size_matches_the_dense_paths`, `the_document_store_shifts_through_tiles`, `a_uniform_extension_holds_no_allocation`, `tiled_turns_and_flips_match_the_exact_remap`, `retained_stores_turn_through_tiles`, `tiled_rotate_matches_the_dense_inverse_map`, `retained_stores_rotate_through_tiles`, `rotated_tiles_spill_and_read_back`.
- [x] 1.2 `pictura-app`: `large_canvas_commands_are_admitted_when_scratch_holds_them`, `full_scratch_disks_refuse_large_canvas_commands`.
- [x] 1.3 Qt: `tst_image_ops::canvasSizeRefusalNamesMemory`, `tst_image_ops::canvasSizeNamesFullScratchDisks`, `tst_image_ops::canvasSizeRefusesASideOverTheLimit`, `tst_image_rotate::rotationRefusalsNameMemory`, `tst_document_jobs::anArbitraryRotateRunsAsAJob`, `tst_control_editing::orientationRunsById`.

## 2. Engine

- [x] 2.1 `geometry_tiled.rs`: strip driver; shift, orient and rotate kernels.
- [x] 2.2 `canvas.rs`: alpha channels, document store and Background through shift, on a clone; `extend_background` returns `Result`.
- [x] 2.3 `orient.rs`: every plane through orient on a clone; `flip_document` returns `Result`; progress in rows.
- [x] 2.4 `rotate_in.rs`: `sample_with`; every plane through rotate on a clone; progress in rows.
- [x] 2.5 Plane-byte estimators: `canvas_plane_bytes`, `oriented_plane_bytes`, `rotated_plane_bytes` beside `resized_plane_bytes`.

## 3. App

- [x] 3.1 `memory_gate::geometry_cost`; `image_size_cost` through it.
- [x] 3.2 `canvas_size_admits`, `orient_admits`, `rotate_admits`; the `canvas_size_fits_budget` bridge.
- [x] 3.3 Gate `canvas_size_apply`, `resize_canvas`, `rotate_doc`, `flip_doc`; arbitrary Rotate as a counted `document_op`.
- [x] 3.4 C++: `commitRefusalMessage`; Canvas Size, orientation and Arbitrary handlers report refusals; Arbitrary runs through `runDocumentCommand`; the control server's canvas check and the orientation ids.

## 4. Benchmark

- [x] 4.1 M16–M18 in `scripts/memory_scenarios.py`; docs strings name M1–M18.
- [x] 4.2 Run M16–M18 at 17000² under `MemoryMax=2G` and `4G`; record the results.

## 5. Close-out

- [x] 5.1 `docs/dev/STATE.md`, `ROADMAP.md` if it lists the item.
- [x] 5.2 `openspec validate --all --strict`, `check-test-plan.py`, one build, `verify-fast`.
- [x] 5.3 Archive the change.
