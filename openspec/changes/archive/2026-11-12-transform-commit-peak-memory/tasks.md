# Tasks

## 1. Tests named by the plan

- [x] 1.1 `pictura-render`: `an_integer_move_shares_the_source_planes`, `an_integer_move_matches_the_resampled_result`, `a_smoother_move_still_resamples`, `a_failed_resample_allocation_refuses_the_transform`, `an_integer_move_allocates_no_resample_buffer`.
- [x] 1.2 `pictura-app`: `commit_bytes_charges_a_move_only_for_the_composite`, `a_move_about_an_off_centre_pivot_keeps_the_planes`.

## 2. Engine

- [x] 2.1 Integer-move fast path in `transform_layer_with_map` (planes moved, mask shifted, one-unit job stage).
- [x] 2.2 `resample_plane` builds its buffers with `memory::try_plane` and returns `Option`; `transform_mask` and `transform_native` propagate the refusal.

## 3. App

- [x] 3.1 `commit_bytes` drops the layer-plane term for a whole-pixel move.
- [x] 3.2 `apply_session_transform` passes the offset through unchanged when the linear part is the identity.

## 4. Close-out

- [x] 4.1 `docs/dev/STATE.md`; ROADMAP unchanged (no roadmap item for #1290).
- [x] 4.2 `openspec validate --all --strict`, `scripts/check-test-plan.py`, one build, `verify-fast`.
- [x] 4.3 Archive the change in the PR.
