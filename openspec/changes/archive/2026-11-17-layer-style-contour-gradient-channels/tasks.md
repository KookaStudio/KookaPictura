# Tasks

Authoring only: the single build and all verification run in section 7. Do not
run build, list, or test commands while authoring. Each task names the check it
must ultimately satisfy.

## 1. Tests first and scaffolding

- [x] 1.1 Add the contour and authoring test functions named in `test-plan.md` (`bevel_contours_decode`, `bevel_contours_default_to_linear`, `bevel_contour_malformed_is_linear`, `gloss_contour_remaps_the_shading`, `edge_contour_remaps_the_profile`, `edge_contour_off_is_a_no_op`, `contour_range_scales_the_profile`, `linear_contours_are_byte_identical`, `contour_text_round_trips`, `absent_contour_reads_linear`, `bad_contour_text_is_refused`, `contour_edit_preserves_point_items`, `bevel_sub_panel_fields_round_trip`, `bevel_gloss_antialias_defaults_on`, `gradient_text_round_trips`, `gradient_edit_preserves_opacity_stops`, `bad_gradient_text_is_refused`, `authored_contour_and_gradient_survive_save`, `channel_include_toggles_the_restriction`, `channels_are_included_by_default`, `group_accepts_channel_include`, `default_gradient_text_creates_an_absent_effect`, `malformed_stored_contour_and_gradient_are_replaced`) under `crates/pictura-render/src/tests/layer_effects/bevel.rs` and `crates/pictura-render/src/document_ops/layer_ops/layer_style_tests.rs`; verify them in section 7.
- [x] 1.2 Add the codec functions named in `test-plan.md` (`brst_decodes_channel_restrictions`, `absent_brst_includes_every_channel`, `malformed_brst_is_dropped`, `channel_restrictions_round_trip`) under `crates/pictura-codec/src/tests/advanced_blending.rs`; verify them in section 7.
- [x] 1.3 Add the compositor and GPU functions named in `test-plan.md` (`excluded_channel_keeps_the_backdrop`, `empty_channel_restrictions_are_a_no_op`, `excluded_channel_survives_effect_passes`, `excluded_channel_survives_a_pass_through_group`, `excluded_channel_keeps_a_translucent_backdrop`, `excluded_channel_survives_a_destructive_adjustment`, `banded_channel_exclusion_matches_whole`, `gpu_declines_a_channel_excluded_layer`) under `crates/pictura-render/src/tests/`; verify them in section 7.
- [x] 1.4 Add the Qt cases in `tst_layer_style` (`contourPresetChangesTheBevelAndCancelDropsIt`, `glossContourWritesTheEffect`, `contourCheckboxGatesTheSubControls`, `gradientEditorWritesTheOverlayStops`, `gradientEditorCancelKeepsTheGradient`, `gradientEditorKeepsLocationsStrict`, `gradientEditorHidesNoiseAndSmoothness`, `channelIncludeChangesTheCompositeAndCancelRestores`, `channelRowFollowsTheDocumentMode`, `gradientEditorPresetStaysOpaque`, `copyEnabledForChannelOnlyStyle`); verify the suite in section 7.
- [x] 1.5 Scaffold `crates/pictura-render/src/layer_effects/contour.rs`, `crates/pictura-render/src/document_ops/layer_ops/layer_style_contour.rs`, `crates/pictura-render/src/document_ops/layer_ops/layer_style_commands.rs` and `crates/pictura-app/cpp/layer_style_contour.{h,cpp}`, and register the C++ pair in `cmake/sources/dialogs.cmake`; verify `python3 scripts/check-cmake-source-fragments.py`.

## 2. Contour model and bevel rendering

- [x] 2.1 Implement `Contour` in `layer_effects/contour.rs`: point parsing from `ShpC`/`Crv`/`CrPt`, the linear default, the 256-step piecewise-linear LUT, `is_linear` defined on the LUT (any identity point list counts), and descriptor writing that updates only `Hrzn`/`Vrtc` and preserves other `CrPt` items; verify `pictura-render::bevel_contours_decode`, `bevel_contours_default_to_linear` and `bevel_contour_malformed_is_linear`.
- [x] 2.2 Extend `BevelEmboss` and `decode_bevel_emboss` with `gloss_contour`, `edge_contour`, `edge_contour_on`, `edge_range` (clamped 1–100) and `edge_anti_alias`, defaulting to the spec; add `bevel.edgeContourOn` (`useShape`), `bevel.edgeRange` (`Inpr`) and `bevel.edgeAntiAlias` (`AntA`) to the `BEVEL` field table and set `bevel.antiAlias` (`antialiasGloss`) to default 1; verify `pictura-render::bevel_sub_panel_fields_round_trip` and `bevel_gloss_antialias_defaults_on`.
- [x] 2.3 Apply the gloss LUT to the clamped signed shading and the edge LUT to the clamped rounded chamfer in `build_bevel_emboss`; a linear/identity contour and `useShape` off take the exact no-op path that skips both the LUT and the range scaling; verify `pictura-render::gloss_contour_remaps_the_shading`, `edge_contour_remaps_the_profile`, `edge_contour_off_is_a_no_op`, `contour_range_scales_the_profile` and `linear_contours_are_byte_identical`.

## 3. Structured authoring

