# Tasks

## 1. Contract skeleton and tests

- [x] 1.1 Add the `Edit` and `Effect` types plus stubbed `PictureViewRust::commit`, `PictureView::edit`/`preview`/`commit_move_edit`, and `effect_of` in a new `cxxqt_object` module; verify the crate compiles with `cargo check -p pictura-app`.
- [x] 1.2 Add the transaction tests named in `test-plan.md` (`pictura-app::*`), covering the no-op, returned outcome, bounded/multi-rect/meta/record/structural/full presentation, capture-after-refresh, preview, and move-revision cases; verify they compile and fail against the stubs.
- [x] 1.3 Add `pictura-render::a_geometry_op_leaves_the_composite_untouched`, `pictura-render::a_merged_only_document_keeps_its_pixels_through_geometry`, and the merged-only save case named in the plan; verify they fail against the current `recompute` calls.
- [x] 1.4 Run `python3 scripts/check-test-plan.py`; verify every scenario in the change (including the three modified capabilities) has a row and only the expected not-yet-written references fail.

## 2. Transaction core

- [x] 2.1 Implement `commit` (capture one state, dirty, content revision, no compositing) and the `effect_of` mapping; verify group 1.2's no-op, outcome, meta, record, structural, and full tests pass.
- [x] 2.2 Implement `edit` as mutate → composite (`refresh_regions` or `composite_full`) → clear links for `Structural` → `commit` → notify; verify the capture-after-refresh and bounded/multi-rect tests pass and `Meta`/`Record` emit the intended signals.
- [x] 2.3 Split `composite_full` out of `recomposite` (no signal) and keep `recomposite` as `composite_full` + `changed` for undo/redo/open; verify the existing undo, open, and canvas-present tests still pass.
- [x] 2.4 Implement `preview` (no capture, revision bump) and `commit_move_edit` (capture without a revision bump) and verify the preview and move-revision tests pass.
- [x] 2.5 Keep the Region/Record signal policy: `Regions` and `Record` emit no `changed`; verify selftest 363 (`vis_toggle_region`), selftest `canvas_present_contract`, and `tst_layers_preview_coalescing::opacityPreviewUsesTheRegionPath` are unchanged.

## 3. Migrate helpers and every commit tail

- [x] 3.1 Rewrite `mutate_layer`, `mutate_meta`, `batch_changed`, and `batch_property` on the transaction; verify `cargo nextest run -p pictura-app` and `tst_layers_preview_coalescing` pass.
- [ ] 3.2 Migrate the remaining commit sites (done except the deliberate exceptions: shape add-layer, fills `apply`, the detached filter-job commit, and free transform’s bounded link clear): `set_layers_visible`, `commit_layers_opacity/fill`, `apply_visibility`, `delete_layers`, `apply_metadata_edits`/`apply_metadata_template`, `assign_profile`/`convert_profile`, `apply_selection_labeled`, `paths::commit`, `path_list::commit`, `guides::commit`, `annotations::commit`, `layer_style_commit`, `adjustment_commit`, image match/replace/HDR, and filter-job commits.
- [ ] 3.3 Classify the link-invalidating non-tree commands as `Structural` (Pictura Raw, type conversion/style, rasterize shape done; free transform keeps a manual link clear to preserve its bounded refresh); verify their link behavior is unchanged and `delete_layers` clears links inside the transaction rather than after `batch_changed`.
- [x] 3.4 Route paint and mask commits through `Edit::Regions` with the stroke's disjoint dirty rectangles; verify `a_multi_rect_stroke_commit_equals_a_full_recomposite` and the one-blit contract pass.
- [ ] 3.5 Add a grep-based audit/guard for `record(`, `record_move(`, `refresh_region(s)(`, `recomposite(`, and `clear_link_sets(` under `crates/pictura-app/src` with an explicit allowlist for undo/redo/open/cancel and preview internals; verify the guard passes and no command hand-writes the sequence.
- [x] 3.6 Verify the migrated helpers contain no manual refresh/capture/link sequence and that no-op and region rules come from `Edit`.

