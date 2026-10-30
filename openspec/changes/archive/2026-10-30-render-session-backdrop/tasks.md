# Tasks

## 1. Measurement gate

- [x] 1.1 Add two ignored profiles to `crates/pictura-app/src/cxxqt_object/tests_profiles.rs`: a Curves drag and a canvas-sized opacity drag, each at fit and at 100% zoom, printing ms per tick, composites per input event, and the composite/store/pyramid split. Verify both run with `cargo test -p pictura-app -- <name> --ignored --nocapture` and print the numbers.
- [x] 1.2 Record the baseline numbers and the backdrop budget chosen from them in `design.md` under Open Questions. Verify the recorded numbers match the profile output.

## 2. Render-session core (`pictura-render`)

- [x] 2.1 Add a `render_session` module skeleton with the public API (prefix digest, backdrop slot, preview and refine entry points, and the compositor entry point that composites `doc.layers[k..]` onto a caller-supplied canvas) so the tests compile. Then write `render_session::prefix_digest_changes_on_a_pixel_edit`, `prefix_digest_changes_on_a_property_edit`, `prefix_digest_changes_on_a_reorder`, `prefix_digest_changes_on_a_pattern_edit`, and `prefix_digest_ignores_edits_at_or_above`; verify the assertions fail once it compiles.
- [x] 2.2 Write `render_session::backdrop_plus_suffix_equals_full_composite`, `backdrop_matches_document_oracle`, `backdrop_matches_clipping_oracle`, `backdrop_matches_knockout_oracle`, `second_frame_reuses_the_backdrop`, `level0_preview_equals_the_full_region`, `reduced_preview_need_not_equal_the_exact_level`, `reduced_document_rebuilds_when_the_document_changes_outside_the_preview`, `unreducible_content_falls_back_to_full`, `empty_rect_is_a_noop`, and `no_presented_level_falls_back_to_exact`; verify the assertions fail.
- [x] 2.3 Write `render_session::clipped_target_falls_back_to_full`, `nested_target_falls_back_to_full`, `knockout_above_the_resume_point_is_exact`, `root_level_target_resumes_and_matches_full`, and `over_budget_backdrop_is_refused`; verify the assertions fail.
- [x] 2.4 Implement the module body: the prefix digest (including the document pattern library's identity), the backdrop slot, the preview and refine entry points, and the `doc.layers[k..]` compositor entry point; verify 2.1-2.3 pass.
- [x] 2.5 Export the module from `crates/pictura-render/src/lib.rs`; verify `cargo nextest run -p pictura-render render_session` passes.
- [x] 2.6 Set the session backdrop budget constant from 1.2 and verify `render_session::over_budget_backdrop_is_refused` uses it.

## 3. Preview document and display plumbing

- [x] 3.1 Add a `display_preview` module skeleton with the display-preview state and API so the tests compile. Then write `display_preview::preview_patches_only_the_presented_level`, `preview_leaves_the_composite_unchanged`, `commit_produces_the_exact_frame`, `display_preview_commit_follows_the_bound`, `unchanged_preview_records_nothing`, `noop_round_trip_records_nothing`, `cancel_restores_the_pre_gesture_display`, `no_stored_level_falls_back_to_exact`, and `solo_mask_and_live_stroke_fall_back`; verify the assertions fail once it compiles.
- [x] 3.2 With the display-preview state skeleton in place, write `picture::a_display_preview_is_composited`; verify the assertion fails.
- [x] 3.3 Add the Qt Test `crates/pictura-app/cpp/tests/tst_properties_preview.cpp` with `adjustmentDragCoalescesPreview`, `commitUsesTheLatestValue`, and `commitFlushesThePendingPreview`, register it in `crates/pictura-app/cpp/tests/CMakeLists.txt`, and verify it fails.
- [x] 3.4 Fix `FilterProxy::reduce_layer` so a channel-less fill layer keeps its rect mapped to the proxy grid, and add a preview-document handle that sets the edited layer's parameters and composites it; verify the existing `filter_proxy` tests plus a new fill-layer test pass.
- [x] 3.5 Add the bridge method that reports the presented level and the visible document rectangle from `ImageView`. The preview patches the level the canvas presents, so the canvas reads it without forcing a level. Verify the bridge reports both at fit and at 100%.
- [x] 3.6 Add the session and the display-preview state to `PictureViewRust` and implement the preview presentation: level 0 writes the sRGB rectangle into `PictureViewRust::level0`; levels above 0 convert the sRGB rectangle to premultiplied interleaved RGBA and use `pyramid.patch_level`; bump the canvas revision and emit display damage only, not the document damage account or history. Verify 3.1 passes.
- [x] 3.7 Route `adjustment_set`, `adjustment_set_curve`, `adjustment_reset`, and the canvas-sized `preview_layers_opacity`, `preview_layers_fill`, blend, and visibility previews through the display preview; verify 3.1 passes.
- [x] 3.8 Coalesce the Properties panel with a single-shot timer scheduled for the next event-loop turn (0 ms), mirroring the Layers panel's 120 ms pattern; verify 3.3 passes.
- [x] 3.9 Flush the pending preview in `PropertiesPanel::commit()` (the `gestureEnded` handler) before `adjustment_commit`; verify `tst_properties_preview::commitFlushesThePendingPreview` passes.
- [x] 3.10 Rebuild the reduced document when the document changes outside the preview's own revision bump (undo/redo, edits above the target); verify `render_session::reduced_document_rebuilds_when_the_document_changes_outside_the_preview` passes.
- [x] 3.11 Implement the cancel path: discard the pending value, restore the pre-gesture display (reset the pyramid or repatch the captured frame), and record nothing; verify `display_preview::cancel_restores_the_pre_gesture_display` passes.

## 4. Commit, refine, and current-picture

- [x] 4.1 Change `adjustment_commit` and the canvas-sized property commits to run the exact composite before recording; verify `display_preview::commit_produces_the_exact_frame` passes.
- [x] 4.2 Capture the pre-gesture value for adjustments and opacity/fill, compare the final value at commit, and return `Edit::None` when they are equal; verify `display_preview::unchanged_preview_records_nothing`, `display_preview::noop_round_trip_records_nothing`, and the `rs_commit_noop` row pass.
- [x] 4.3 Clear the display-preview state and reset the pyramid after the exact composite; verify every level equals a rebuild from the exact level 0.
- [x] 4.4 Add the display-preview guard to `picture::is_current`; verify `picture::a_display_preview_is_composited` passes.
- [x] 4.5 Verify the cancel path (Esc, layer switch, document replacement) records nothing and restores the pre-gesture display.

## 5. Integration and documentation

- [x] 5.1 Run `cargo nextest run -p pictura-render -p pictura-app` and `ctest --test-dir build -R '^tst_properties_preview$' --output-on-failure`; verify all pass.
- [x] 5.2 Run `bash scripts/verify-fast.sh`; verify fmt, clippy, the test report, the file-size check, the guard, and OpenSpec validation pass.
- [x] 5.3 Re-run the profiles from 1.1 on the branch build and record the before/after in the PR description; verify the numbers come from the branch build.
- [x] 5.4 Update `docs/dev/STATE.md` with the change and its measured win; verify `scripts/guard.sh` accepts the commit with `TASK-ALLOWS-DOCS`.
