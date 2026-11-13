# Test plan

This plan covers `canvas-geometry-out-of-core` (#1294):

- the new capability `document/out-of-core-canvas-geometry`;
- the modified requirements of `document/image-resize-memory-budget`,
  `document/long-operation-progress` and `verification/memory-benchmarks`;
- the requirement added to `interop/agentic-control`.

**Engine fixtures.** The engine tests use seeded documents of 600 × 300 and
larger, so every plane spans more than one strip and more than one block. The
dense paths stay as `#[cfg(test)]` oracles:

- `extend_channel` and `native_store::extend_samples` for Canvas Size, plus a
  dense Background extension;
- `orient::remap_plane` and `native_store::remap_samples` for the turns and
  flips;
- `rotate_in`'s dense `resample_samples` for Rotate.

The spill test configures a temporary scratch volume and runs
`eviction::run_pass`.

**Gate and Qt fixtures.** The gate tests pin the host's available memory and
the scratch volumes' available bytes. The Qt refusal tests pin 260 MiB of
available memory, which leaves 4 MiB of room and is under one worker's window,
and no scratch space. The scratch-full row pins 2 GiB (1.75 GiB of room) and no
scratch, so the working set fits and the planes do not. None of them depends on the host.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ocg_canvas_exact` | Canvas Size of a seeded 600 × 300 document with an alpha channel and a Background to 900 × 700 Center, 300 × 200 Bottom Right, and 601 × 299 Top Left; extend the Background in a colour | Alpha and Background channels tiled and equal to the dense paths; other layers' planes `shares` their old planes | `pictura-render::tiled_canvas_size_matches_the_dense_paths` |
| `ocg_canvas_deep` | Canvas Size of a 16-bit and a 32-bit document with a three-plane document store to 900 × 700 Center | Store tiled and equal to `extend_samples` | `pictura-render::the_document_store_shifts_through_tiles` |
| `ocg_canvas_uniform` | Canvas Size of a 1024² white-Background document to 2048² Center, extended in white | Each Background channel allocates under a quarter of the dense plane | `pictura-render::a_uniform_extension_holds_no_allocation` |
| `ocg_orient_exact` | 90° CW, 90° CCW, 180°, Flip H and Flip V of a seeded 600 × 300 document (full layer, offset masked layer, alpha) | Every plane tiled and equal to the dense remap | `pictura-render::tiled_turns_and_flips_match_the_exact_remap` |
| `ocg_orient_deep` | 90° CW and Flip H of 16- and 32-bit documents with a layer store (with `-2`) and a document store | Every retained plane tiled and equal to `remap_samples` | `pictura-render::retained_stores_turn_through_tiles` |
| `ocg_rotate_exact` | Rotate a seeded 600 × 300 document by 15°, 45°, −30°, 123.4° and 270.5° about the centre, and by 33° about (100, 50) | Every plane tiled and equal to the dense inverse map | `pictura-render::tiled_rotate_matches_the_dense_inverse_map` |
| `ocg_rotate_deep` | Rotate 16- and 32-bit documents with a layer store (with `-2`) and a document store by 20° | Every retained plane tiled and equal to the dense map | `pictura-render::retained_stores_rotate_through_tiles` |
| `ocg_output_spills` | Rotate a seeded 1024² document 15° with a scratch volume; run eviction passes | Some tiles spilled; the layer reads back equal to the dense map | `pictura-render::rotated_tiles_spill_and_read_back` |
| `ocg_memory_max` | `python3 scripts/profile-tools.py --scenarios M16,M17,M18 --memory-max 2G`, then `--memory-max 4G`, at 17000² | `outcome: ok` in every run, with no OOM kill or refusal | manual: needs a 17000² document under a systemd cgroup memory limit and several minutes of a real app; run for this change and recorded in `docs/dev/STATE.md` |
| `ocg_gate_spills` | Gate a 17000² document with a Background and a full-canvas layer: Canvas Size to 150 %, 90° turn, and 15° Rotate at 1.75 GiB room and 64 GiB scratch | Each admitted; `expects_spill` holds | `pictura-app::large_canvas_commands_are_admitted_when_scratch_holds_them` |
| `ocg_gate_scratch_full` | Gate the same three at 1.75 GiB room and no scratch | Each refused; `edit_refused_for_scratch_full` | `pictura-app::full_scratch_disks_refuse_large_canvas_commands` |
| `ocg_canvas_refused` | New 40×20 document; pin 260 MiB and no scratch; answer the Canvas Size dialog with 30000 × 30000 | Size and history unchanged; the status bar says "resize the canvas" and names memory | `tst_image_ops::canvasSizeRefusalNamesMemory` |
| `ocg_rotate_refused` | New 40×20 document; the same pins; dispatch Rotate 90° CW, Flip Horizontal, and Arbitrary 15° | Document and history unchanged; the status bar names rotating or flipping the canvas and memory | `tst_image_rotate::rotationRefusalsNameMemory` |
| `ocg_scratch_message` | Open an opaque 40×20 PNG (a Background); pin 2 GiB and no scratch; ask `canvas_size_fits_budget` about 30000 × 30000; read `commitRefusalMessage` | False; the message names the scratch disks | `tst_image_ops::canvasSizeNamesFullScratchDisks` |
| `rs_shared_budget` | Shared predicate against synthetic budgets; the resize bound against low, high and unknown room | Sides first, byte limits enforced, inside accepted, the resize bound drops the floor | `pictura-app::budget_judges_sides_then_bytes`, `pictura-app::budget_honours_the_document_limit`, `pictura-app::a_resize_document_limit_drops_the_allocation_floor` |
| `rs_canvas_side_limit` | Pin 64 GiB available and 64 GiB scratch; ask `canvas_size_fits_budget` about 30001 × 10 | False | `tst_image_ops::canvasSizeRefusesASideOverTheLimit` |
| `lop_whole_image_job` | Dispatch Rotate 90° CW on the 2400 × 1800 fixture with the job held 800 ms; separately, check which views want a job | Window shown with a counted value; 1800 × 2400 with one `"Rotate"` state; only an armed large view wants a job | `tst_document_jobs::aRotateRunsAsAJob`, `pictura-app::a_document_job_lands_one_history_state`, `pictura-app::only_an_armed_command_on_a_large_document_leaves_the_gui_thread` |
| `lop_arbitrary_rotate_job` | Dispatch Arbitrary at 15° (dialog answered by the test driver) on the 2400 × 1800 fixture with the job held 800 ms | Window shown; size unchanged while held; on landing the size is the rotated bounds with one added `"Rotate"` state | `tst_document_jobs::anArbitraryRotateRunsAsAJob` |
| `lop_whole_image_cancel` | Dispatch Flatten on a two-layer fixture held 30 s; press Cancel; then flatten again | Layers, history count and index, and pixels unchanged; second flatten adds one state | `tst_document_jobs::cancellingAFlattenChangesNothing`, `pictura-app::a_cancelled_document_job_changes_nothing` |
| `lop_merge_job` | Start a Merge Down job on a two-layer view and land it | Outcome path is the target, count 2, one layer left, one added state | `pictura-app::a_merge_job_reports_the_merged_layer` |
| `lop_small_sync` | Dispatch Rotate on a 64 × 48 fixture with the delay seam at 800 ms | No window, returns in under 800 ms, rotated, one state | `tst_document_jobs::aSmallDocumentAppliesAtOnce` |
| `ce_run_orient` | `run_command` `image.rotate90cw` on a 60 × 40 document | 40 × 60 | `tst_control_editing::orientationRunsById` |
| `ce_run_orient_key` | `run_command` `image.flipHorizontal` with `angle` | `invalid_param` | `tst_control_editing::orientationRunsById` |
| `mb_catalogue` | `profile-tools.py --self-check` with a built app | Every M1–M18 row `outcome: ok` at 512² | `profile-tools::self_check_catalogue` |
