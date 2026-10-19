# Tasks

Repository rules apply: no explanatory comments, no new dependencies, and the
file-size caps (`scripts/check-file-size.sh`).

## 1. Tests first

- [x] 1.1 Add `opacityDragCoalescesPreview`, `fillDragCoalescesPreview`, `midDragPausePreviewsWhileHeld`, `fastOpacityRoundTripRecordsNothing`, and `fillFastRoundTripRecordsNothing` to `crates/pictura-app/cpp/tests/tst_layers_preview_coalescing.cpp` (registered in `crates/pictura-app/cpp/tests/CMakeLists.txt`). Drive the slider inline (`findChild<QSlider*>`, `setSliderDown`, `setValue`, `qWait`, `sliderReleased`); do not add seams to `crates/pictura-app/cpp/panels/layers_panel_test.cpp` (1393/1400 lines). Verify `python3 scripts/check-test-plan.py` resolves every new slot.
- [x] 1.2 Add `pendingPreviewSurvivesPanelRefresh`, `documentSwitchCancelsPendingPreview`, `selectionChangeCancelsPendingPreview`, `sameViewReplacementCancelsPendingPreview`, and `fastRoundTripAtNonFixedByteRecordsOneState` to the same file; verify `python3 scripts/check-test-plan.py` resolves them.
- [x] 1.3 Add `noSelectionShowsDefaultPercent`, `ineligibleTargetsShowDefaultPercent`, and `multiSelectionKeepsEligibleFieldEnabled`; test the Background, a fully locked layer, a group, a synthetic Smart Filters row, an all-ineligible multi-selection, and Fill on a mixed multi-selection; verify the script resolves them.
- [x] 1.4 Add `opacityPreviewUsesTheRegionPath`, `opacityCommitUsesTheRegionPath`, `canvasSizedOpacityUsesTheRegionPath`, and `groupOpacityPreviewFallsBackToFullRecomposite`. Use `place_image` for a genuinely bounded layer (as in `tst_layers_snapshot.cpp`) and assert the emitted region rect and pixels outside it. Assert the commit leaves canvas damage (a full rebuild clears it), proving no full rebuild. Add the Rust helper test `changed_property_region_bounds_a_pixel_layer_and_rejects_a_group` in `impl_layers.rs`; verify the script resolves all five.
- [x] 1.5 Add the ignored profile `opacity_preview_profile_4000` in `crates/pictura-app/src/cxxqt_object/tests_profiles.rs` with a reason string naming the `--ignored --nocapture` command; verify `python3 scripts/check-test-plan.py` resolves it.

## 2. Panel preview coalescing

- [x] 2.1 In `layers_panel.{h,cpp}`, add the pending `{paths, field, value}` state and the single-shot 120 ms timer; start it only when inactive, update the pending value on every tick, and apply the latest value on timeout. Verify `tst_layers_preview_coalescing::opacityDragCoalescesPreview`, `fillDragCoalescesPreview`, and `midDragPausePreviewsWhileHeld` pass.
- [x] 2.2 In `syncControls()`, skip `setValue()` for the field with a pending preview; add `NumericField::cancelPending()` and cancel the pending preview and the field's pending commit on a real view change, a selected-path change, or a loss of eligibility. Make `valueCommitted` use the captured gesture paths. Verify `pendingPreviewSurvivesPanelRefresh`, `documentSwitchCancelsPendingPreview`, and `selectionChangeCancelsPendingPreview` pass, and that a release after cancellation adds no state.
- [x] 2.3 In `syncControls()`, compute `opacityAny`/`fillAny` before setting the field values and show `100` for every field without an eligible target; verify `noSelectionShowsDefaultPercent`, `ineligibleTargetsShowDefaultPercent`, and `multiSelectionKeepsEligibleFieldEnabled` pass.

## 3. Region-scoped preview and commit

- [x] 3.1 Extract the before/after compositing check and `layer_visibility_region` union from `batch_property` into one private helper in `impl_layers.rs`; call it from `batch_property`; verify `cargo test -p pictura-app changed_property_region_bounds_a_pixel_layer_and_rejects_a_group` and the existing bounded-property tests pass.
- [x] 3.2 Use the helper in `preview_layers_opacity` and `preview_layers_fill`, keeping the sticky preview flag and `content_revision` bump; verify `tst_layers_preview_coalescing::opacityPreviewUsesTheRegionPath` and `groupOpacityPreviewFallsBackToFullRecomposite` pass.
- [x] 3.3 Use the helper in `commit_layers_opacity` and `commit_layers_fill`: when `changed > 0`, region-refresh or full-fallback and `record`; when `changed == 0 && pending`, only `record` and emit `changed`; when neither, no-op. Verify `tst_layers_preview_coalescing::opacityCommitUsesTheRegionPath`, `canvasSizedOpacityUsesTheRegionPath`, and `cargo test -p pictura-app layer_opacity_region_refresh_equals_a_full_recomposite` pass.

## 4. Integration, measurement, archive

- [x] 4.1 Run `cargo test -p pictura-app opacity_preview_profile_4000 -- --ignored --nocapture`; record the full and region per-step cost.
- [x] 4.2 Run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`; verify both finish clean.
- [x] 4.3 Run `cargo nextest run --workspace` and `cargo test --workspace --doc`; verify no failures.
- [x] 4.4 Build with `cmake --build build --parallel` and run `ctest --test-dir build -R '^(tst_layers_panel_fixes|tst_layers_preview_coalescing)$' --output-on-failure`; verify the new cases and the existing suite pass.
- [x] 4.5 Run `bash scripts/verify-fast.sh`; verify file-size, guard, test-plan, and OpenSpec checks pass.
- [x] 4.6 Run `openspec validate --all --strict`, then `openspec archive layer-property-preview-coalescing`; verify `scripts/check-openspec-archived.sh` passes.
- [x] 4.7 Claim issue #448, commit as `perf(layers): coalesce opacity and fill previews (#448)`, and open the PR as `perf: coalesce opacity and fill previews (#448)` with the test plan's manual rows listed as the reviewer checklist.

## 5. Review fixes

- [x] 5.1 Keep a persistent gesture record with stable name paths and the document generation, independent of the timer slot; use it for target-change checks and commit paths, and make the commit a no-op when the captured target no longer resolves.
- [x] 5.2 Reconcile the restored selection at the end of `refresh()`: an empty selection is transient, a non-empty selection whose layers differ cancels, and a same-view document replacement bumps the generation so it cancels.
- [x] 5.3 Strengthen the cancellation tests with non-fixed stored values and a preview fired before the target change; assert `valueCommitted`, both targets, dirty state, and history unchanged. Add same-view replacement, continuous-tick, synthetic-row, all-ineligible, and Fill multi-selection coverage.
- [x] 5.4 Guard `NumericField::cancelPending()` against a continued drag re-arming, clear a pending commit on `setValue`, mark the document dirty on a preview, and assert the commit uses no full rebuild via the canvas damage account.
