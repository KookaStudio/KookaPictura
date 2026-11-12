# Tasks

## 1. Tests named by the plan

- [x] 1.1 `pictura-render`: `tiled_resample_matches_the_dense_resampler`, `deep_planes_resample_through_tiles`, `a_solid_layer_scales_into_uniform_tiles`, `transformed_tiles_spill_and_read_back`, `a_tiled_region_equals_the_flat_region`, `patching_a_tiled_region_does_not_materialize`.
- [x] 1.2 `pictura-app`: `a_large_resize_is_admitted_when_scratch_holds_it`, `full_scratch_disks_refuse_the_transform`, `a_working_set_beyond_ram_is_a_memory_refusal`, `a_deferred_whole_landing_skips_the_display_conversion`.
- [x] 1.3 Qt: `tst_memory_governor::transformRefusalNamesFullScratchDisks`.

## 2. Core

- [x] 2.1 `Plane::build_tiled_strips`; `build_tiled` on top of it.
- [x] 2.2 `memory::try_buffer<T>`; `try_plane` delegates to it.

## 3. Engine

- [x] 3.1 `transform_tiled.rs`: generic block-window resampler; `resample_plane` and the typed path use it; dense resampler kept as the test oracle.
- [x] 3.2 Tiled banded composite for regions of 16 Mpx or more.
- [x] 3.3 `patch_buffer_region` reads the region through `read_rect_into`.

## 4. App

- [x] 4.1 `CommitCost` and `memory_gate::admit_transform`; scratch-full refusal and its test seam.
- [x] 4.2 Lazy `RegionComposite::srgb`.
- [x] 4.3 `transformMemoryRefusalMessage()` names the scratch disks after a scratch-full refusal.

## 5. Close-out

- [x] 5.1 `docs/dev/STATE.md`.
- [x] 5.2 `openspec validate --all --strict`, `check-test-plan.py`, one build, `verify-fast`.
- [x] 5.3 Archive the change.