- [x] 3.1 Move contour authoring into `layer_style_contour.rs` and implement `layer_style_contour` / `set_layer_style_contour` with the `"x,y …"` format, `Nm` and `CrPt` item preservation, group/Background refusal, and an absent bevel added off at its defaults; verify `pictura-render::contour_text_round_trips`, `absent_contour_reads_linear`, `bad_contour_text_is_refused` and `contour_edit_preserves_point_items`.
- [x] 3.2 Implement `layer_style_gradient` / `set_layer_style_gradient` with the six-digit `"loc:rrggbb …"` format, rejecting alpha; replace only `Clrs`, preserving `Trns`/`GrdF`/`Intr` and `Mdpn` for unchanged locations; refuse groups and the Background; verify `pictura-render::gradient_text_round_trips`, `gradient_edit_preserves_opacity_stops`, `bad_gradient_text_is_refused` and `authored_contour_and_gradient_survive_save`.
- [x] 3.3 Add `blending.include0..2` to `blending_value` and `set_blending_value`, mapping onto the layer's channel restrictions, accepting groups and refusing the Background; verify `pictura-render::channel_include_toggles_the_restriction`, `channels_are_included_by_default` and `group_accepts_channel_include`.
- [x] 3.4 Add the bridge functions `layer_style_contour`, `layer_style_set_contour`, `layer_style_gradient`, `layer_style_set_gradient` and `layer_style_channel_names`, sharing the before/after paint-bounds refresh with the scalar setter; verify the existing `tst_layer_style::dialogOpensOnItsEffectAndCancelDropsTheEdits` still passes in section 7.
- [x] 3.5 Carry `channel_exclude` in `LayerStyle` and its copy/paste path; verify `pictura-render::channel_exclude_survives_copy_and_paste`.

## 4. Channel restrictions: codec, compositor, GPU

- [x] 4.1 Add `Layer.channel_exclude`, parse `brst` into it on read (removing it from `extra_blocks`), drop a malformed payload instead of preserving it, and emit it on write when non-empty; verify `pictura-codec::brst_decodes_channel_restrictions`, `absent_brst_includes_every_channel`, `malformed_brst_is_dropped` and `channel_restrictions_round_trip`.
- [x] 4.2 Apply the excluded-channel mask at the layer boundary in `composite_layer_inner`: snapshot the excluded planes over the layer's paint reach (or the whole canvas for a group or destructive adjustment) before below-content effects and restore them after above-content effects, around a pass-through group's children, and in `composite_knockout` on the running canvas after copying covered pixels; the restored value is the entry backdrop's straight channel at the normal output alpha. Bypass the fast row kernels and the banded skip path when the mask is non-zero. Verify `pictura-render::excluded_channel_keeps_the_backdrop`, `empty_channel_restrictions_are_a_no_op`, `excluded_channel_survives_effect_passes`, `excluded_channel_survives_a_pass_through_group`, `excluded_channel_keeps_a_translucent_backdrop`, `excluded_channel_survives_a_destructive_adjustment` and `banded_channel_exclusion_matches_whole`.
- [x] 4.3 Add a non-empty channel restriction list to `own_attribute_decline` so the GPU reports `UnsupportedAdvancedBlending`; verify `pictura-render::gpu_declines_a_channel_excluded_layer`.

## 5. Contour UI

- [x] 5.1 Add `CurveWidget::setLinearInterpolation(bool)` (default off) so the editor's LUT matches the engine without changing Curves; verify the existing `tst_shared_widgets`, `tst_design_system`, `tst_image_adjustments`, `tst_acr_dialog` and Curves suites still pass.
- [x] 5.2 Implement `ContourSwatch` and `ContourEditorDialog` in `layer_style_contour.{h,cpp}` with the preset list, Reset, OK/Cancel and the swatch curve, applying on accept; verify the pair builds and the swatch shows the current curve in the new Qt cases.
- [x] 5.3 Add the bevel page Gloss Contour row and Contour group (checkbox, swatch, Range, Anti-aliased) with the sub-controls gated by the checkbox, and update the dialog header's ceiling note; verify `tst_layer_style::contourPresetChangesTheBevelAndCancelDropsIt`, `glossContourWritesTheEffect` and `contourCheckboxGatesTheSubControls`.

## 6. Gradient and Channels UI

- [x] 6.1 Add a solid-only mode to `GradientEditorDialog` (Noise and Smoothness unavailable) and a call-site guard that keeps stop locations strictly rising and forces accepted stops opaque; replace the Gradient Overlay page's two swatches with one `GradientSwatch` opening the editor, writing on accept only and updating the swatch only when the write succeeds; keep Reverse/Style/Align/Angle/Scale/Dither unchanged; verify `tst_layer_style::gradientEditorWritesTheOverlayStops`, `gradientEditorCancelKeepsTheGradient`, `gradientEditorKeepsLocationsStrict`, `gradientEditorHidesNoiseAndSmoothness` and `gradientEditorPresetStaysOpaque`.
- [x] 6.2 Add the Channels Include row to Blending Options from `layer_style_channel_names`, shown only for RGB and binding `blending.includeN`; verify `tst_layer_style::channelIncludeChangesTheCompositeAndCancelRestores` and `channelRowFollowsTheDocumentMode`.

## 7. Integration and close-out

- [x] 7.1 Build the app (`cmake -S . -B build -G Ninja` if needed, then `cmake --build build --parallel`) and fix compile errors; verify the build succeeds.
- [ ] 7.2 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` and `bash scripts/verify-fast.sh`; verify every gate passes, including `python3 scripts/check-test-plan.py`.
- [x] 7.3 Update `ROADMAP.md` so the #581 line records contour, gradient editor and channel include as shipped and the texture half points at #606; verify the line.
- [x] 7.4 Update the LAY-3 row in `docs/dev/doc-code-parity-gaps.md` to PARTIAL (texture remains #606) in a separate commit marked `TASK-ALLOWS-DOCS`; if the register update is deferred, record that deferral in the PR body.
- [x] 7.5 Archive the change with `openspec archive layer-style-contour-gradient-channels` and verify `bash scripts/check-openspec-archived.sh`.
