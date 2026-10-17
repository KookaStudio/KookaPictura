# Tasks

> Work the groups in order. Each group is one session: add its tests first, then
> its code, then run that group's checks. Do not run the full workspace suite
> until group 5. This is the multi-session exception to the single compile-last
> build: within a session, finish the code before compiling.

## 1. Move normalization

- [x] 1.1 Add the move-path Rust tests to the `clipping` tests module:
  `dragging_a_clipped_layer_below_its_base_releases_it`,
  `dragging_a_clipped_layer_above_a_non_member_releases_it` (base A, clipped B
  and B2, unclipped C), `moving_a_clipped_layer_into_another_run_joins_it`,
  `dropping_a_layer_inside_the_run_joins_it`,
  `moving_a_clipped_layer_to_the_top_of_its_run_keeps_it_clipped`,
  `moving_a_clipped_layer_to_the_bottom_of_its_run_keeps_it_clipped`,
  `released_layers_stay_released_when_reordered_back`,
  `moving_the_base_out_releases_its_run`,
  `moving_the_only_child_out_of_a_group_is_safe`,
  `moving_a_clipped_layer_to_the_top_of_the_document_is_safe`, and
  `arranging_a_clipped_layer_to_the_back_releases_it`; verify they compile and
  fail against the current behavior with `cargo nextest run -p pictura-render clipping`.
- [x] 1.2 Add `pub(crate) fn release_run_above`, `pub(crate) fn base_below`,
  `pub(crate) struct Moved`, and `pub(crate) fn normalize_clipping` to
  `crates/pictura-render/src/document_ops/layer_ops/clipping.rs`. Both
  normalizing helpers MUST use `first_mut`/`get` so an empty container or a move
  to the last index cannot panic. `Moved` carries `index`, `was_clipped`, and
  `same_base_below`; the doc comments state the no-base rule, the moved-node rule,
  and that `was_clipped` and the former base are read before removal. Verify the
  module compiles.
- [x] 1.3 Wire `move_path` in `properties.rs`: read `was_clipped` before removal,
  derive the former base with `base_below`, call `release_run_above` before
  removal, map the base index across removal/insertion to compute
  `same_base_below`, then call `normalize_clipping(Some(Moved))` after insertion.
  Verify the same-container move tests pass.
- [x] 1.4 Wire `move_path_to` in `properties.rs`: read `was_clipped` before
  removal, call `release_run_above` on the source before removal, call
  `normalize_clipping(None)` on the source after a cross-container removal, then
  `normalize_clipping(Some(Moved))` on the destination with the final clamped
  insertion index (`at`, not `dest_index` or `None`). Same-container moves compute
  `same_base_below`; cross-container moves pass false. Verify the cross-container
  and another-run tests pass.
- [x] 1.5 Run `cargo nextest run -p pictura-render clipping` and confirm all move
  tests and the arrange test pass.

## 2. Delete, Background conversion, and bottom insertion

- [x] 2.1 Add the removal Rust tests: `deleting_the_base_releases_its_run`,
  `deleting_a_clipped_layer_keeps_the_run`,
  `converting_the_base_to_background_releases_its_run`,
  `converting_a_grouped_base_to_background_releases_its_run`, and
  `new_clipped_layer_at_container_bottom_is_released`; verify they fail against
  the current behavior.
- [x] 2.2 Wire `delete_paths` in `properties.rs`: call `release_run_above` before
  removal and `normalize_clipping(None)` after removal. Verify the two delete
  tests pass.
- [x] 2.3 Wire `background_from_layer` in `create.rs`: call `release_run_above` on
  the source before removal, normalize the source container after removal, clear
  the moved node's own clipping flag, insert at root 0, then
  `normalize_clipping(None)` on the root. Verify both Background tests pass.
- [x] 2.4 Guard `insert_node` in `create.rs`: when the chosen insertion index is
  0, clear `node.clipping` before inserting. Verify
  `new_clipped_layer_at_container_bottom_is_released` passes.
- [x] 2.5 Run `cargo nextest run -p pictura-render clipping` and confirm the
  removal and insertion tests pass.

## 3. Reverse and ungroup

- [x] 3.1 Add the Rust tests `reverse_clears_a_clipped_bottom`,
  `ungroup_clears_a_clipped_bottom` (craft a group whose first child carries a
  clipped flag), and `ungrouping_a_base_group_releases_its_run`; verify they fail
  against the current behavior.
- [x] 3.2 Wire `reverse_paths` in `properties.rs` to call
  `normalize_clipping(None)` after reversing; verify `reverse_clears_a_clipped_bottom`
  passes.
- [x] 3.3 Wire `ungroup_paths` in `properties.rs` and `ungroup_layer` in
  `create.rs`: call `release_run_above` on the parent before removing the group,
  then `normalize_clipping(None)` after splicing. Verify both ungroup tests pass.
- [x] 3.4 Run `cargo nextest run -p pictura-render clipping` and confirm all
  reverse/ungroup tests pass.

## 4. App bridge, GUI, and stamp regression

- [x] 4.1 Route `PictureView::move_layer` in
  `crates/pictura-app/src/cxxqt_object/impl_layers.rs` through
  `pictura_render::move_path`; keep the existing history label. Document the
  shift-vs-swap semantics for `|delta| > 1` in the method comment. Verify the
  bridge compiles.
- [x] 4.2 Add the Qt Test slot `dragBelowBaseReleasesClip` to
  `crates/pictura-app/cpp/tests/tst_layers_panel_fixes.cpp`: clip a full-canvas
  fill to a shape, call `move_layer_to(fill, base, 1)`, re-resolve both rows by
  name after the move, and assert `ClippingRole` is false for the fill,
  `ClipBaseRole` is false for the shape, and the fill renders outside the shape
  again. Verify with `ctest --test-dir build -R '^tst_layers_panel_fixes$' --output-on-failure`.
- [x] 4.3 Add the Qt Test slot `legacyMoveLayerShiftsAndRefuses` to the same file:
  move a clipped layer by `-1` and assert release; move a base by more than one
  slot and assert the shift and the run release; assert a locked layer and the
  Background are refused. Verify the same ctest command.
- [x] 4.4 Add the stamp regression test `stamp_above_a_clip_run_keeps_its_placement`
  to `merge_tests.rs`: stamp above a clipped layer and above a clipped run, and
  assert the stamp lands at the anchor with the documented clipping state. Verify
  with `cargo nextest run -p pictura-render stamp`.
- [x] 4.5 Build the app and run `ctest --test-dir build -R '^tst_layers_panel_fixes$' --output-on-failure`.

## 5. Integration verification and archive

- [x] 5.1 Run `cargo fmt --all` and
  `cargo clippy --workspace --all-targets -- -D warnings`; fix any findings.
- [x] 5.2 Run `cargo nextest run --workspace` and
  `cargo test --workspace --doc`; fix any regression.
- [x] 5.3 Run `ctest --test-dir build -R '^tst_' --output-on-failure`; confirm the
  new Qt cases and the existing panel suites pass.
- [x] 5.4 Run `openspec validate --all --strict` and
  `bash scripts/verify-fast.sh`; fix any guard, test-plan, or file-size failure.
- [x] 5.5 After review, archive the change with
  `openspec archive clipping-stack-invariant` and verify `openspec list --json`
  reports no open changes.
