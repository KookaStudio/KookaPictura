# Tasks

## 1. Background floor for moves and Arrange

- [x] 1.1 Add the regression tests and update the stale assertions:
      `move_path_to_refuses_below_background`,
      `move_path_clamps_at_background_floor`,
      `move_path_refuses_a_source_below_the_background`,
      `arrange_stops_above_background`, and
      `a_group_with_a_background_keeps_it_bottom` in
      `crates/pictura-render/src/document_ops/layer_ops/tests.rs`. Extend
      `arrange_moves_within_container_and_refuses_edges` with Forward then
      Backward on the same middle layer and with Front on the top layer; extend
      `arrange_refuses_background_and_locked` to all four Arrange ops and change
      its Send to Back expectation to `[Background, normal, locked]`. In
      `move_path_clamps_and_rename_resolves`, rename the bottom top-level layer
      to `"Background"` and assert its flag stays false. If the new tests would
      push `tests.rs` past 1400 lines, create `tests_background.rs` and declare
      it from `layer_ops/mod.rs` instead. Verify they fail against the current
      code with `cargo nextest run -p pictura-render <test-name>`.
- [x] 1.2 Add the private `background_floor(container)` helper in
      `properties.rs` (highest flagged Background + 1 via `rposition`) and apply
      it in `move_path` (refuse `index < floor` or `floor > last`, otherwise
      clamp to `[floor, last]`), `move_path_to_dest` (refuse a concrete
      destination below the floor), and `arrange_target` (refuse `index < floor`
      or `floor > front`, otherwise clamp Back/Backward to `[floor, front]`).
      Verify the tests from 1.1 pass and
      `cargo nextest run -p pictura-render` is green.
- [x] 1.3 Update the rustdoc on `move_path`, `move_path_to_dest`, and
      `arrange_target` to state the floor and the malformed-source refusal, and
      verify `cargo fmt --all --check` and
      `cargo clippy -p pictura-render --all-targets -- -D warnings` pass.

## 2. One Background and no Background clipping base

- [x] 2.1 Add the regression tests in
      `crates/pictura-render/src/document_ops/layer_ops/clipping.rs`:
      `a_background_is_never_a_clipping_base` (create is refused, a normal base
      still works, `add_layer_full` with `clipping: true` above the Background
      lands unclipped, and a nested Background behaves the same),
      `reverse_releases_a_clip_on_the_background`,
      `grouping_a_clipped_layer_clears_the_new_bottom`, and
      `background_from_layer_refuses_a_second_background`. Verify they fail
      against the current code with
      `cargo nextest run -p pictura-render <test-name>`.
- [x] 2.2 In `clipping.rs`, add `clipping_base_allowed(container, index)` and
      make `can_create_clipping_mask` keep every existing guard
      (`!is_background(selected)`, `!selected.clipping`) while resolving the
      container and the true base with `base_below`, refusing when the base is
      a Background. Verify the create and insertion assertions from 2.1 pass.
- [x] 2.3 In `create.rs`, replace the three `at == 0` clipping clears in
      `insert_node` with `clipping_base_allowed`, so a requested clip with no
      legal base is dropped. Verify the New Layer assertion from 2.1 passes.
- [x] 2.4 In `normalize_clipping`, find the highest flagged Background with
      `rposition` and release the contiguous clipped run directly above it,
      after the moved-flag reconciliation. In `group_paths` (`properties.rs`)
      and `group_layer` (`create.rs`), normalize the new group's children before
      insertion and the parent container after insertion. Verify
      `reverse_releases_a_clip_on_the_background` and
      `grouping_a_clipped_layer_clears_the_new_bottom` pass and the existing
      clipping tests stay green with `cargo nextest run -p pictura-render`.
- [x] 2.5 Enforce one Background and align rename:
      `background_from_layer` refuses when any node in the document already
      carries the flag (tree walk); `rename_path` no longer sets or clears the
      Background flag and only renames. Update its rustdoc and the existing
      rename test. Verify `background_from_layer_refuses_a_second_background`
      passes and the rename assertion from 1.1 passes.
- [x] 2.6 Make `group_layer` refuse the Background, matching `group_paths` and
      the Group Layers requirement. Add an assertion to the existing group tests
      and verify `cargo nextest run -p pictura-render` is green.

## 3. Qt regression coverage

- [x] 3.1 Add `dropBelowBackgroundRefused`, `cannotClipToBackground`, and
      `moveDownStopsAtBackground` slots and bodies to
      `crates/pictura-app/cpp/tests/tst_layers_panel_fixes.cpp`.
      `dropBelowBackgroundRefused` asserts `can_move_layer_to(layer, "0", 1)` is
      false, the hover shows no drop indicator
      (`dropIndicatorShownForTest()` false), the lower-edge drop is ignored, and
      history is unchanged. `cannotClipToBackground` asserts the menu predicate
      (`clipping_can_create`) is false, `clipping_toggle` on the layer above the
      Background is false, records no history, and
      `new_layer_dialog(..., clipping=true)` above the Background lands
      unclipped. `moveDownStopsAtBackground` asserts `move_layer_path(layer, -1)`
      is false directly above the Background.
- [x] 3.2 Add `arrangeStopsAtBackground` to
      `crates/pictura-app/cpp/tests/tst_command_tree.cpp`: with a Background,
      Send to Back on the top layer lands directly above it and records one
      state; Send Backward on that layer is disabled and records nothing.
      Extend `layerArrangeStamp` with `history_count` checks around Front,
      Back, Forward, and Reverse and assert Forward/Front are disabled on the
      top layer; extend `tst_layers_panel::clippingMasks` with history checks
      for release and both Alt-clicks; extend
      `tst_layers_panel::dropAnywhereOnLayerRowReorders` to assert
      `rootDropEnabledForTest()` and that `selectedPaths()` is unchanged after
      the drop.
- [x] 3.3 Build and run the suites: `cmake --build build --target
      tst_layers_panel_fixes tst_command_tree tst_layers_panel` then
      `ctest --test-dir build -R '^tst_(layers_panel_fixes|command_tree|layers_panel)$'
      --output-on-failure`. Verify the new cases and the extended assertions
      pass.

## 4. Integration verification and archive

- [x] 4.1 Run `python3 scripts/check-test-plan.py` and
      `openspec validate --all --strict`; verify this change's test plan has a
      row for every scenario and the deltas validate.
- [x] 4.2 Run `bash scripts/verify-fast.sh`; verify fmt, clippy, Rust tests,
      C++ tests, file-size, guard, and OpenSpec checks are green. Fix any
      fallout without weakening a test.
- [x] 4.3 Archive the change with `openspec archive background-layer-invariants`
      and verify `openspec list --json` reports no active changes.
