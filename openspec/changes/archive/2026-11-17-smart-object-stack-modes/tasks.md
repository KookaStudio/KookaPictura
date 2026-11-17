# Tasks

## 1. Core model and reducer

- [x] 1.1 Write the new core tests in `crates/pictura-core/src/stack_mode.rs`: `stack_mode_defaults_and_equality`, `reduce_stack_excludes_transparent_samples_and_sets_max_alpha`, `reduce_stack_matches_each_formula`, `reduce_stack_even_median_averages_middle_samples`, `reduce_stack_guards_single_and_zero_deviation_samples`, `reduce_stack_scales_entropy_to_the_channel_range`, `reduce_stack_clamps_summation_and_moments` (they fail until 1.2-1.3 land)
- [x] 1.2 Implement `StackMode` (the 11 variants, labels, stable mode codes) and `reduce_stack` with the documented formulas, clamping, and conventions; verified by the tests in 1.1 in the single compile phase (`cargo nextest run -p pictura-core --lib stack_mode`)
- [x] 1.3 Add `SmartObject.stack_mode: Option<StackMode>` with a `None` default and inclusion in `PartialEq`, export `StackMode`/`reduce_stack` from `pictura-core`, and verify `pictura-core::stack_mode_defaults_and_equality`; `crates/pictura-core/src/lib.rs` is 1190 lines against the 1200 hard cap in `scripts/check-file-size.sh`, so keep the edit to the field, the `Default` arm, the `PartialEq` arm, and module/export lines (or split `SmartObject` into its own module first)

## 2. Render integration

- [x] 2.1 Write the new render tests in `crates/pictura-render/src/tests/smart_object_stack.rs` and `crates/pictura-render/src/layer_source/stack_tests.rs`: `stack_frames_are_visible_top_level_layers`, `stack_frames_keep_the_embed_nesting_bound`, `stack_reduction_matches_the_whole_source`, `stack_mode_unknown_filter_keeps_the_proxy`, `stack_mode_mean_then_maximum_is_not_cumulative`, `set_stack_mode_sets_and_clears`, `set_stack_mode_refuses_fewer_than_two_frames`, `set_stack_mode_refuses_ineligible_targets`, `stack_mode_renders_the_per_pixel_median`, `stack_mode_applies_smart_filters_after_reduction`, `stack_mode_falls_back_for_an_unusable_stack`, `clearing_a_stack_mode_restores_the_proxy`, `rasterize_bakes_the_stack_render`, `stack_mode_overrides_the_proxy`, `stack_mode_change_invalidates_the_cached_source`, `stack_mode_change_invalidates_the_composited_raster`
- [x] 2.2 Add `composite_region::composite_layer_region`, a one-layer plan built with `CompositePlan::build_for_layers`, and verify it by the frame test in 2.1
- [x] 2.3 Add `layer_source/stack.rs` with the banded frame composition and reduction, reusing a shared decode-plus-nesting helper (the loop now in `decode_embedded`) so frames cannot bypass `MAX_EMBED_NESTING`, make `embedded_source` branch on `stack_mode`, and add the mode to `embedded_key` and `chain_hash`; verified by the median, frame, non-cumulative, cache, `stack_frames_keep_the_embed_nesting_bound`, and `stack_reduction_matches_the_whole_source` tests in 2.1 (the latter compares the banded path with a small whole-source reduction) plus the existing `a_deeply_nested_payload_renders_on_a_small_pool_stack` fixture
- [x] 2.4 Add `stack_source_applies` (a mode is active and no enabled Smart Filter fails to decode), return `ContentRoute::SmartSource` under it, and let `Body::SmartSource` prefer the source over a proxy only under it; an undecodable enabled filter keeps the normal proxy path; verified by `stack_mode_overrides_the_proxy`, `clearing_a_stack_mode_restores_the_proxy`, and `stack_mode_unknown_filter_keeps_the_proxy`
- [x] 2.5 Add `document_ops/layer_ops/smart_object_stack.rs` with `can_set_smart_object_stack_mode` and `set_smart_object_stack_mode` (eligibility, ≥2 frames, clear for `None`, refusal without mutation) and export them; verified by the set/refuse tests in 2.1
- [x] 2.6 Make `rasterize_smart_object` materialize the reduced source when a mode is active even with a colour channel, and drop the mode marker with the object; verified by `rasterize_bakes_the_stack_render`

