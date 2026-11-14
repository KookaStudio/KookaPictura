# Tasks

## 1. Tests first

- [x] 1.1 Add `crates/pictura-select/tests/aa.rs` with `ellipse_aa_has_fractional_edge`, `ellipse_aa_off_is_binary`, `polygon_aa_has_fractional_edge`, `polygon_aa_off_is_binary`, `aa_contour_follows_the_primitive`, `aa_clips_at_the_canvas_edge`, `aa_applied_before_feather`, and `aa_leaves_existing_selection_unchanged`; verify each name exists in the file and the plan maps them.
- [x] 1.2 Add `wand_aa_grades_boundary`, `wand_aa_off_is_binary`, `wand_aa_zero_tolerance_is_binary`, `wand_aa_global_fringes_each_region`, and `wand_aa_low_tolerance_fringes` unit tests to `crates/pictura-select/src/wand.rs`; verify each name exists.
- [x] 1.3 Add `channel_budget_counts_colour_and_extra_channels`, `channel_budget_refuses_at_56`, `channel_budget_multichannel_uses_composite`, and `channel_budget_uses_the_source_mode` to the new core channel-budget module; verify the names exist.
- [x] 1.4 Add `extra_channels_write_at_the_56_channel_limit` to a new `crates/pictura-codec/tests/channel_budget.rs` (not `crates/pictura-codec/src/tests.rs`, which is at its 1429-line allowlist ceiling); verify the name exists.
- [x] 1.5 Add `new_alpha_refuses_at_cap` to `crates/pictura-app/src/cxxqt_object/channels/imp.rs` and `save_selection_refuses_at_cap` to a new `crates/pictura-app/src/cxxqt_object/impl_selection/cap_tests.rs`; verify both names exist.
- [x] 1.6 Add `tst_selection_aa.cpp` with `optionsEnableAntiAlias`, `antiAliasedEllipseCommitsPartialCoverage`, `lassoAntiAliasCommitsPartialCoverage`, `polygonalAntiAliasFollowsThePolygon`, `magneticAntiAliasCommitsPartialCoverage`, `wandAntiAliasFringe`, `wandAntiAliasGlobalFringe`, and `saveSelectionRefusesAtCap`; extend `wandAntiAliasFringe` to assert Add-mode union and exactly one history state; register it in `crates/pictura-app/cpp/tests/CMakeLists.txt` and verify the slots exist.
- [x] 1.7 Add `newChannelDisabledAtCap` to `tst_channels_panel.cpp` and `antiAliasRoundTrips` / `wandAntiAliasIndependent` to `tst_tool_options_persistence.cpp`; verify the slots exist.
- [x] 1.8 Update `optionRows()` in `crates/pictura-app/cpp/tests/tst_tool_options_persistence.cpp`: add `antiAlias` to the `selection`, `ellipticalSelection`, and `quickSelection` rows, add a `wandSelection` row, and verify `roundTripTableIsComplete` passes.

## 2. Channel budget

- [x] 2.1 Add `crates/pictura-core/src/channels.rs` with `MAX_CHANNELS = 56` and `Document::can_add_channel()`, counting `source_mode.unwrap_or(mode)` colour channels and the composite count for Multichannel; declare it in `pictura-core/src/lib.rs`, and verify the core budget tests, including `channel_budget_uses_the_source_mode`, pass.
- [x] 2.2 Replace the private codec constant in `pictura-codec/src/common.rs` with the core constant and verify `extra_channels_write_at_the_56_channel_limit` in the new codec integration test passes.

## 3. Anti-aliased primitives

- [x] 3.1 Add `Selection::ellipse_aa` with analytic boundary supersampling in `crates/pictura-select/src/lib.rs` and verify the ellipse AA tests, `aa_applied_before_feather`, `aa_leaves_existing_selection_unchanged`, and `aa_clips_at_the_canvas_edge` pass.
- [x] 3.2 Add `Selection::polygon_aa` / `polygon_bounded_aa` using `pictura_core::shape::coverage_box` and verify the polygon AA, 50 %-contour, order, and existing-selection tests pass.
- [x] 3.3 Add the Magic Wand boundary ramp to `crates/pictura-select/src/wand.rs` using `width = max(0.5 * tolerance, 1.5)` and `coverage = clamp(1 - (d - tolerance) / width, 0, 1)`, and verify the five wand AA tests pass.
- [x] 3.4 Update every `magic_wand` caller and the dense oracle for the six-argument signature (`crates/pictura-app/src/cxxqt_object/impl_selection.rs`, `crates/pictura-select/src/wand.rs`, `crates/pictura-select/src/tests.rs`, `crates/pictura-select/src/tiled/tests.rs`, and the `pictura-app` tests); verify the workspace compiles and its tests pass.

## 4. App bridge and cap enforcement

- [x] 4.1 Add the three free anti-alias commit functions to the existing `crates/pictura-app/src/cxxqt_object/impl_selection/bridge.rs` and implement them in `impl_selection/bridge/imp.rs`; do not add a new bridge file, `mod`, `build.rs` entry, or bridge C++ header. Verify the crate compiles and the bridge functions are generated.
- [x] 4.2 Enforce `can_add_channel` in `channels_new_alpha` and `save_selection_channel`, add the `channels_can_add` bridge query, and verify `new_alpha_refuses_at_cap` and `save_selection_refuses_at_cap` pass.

## 5. C++ shell

- [x] 5.1 Add `SelectionOptions::setAntiAlias`; enable and bind the Anti-alias checkboxes on the ellipse/lasso and wand options pages with `setChecked(group->antiAlias())`, connect them to `setAntiAlias`, add object names, replace the old "Not modelled / binary" tooltips with the new behaviour, and verify `tst_selection_aa::optionsEnableAntiAlias` passes.
- [x] 5.2 Route the ellipse, lasso, polygonal, magnetic, and wand commits through the new anti-alias functions, including the existing `api/selection_bridge.h` in the tool sources that call them, and verify the commit cases plus the extended `wandAntiAliasFringe` Add-mode/undo case in `tst_selection_aa` pass.
- [x] 5.3 Add `antiAlias` to the selection options persistence fields, call `setAntiAlias` from `applySelection`, copy `antiAlias_` in `SelectionOptions::apply`, add the independent `wandSelection` group, and verify `antiAliasRoundTrips`, `wandAntiAliasIndependent`, and `roundTripTableIsComplete` pass.
- [x] 5.4 Disable New Channel at the cap and show the non-modal status-bar Save Selection refusal in the panel and the Select menu; verify `newChannelDisabledAtCap` and `saveSelectionRefusesAtCap` pass.
- [x] 5.5 Update `crates/pictura-app/cpp/selftest_tools_selection.cpp:206,223-224` from `ToolId::Marquee` to `ToolId::MagicWand` for the wand options, run the app self-test, and keep the file under its 1200-line cap.

## 6. Integration and records

- [ ] 6.1 Build the app and run the targeted Qt suites `tst_selection_aa`, `tst_channels_panel`, and `tst_tool_options_persistence` offscreen, plus the app self-test; verify all pass.
- [ ] 6.2 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo nextest run --workspace`; verify all pass.
- [ ] 6.3 Run `python3 scripts/check-test-plan.py` and `bash scripts/verify-fast.sh`; verify both pass.
- [ ] 6.4 Update `ROADMAP.md`, `docs/dev/doc-code-parity-gaps.md` (SEL-6), and `docs/dev/STATE.md`; archive the change with `openspec archive aa-selection-raster` and verify `openspec validate --all --strict` passes.