## 4. Engine composite side effect and merged-only documents

- [x] 4.1 Delete `document_ops::recompute` and its nine call sites (`resize`, `resize_canvas_document`, `extend_background`, `crop_document`, `delete_cropped_pixels`, `translate_layer`, `orient`, `rotate_in`, `perspective_crop`); update the ops' doc comments.
- [x] 4.2 Add `materialize_merged_composite` (colour planes plus alpha, bottom Background, only when the composite matches the document size) and call it at the start of the geometry operations; verify `pictura-render::a_merged_only_document_keeps_its_pixels_through_geometry` passes.
- [x] 4.3 Update `document_oracle.rs` (`assert_structural_oracle` and the `psd_tools_sees_*` tests) and the geometry unit tests to assign `doc.composite = composite_rgba(&doc)` before writing or asserting; verify `resize_document_preserves_structure_and_composite`, `resize_canvas_document_preserves_structure_and_composite`, `rotate_document_preserves_structure_and_composite`, and `flip_document_preserves_structure_and_composite` pass.
- [x] 4.4 Update the modified capability deltas if the implementation reveals a different materialization rule; verify `openspec validate edit-transaction --strict` passes.
- [x] 4.5 Verify `pictura-render::a_geometry_op_leaves_the_composite_untouched` passes, `translate_layer_active` still matches the CPU oracle, and `cargo nextest run -p pictura-render` is green.
- [x] 4.6 Verify a resize command composites once in the app (`tst_image_ops::imageSizeResamplesAndSetsResolution`, `tst_image_mode`, `tst_large_document`) and the merged-only Qt cases display and save.

## 5. Preview, record-only, and resolution commits

- [ ] 5.1 Convert `edit_adjustment` and `edit_target` to `preview`; convert `adjustment_commit` and `layer_style_commit` to `Edit::Record` (adjustment preview/commit and layer-style commit done; `edit_target` still refreshes through its own helper); verify adjustments, paths, and layer styles record once and emit no extra `changed`.
- [x] 5.2 Convert the resolution-only Image Size path to `Edit::Meta`; verify one state is recorded, no compositor runs, and the image-size tests pass.
- [x] 5.3 Verify `tst_layers_preview_coalescing`, the adjustment-editor tests, and the mask-refine tests pass and previews add no history states.

## 6. Integration and archive

- [x] 6.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, and `cargo test --workspace --doc`; verify all pass.
- [x] 6.2 Run `bash scripts/verify-fast.sh` and `ctest --test-dir build -R '^tst_' --output-on-failure`; verify the guards and Qt suites pass.
- [x] 6.3 Check file sizes against `scripts/file-size-allowlist.txt` after each migration phase; verify `impl_layers.rs` stays under its cap and no new allowlist entry is needed.
- [x] 6.4 Run `./build/pictura --headless --self-test`; verify `failed=0`, especially `vis_toggle_region`, `canvas_present_contract`, and the undo/redo checks.
- [x] 6.5 Run `python3 scripts/check-test-plan.py` before archiving; verify every referenced test exists and every scenario is covered.
- [x] 6.6 Archive with `openspec archive edit-transaction`, then verify `openspec validate --all --strict` and `python3 scripts/check-test-plan.py` pass.

## Status

The transaction core, engine recompute removal, merged-only materialization, helper/structural migrations, paint regions, and adjustment preview/record are implemented. Build and tests pass: workspace clippy, nextest, doctests, the C++ build, all 119 Qt suites, and both self-test invocations. Remaining open tasks are deliberate exceptions: four commit sites (shape add-layer, fills `apply`, the detached filter-job commit, free transform’s bounded link clear), free transform’s manual link clear, the grep audit guard, and `edit_target`’s own refresh helper.
