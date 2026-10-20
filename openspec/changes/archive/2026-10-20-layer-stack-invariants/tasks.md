# Tasks

## 1. Validator and failing invariant tests

- [x] 1.1 Add `StackViolation` (deriving `Debug, Clone, PartialEq, Eq`) and a
      free `well_formed(&[Layer])` checker in a new
      `crates/pictura-core/src/layer_stack.rs`, exported from the crate root.
      Traverse root-first and bottom-first. Add unit tests for a valid tree, a
      clipped container bottom, an illegal clip base, and each misplaced or
      duplicate Background case. Verify with
      `cargo nextest run -p pictura-core layer_stack`.
- [x] 1.2 Add `crates/pictura-render/src/document_ops/layer_ops/tests_stack.rs`
      with a canonical small tree, register `#[cfg(test)] mod tests_stack;` in
      `layer_ops/mod.rs`, and add the op-matrix test
      `every_structural_op_leaves_a_well_formed_stack`. It asserts
      `well_formed`, that every `can_*` dry run leaves the document unchanged,
      and base identity/released sets for the base-removal cases. Verify with
      `cargo nextest run -p pictura-render tests_stack` and record which
      un-normalized sites it catches today.
- [x] 1.3 Add the targeted invariant tests in `tests_stack.rs` using the current
      API and the free checker: `merge_removing_a_base_releases_its_run`,
      `smart_object_conversion_splices_a_valid_run`,
      `insertion_and_lift_never_leave_a_baseless_clip`,
      `grouping_a_base_keeps_its_run_attached` (with and without the run),
      `grouping_a_middle_member_leaves_a_valid_stack`,
      `ungrouping_a_base_releases_its_run`,
      `reverse_keeps_the_container_bottom_unclipped`, and
      `scratch_filtering_releases_a_clipped_base`. Verify each fails against the
      current implementation where the behavior is missing.
- [x] 1.4 Create `crates/pictura-codec/src/tests/clipping_invariants.rs`,
      register it from `src/tests.rs`, and add
      `malformed_clip_round_trips_unchanged`: write a tree with a clipped
      bottom, read it back, write it again, and assert the flag survives and the
      bytes match. Verify the test passes before the type change.
- [x] 1.5 Run the existing `clipping.rs`, `tests_background.rs`, and
      `merge_tests.rs` suites as the baseline, including
      `stamp_above_a_clip_run_keeps_its_placement`. Verify with
      `cargo nextest run -p pictura-render`.

## 2. LayerStack type and mechanical migration

- [x] 2.1 Implement `LayerStack` with `from_layers`, `Debug, Clone, PartialEq,
      Default`, `Deref<Target = [Layer]>`, `Index<usize>`/`IndexMut<usize>`,
      `get_mut`/`first_mut`/`last_mut`/`iter_mut`, and `IntoIterator` for
      owned/`&`/`&mut`; switch `Document.layers` and `Layer.children` to it.
      Verify there is no `DerefMut` impl and no slice-level reordering through
      indexing.
- [x] 2.2 Migrate `crates/pictura-core` constructions (`from_rgba`,
      `Layer::default`, tests) to `from_layers`; add
      `from_layers_accepts_a_malformed_tree_unchanged` and
      `malformed_background_is_not_repaired`. Verify with `rg` that no
      `doc.layers.push` or `children: Vec` remains in the crate.
- [x] 2.3 Migrate `crates/pictura-render` assignments, fixtures, and scratch
      builders to `from_layers`, including `mode/flat.rs` `clear`,
      `mode/planes.rs` push, `place_smart_object`/`mask_document` pushes, and
      `merge.rs` scratch `extend`. Change the 16 `&mut [Layer]` and 4
      `&mut Vec<Layer>` helper signatures to `&mut LayerStack` without adding an
      `as_mut_slice` escape.
- [x] 2.4 Migrate `crates/pictura-app` (including the smart-object `pop` and
      history `pop`/`swap`), `crates/pictura-paint`, and `crates/pictura-codec`
      (including the PDF export `clear`) construction sites and test fixtures.
- [x] 2.5 Move `well_formed` onto `LayerStack`, delete the temporary free
      function, update the tests from group 1, and verify the workspace builds.

## 3. Stack primitives and layer_ops rewrite

- [x] 3.1 Write the deterministic core property test
      `property_structural_primitives_keep_the_tree_well_formed` (small-tree
      generator, fixed seed, no new dependency), the first-edit test
      `first_edit_normalizes_only_the_touched_container`, and primitive tests
      for the Background floor, `set_clipping` at an illegal position,
      `retain_tree` recursion, `splice` atomicity, and partial-run grouping.