## 3. Codec marker

- [x] 3.1 Write the codec tests in `crates/pictura-codec/src/stack_marker.rs`: `stack_mode_marker_round_trips`, `stack_mode_marker_clears`, `unknown_stack_mode_marker_is_preserved`, `unknown_stack_mode_marker_is_replaced_by_a_modelled_mode`, `stack_mode_leaves_the_proxy_untouched`
- [x] 3.2 Add the `PiSt` marker module (version 1 plus a mode code) and wire it into `smart_object.rs` (consume a recognized block into `stack_mode`) and `write_layers.rs` (author at most one `PiSt` per layer: replace a preserved marker when a mode is modelled, re-emit an unrecognized block only when no mode is modelled, and remove the authored block on clear); verified by the tests in 3.1 in the compile phase

## 4. App command surface

- [x] 4.1 Add Qt Test slots `stackModeMenuRows`, `stackModeStateFollowsTheLayer`, `stackModeRecordsOneState`, `stackModeRefusalRecordsNothing` to `crates/pictura-app/cpp/tests/tst_smart_object_actions.cpp`, and extend `tst_edition_gating.cpp` with the 12 stack-mode ids
- [x] 4.2 Replace the placeholder catalogue row with the 12 leaves under `Layer / Smart Objects / Stack Mode`, add their `const_names`, map all 12 ids to `Edition::Extended`, run `python3 scripts/gen-command-ids.py`, and verify `python3 scripts/gen-command-ids.py --check`; update `extended_only_command_ids` in `crates/pictura-script/src/command_catalogue.rs` for the 12 new ids (or assert the set), because it currently hardcodes the 7 Extended ids
- [x] 4.3 Add the cxx-qt bridge for the current mode code, eligibility, and one apply/clear operation that recomposites and records `Stack Mode: <label>`; the bridge compares the current mode and records nothing when the requested mode is already active; verified by the undo and refusal slots in 4.1
- [x] 4.4 Wire the 12 actions in `crates/pictura-app/cpp/menu_smart_object.cpp` with a handler, enabled provider, and checked provider per leaf; verified by the menu, state, undo, and edition slots from 4.1

## 5. Docs and parity

- [x] 5.1 Update the parity register (`LAY-13` to `SHIPPED` with evidence), tick `Smart-object Stack Modes (#591)` in `ROADMAP.md`, drop stack modes from the open list in the `ROADMAP.md:36` summary row, and record the change in `docs/dev/STATE.md`; when editing `docs/dev/doc-code-parity-gaps.md`, add a `Re-checked: #591` note and adjust the `157 gaps` count at line 12; commit the docs separately with `TASK-ALLOWS-DOCS` and verify `GUARD_BASE=origin/master bash scripts/guard.sh` passes for that commit

## 6. Integration and archive

- [x] 6.1 Run `openspec validate smart-object-stack-modes --strict` and `python3 scripts/check-test-plan.py`; fix any scenario or coverage gap
- [ ] 6.2 Run `bash scripts/verify-full.sh` once after all code is written; verify the new Rust tests, Qt suites, file-size, guard, and OpenSpec gates pass
- [x] 6.3 Archive the change with `openspec archive smart-object-stack-modes`, then verify `bash scripts/check-openspec-archived.sh` and `openspec validate --all --strict`
- [x] 6.4 Commit the implementation and the docs commit separately (every message ends `(#591)`), and open the PR titled `feat(layers): extended Stack Modes for smart objects (#591)` with the test plan's manual rows (none) in the body
