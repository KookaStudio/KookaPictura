# Tasks

## 1. Grayscale-native mask edit document

- [x] 1.1 Added `mask_document_coverage` (`mask_edit.rs`): a one-layer, one-channel (`id == 0`) grayscale document whose channel 0 is the coverage, alongside the RGBA `mask_document` filters/fills keep. `begin_mask_paint` uses it; `write_mask_back` copies the single plane unchanged.
- [x] 1.2 `coverage_and_rgba_documents_paint_the_same_coverage` runs the exact `Stroke` over both forms and asserts channel 0 matches within ±1 LSB; `a_coverage_document_is_single_channel` pins the shape. Existing `mask_document`/`write_mask_back` tests stay green.

## 2. Mask stroke acceleration parity

- [x] 2.1 `begin_mask_paint` runs `preview_level(diameter, mask_w, mask_h)` and, over the raster budget, seeds the GPU stroke the pixel path uses. The CPU exact raster remains the oracle and the fallback.
  - Deferred: a reduced-level **CPU** preview for masks. A mask preview must show the composited result at level N, which needs a level-N composite the engine has no path for; the GPU path covers the large-brush accelerator case, and CPU-only large brushes stay exact (now memory-optimal). Tracked as a follow-up.
- [x] 2.2 `GpuStrokeParams.channels` + `params[23]` reach the shader, which blends byte 0 only when `channels == 1`; the seed replicates coverage across RGB at opaque alpha. `gpu_mask_stroke_matches_the_cpu_oracle` (`#[ignore]`) asserts ±1 LSB.
- [x] 2.3 `mask_dab` dispatches to `mask_gpu_dab` (`impl_paint/mask_gpu.rs`) when the GPU stroke is seeded, patching the mask document's channel 0 and writing the region back; the exact CPU path is unchanged.

## 3. Live present during a mask drag

- [x] 3.1 `is_painting` (`impl_paint.rs`) counts a live `mask_stroke`, so the shell's `regionBlitted` slot schedules `flush_present` mid-drag. It previously checked the pixel stroke only, so a mask stroke presented the first dab and dumped the rest at commit. `tst_mask_edit.cpp` asserts a mask stroke reports `is_painting` and that a mid-drag flush presents the accumulated region.

## 4. Measurement

- [x] 4.1 `mask_dab_profile_4000` (`#[ignore]`) prints per-dab RGBA-vs-coverage raster timing; run in release it reports 9.06 ms → 7.55 ms. Numbers recorded in `design.md`.

## 5. Verification

- [x] 5.1 `cargo nextest run -p pictura-render` (997 passed), the Qt `tst_mask_edit`/`tst_layers_panel`/`tst_edit_shortcuts` suites, and `openspec validate --all --strict` pass.
- [ ] 5.2 `bash scripts/verify-fast.sh` — rerun after the `mask-solo-edit` docs are committed with `TASK-ALLOWS-DOCS` (its guard rejects the uncommitted docs of that change).