- [x] 3.2 Implement `StackError` and `insert`, `remove`, `move_node`, `splice`,
      `replace`, `group`, `reverse`, `truncate`, `retain_tree`, `set_clipping`,
      and `promote_background`, each reconciling the invariants before
      returning, leaving the tree unchanged on `Err`, and debug-asserting only
      when the pre-edit tree was well-formed. Verify with the property test and
      the primitive tests.
- [x] 3.3 Rewrite `create.rs` (`add_layer`, `add_group`, `duplicate_layer`,
      `group_layer`, `ungroup_layer`, `insert_node`,
      `add_raster_layer_from_rgba`, `background_from_layer`) onto the stack.
      Verify with `cargo nextest run -p pictura-render`.
- [x] 3.4 Rewrite `properties.rs` (`delete_paths`, `duplicate_paths`,
      `group_paths`, `ungroup_paths`, `move_path`, `move_path_to`,
      `reverse_paths`) onto the stack; keep locks, nesting, selection, the
      Background floor, and the last-layer guard in this module.
- [x] 3.5 Rewrite `merge.rs` (`splice_result`, `bake_inputs`, `flatten`, the
      Stamp rollback) onto `splice`/`remove` so a removed base releases its run
      and the result is one atomic splice.
- [x] 3.6 Rewrite `smart_object.rs` conversion, `via.rs`, `move_content.rs`
      (`lift_selection`, `merge_lifted`), and `clipboard.rs` insertion onto the
      stack primitives.
- [x] 3.7 Delete `normalize_clipping`, `release_run_above`,
      `clipping_base_allowed`, and `Moved` from `pictura-render`; keep
      `background_floor` and `has_background` as read-only helpers, and keep
      `can_create_clipping_mask`/`release_clipping_mask` as policy over
      `set_clipping`. Verify with `rg` that no production use of the deleted
      helpers remains.
- [x] 3.8 Run the full op matrix and the existing clipping/Background suites;
      update any test that encoded the old silent re-base, and verify
      `stamp_above_a_clip_run_keeps_its_placement` still passes.

## 4. App, paint, codec, and scratch paths

- [x] 4.1 Replace direct structural writes in `pictura-app` and
      `pictura-paint` (`push` in fill/eraser/art history, `pop`, `clear`,
      `swap`, `extend`) with `insert`, `remove`, `truncate`, or `from_layers`,
      keeping each command's existing position semantics. Verify with `rg` and
      the affected tests.
- [x] 4.2 Rewrite the duplicated `truncate_above` and `strip_adjustments`
      scratch helpers in `pictura-app` and `pictura-paint` as
      `truncate`/`retain_tree` calls. Verify the clone-sampling and
      filter-preview tests.
- [x] 4.3 Migrate `crates/pictura-codec` layer-tree construction
      (`build_tree`, `derive_background`, color-mode retention, smart-object
      lookup, ICC conversion) and the PDF export clear to the stack API.
- [x] 4.4 Migrate history snapshots and the remaining fixture assignments in
      `crates/pictura-app/src/history/` and the app tests. Verify snapshots use
      `clone`, not `from_layers`.
- [x] 4.5 Audit the raw hatches: list every `from_layers` call and confirm it is
      codec load, scratch construction, or a test fixture; audit whole-node
      `layers[...] =` / `children =` assignments and direct `.clipping =` /
      `.background =` writes, leaving only the codec, history restore, and flag
      commands. Record the allowlist in the task result.

## 5. Integration checks

- [x] 5.1 Run `cargo fmt --all --check` and
      `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 5.2 Run `cargo nextest run --workspace` and
      `cargo test --workspace --doc`.
- [x] 5.3 Run `openspec validate --all --strict` and
      `python3 scripts/check-test-plan.py`.
- [x] 5.4 Run `openspec archive layer-stack-invariants --yes`, then
      `bash scripts/check-openspec-archived.sh` and
      `openspec validate --all --strict`.
- [x] 5.5 Run `bash scripts/verify-fast.sh`, then configure and build with CMake
      and run `bash scripts/verify-full.sh` before the PR.
- [x] 5.6 Run `bash scripts/check-file-size.sh`; keep `tests_stack.rs` under the
      1,200-line code cap and the new codec test submodule under the 1,400-line
      test cap, or split them.
