# Tasks

## 1. Tests named by the plan

- [x] 1.1 `pictura-render`: `tiled_image_size_matches_the_dense_resampler`, `retained_stores_resize_through_tiles`, `a_solid_document_resizes_into_uniform_tiles`, `resized_tiles_spill_and_read_back`.
- [x] 1.2 `pictura-app`: `a_large_image_size_is_admitted_when_scratch_holds_it`, `full_scratch_disks_refuse_a_large_image_size`.
- [x] 1.3 Qt: `tst_image_ops::imageSizeRefusesASideOverTheLimit`, `tst_image_ops::imageSizeNamesFullScratchDisks`; pin memory in the existing refusal tests and in `tst_control_editing`'s oversized resize.

## 2. Engine

- [x] 2.1 `pictura_ops::resize`: public `Taps` / `axis_taps`, `sample` over them; `OpsError::Memory`.
- [x] 2.2 `resize_tiled.rs`: strip driver, blocks, row-set windows, split.
- [x] 2.3 `resize.rs`: every plane through it; the stores' rows counted; the dense resamplers test-only.

## 3. App

- [x] 3.1 `memory_gate::image_size_cost` and `image_size_admits`.
- [x] 3.2 `image_size_apply`, `resize_image`, and the `image_size_fits_budget` bridge use the gate.
- [x] 3.3 C++: `imageSizeFitsBudget`, the control server's `image.image.size` check, and the scratch-full message.

## 4. Benchmark

- [x] 4.1 M15 in `scripts/memory_scenarios.py`; docs strings name M1–M15.
- [x] 4.2 Run M15 at 17000² under `MemoryMax=2G` and `4G`; record the results (2G: ok, 35 s, peak 1992 MiB; 4G: ok, 39 s, peak 3429 MiB).

## 5. Close-out

- [x] 5.1 `docs/dev/STATE.md`, `ROADMAP.md` if it lists the item.
- [x] 5.2 `openspec validate --all --strict`, `check-test-plan.py`, one build, `verify-fast`.
- [x] 5.3 Archive the change.
