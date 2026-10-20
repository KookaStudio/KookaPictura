# Tasks

## 1. Tests first

- [x] 1.1 Add `merge_down_bakes_an_adjustment_source` and
  `merge_layers_bakes_an_adjustment_above_the_bottom` to
  `crates/pictura-render/src/document_ops/layer_ops/merge_tests.rs`; extend
  `can_merge_scope_matches_the_merge_guards` for the source case; keep the
  existing target-refusal tests
- [x] 1.2 Add `adjustmentSourceMergesDown`, `adjustmentAboveBottomMergesLayers`,
  `adjustmentTargetStillRefused`, `mergeVisibleSelectsMergedResult`,
  `mergeDownSelectsResult`, `mergeLayersSelectsResult`, and
  `shortcutFollowsSelection` to
  `crates/pictura-app/cpp/tests/tst_merge_commands.cpp`

## 2. Engine guard

- [x] 2.1 In `merge.rs`, validate only the lower layer in `merge_down` and
  `can_merge_down`; leave the bottommost-selection checks in `merge_selected`
  and `can_merge_selected` unchanged

## 3. Merge result selection

- [x] 3.1 Change `merge_scope_changed` to expose the `MergeOutcome`; return the
  merged path as `QString` from `merge_layers` and `merge_visible`, and update
  the declarations in `cxxqt_object.rs`
- [x] 3.2 In `frame_menus.cpp` and `layers_panel_menu.cpp`, rebuild the panel
  rows and select the returned path after a successful merge; adapt
  `selftest_layers_controls.cpp` to the new return type

## 4. Live shortcut enablement

- [x] 4.1 Call `registry_->refresh()` from the Layers panel `selectionChanged`
  handler in `frame_build.cpp`

## 5. Verification and archive

- [x] 5.1 Run `cargo fmt --all`, targeted `pictura-render` merge tests, clippy,
  `tst_merge_commands`, `python3 scripts/check-test-plan.py`, and
  `openspec validate --all --strict`
- [x] 5.2 Build and run the focused Qt suite, then `bash scripts/verify-full.sh`;
  archive the change with `openspec archive merge-source-selection-shortcut` and
  re-run `openspec validate --all --strict`
