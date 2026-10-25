# Tasks

Each group is one unit and one commit (`refactor(<scope>): … (#N)`). Within a
group, the test tasks come first, and each C++ table is deleted in the same group
that adds its bridge. Under AGENTS.md step 5, every "verify with" below runs once
after the single build in group 6, not per task.

## 1. Engine blend-mode list (#271)

- [x] 1.1 Write `pictura-app::blend_mode_rows_follow_layer_modes` and a new
      `tst_blend_mode_list` suite registered in `cpp/tests/CMakeLists.txt`,
      with `layerCombosFollowEngineList`, `layerStyleComboIsEngineIndexed` and
      `comboIndexSetsEngineKey`.
- [x] 1.2 Add a `blend_modes.rs` bridge returning
      `Vec<BlendModeRow{key, name, has_neutral}>` from `BlendMode::LAYER_MODES`
      and `pictura_render::neutral_color`. Register it in `build.rs` and
      `cxxqt_object.rs`, and update `BlendMode::name()`'s doc comment.
- [x] 1.3 Replace `kBlends` in `blend_modes.h` with a cached `pictura::blendModes()`
      accessor in a new `blend_modes.cpp`, listed in `CMakeLists.txt`. Switch
      the New Layer dialog (including its `modesWithoutNeutral()` set), Layer
      Style, the Layers panel combo, the Mode filter and the Properties panel
      label to it. Verify with the 1.1 tests, `tst_layers_panel*` and
      `tst_layer_style`, unchanged.

## 2. Colour-model conversions (#269)

- [x] 2.1 Write the seven `pictura-color` tests from the plan,
      `pictura-app::channel_plane_matches_selection_values`,
      `tst_info_panel::labMatchesColorPicker`,
      `tst_info_panel::cmykMatchesColorPicker`,
      `tst_info_panel::cmykReadoutFollowsProfile` and
      `tst_color_picker::labEditUpdatesRgb`.
- [x] 2.2 Add `pictura_color::model`:
      - the scalar f64 `srgb_to_lab`/`lab_to_srgb` core and helpers, moved from
        `pictura-codec/src/color_mode.rs`. The codec's planar functions and
        re-exports keep their signatures and call the core.
      - `CmykConverter`: f64 transforms over `Profile::coated_cmyk()`, relative
        colorimetric, `NO_CACHE | BLACKPOINT_COMPENSATION`, held in a `OnceLock`.

      Verify with the `pictura-color` tests and the existing
      `pictura-codec`/`pictura-render` colour-mode tests, unchanged.
- [x] 2.3 Add the `pictura-color` path dependency to `pictura-app`. Add a
      `color_models.rs` bridge with `srgb_to_lab`, `lab_to_srgb`,
      `srgb_to_cmyk`, and `channel_plane` built on `impl_selection.rs`'s
      `channel_value`.
- [x] 2.4 Delete these C++ copies:
      - `rgbToLab`/`labToRgb` and the D50 constants from `color_picker_dialog.*`;
      - `QColor::toCmyk` from the picker and the Info panel;
      - `srgbToLab`/`struct Lab` from `info_panel.cpp`;
      - `toCmyk`/`channelValue` from `channels_panel.cpp`, which now calls
        `channel_plane`.

      Then point `selftest_healing.cpp` at the bridge, keeping exit 536 and
      growing no lines, and reword the picker's CMYK tooltip. Verify with the
      2.1 tests, `tst_info_panel`, `tst_color_picker` and `tst_channels_panel`.
      Record every expectation that changed because of D65 to D50 or naive CMYK
      to profile CMYK.
- [x] 2.5 Narrow `docs/dev/doc-code-parity-gaps.md` row CLR-7 (#604) to the gamut
      triangle and 16/32-bit fields, in a separate `TASK-ALLOWS-DOCS` commit.

## 3. Adjustment catalogue (#272)

- [x] 3.1 Write these tests:
      - `pictura-render::adjustment_catalogue_covers_every_kind_once`;
      - `pictura-render::adjustment_catalogue_layer_entries`;
      - `pictura-app::adjustment_catalogue_rows_resolve_commands`;
      - `tst_command_tree::imageAdjustmentsMenuFollowsCatalogue` (one declaration
        line in the .cpp, body in `tst_command_tree_cases.h`);
      - `tst_adjustments_panel::layersPanelAdjustmentMenuFollowsCatalogue`;
      - `tst_adjustments_panel::tilesAndMenuFollowCatalogue`;
      - `tst_image_adjustments::genericDialogRangesMatchLayout`;
      - `tst_image_adjustments::handLaidDialogRangesMatchLayout`.
- [x] 3.2 Add `pictura-render/src/adjustment_catalogue.rs` (25 rows in CS6 order)
      and delete `ADJUSTMENT_DIALOG_KINDS`. Derive `helpers.rs`'s adjustment
      layer names and test list from the catalogue. Pin `AdjustmentEditor.title`
      to the catalogue title with a test, since the layout decodes by
      descriptor, not by kind. Add an `adjustment_catalogue.rs` bridge.
- [x] 3.3 Delete `kDialogs` and `kDirect` from `frame_menus_adjust.cpp`, delete
      `layer_adjustments.h`, `AdjustmentsPanel::kEntries` and the
      `adjustmentsPanel` rows in `panel_group_menu.cpp`. Wire the Image commands,
      New Adjustment/Fill Layer, the Layers panel menu and the Adjustments panel
      tiles and menu from the bridge. HDR Toning, Match Color and Replace Color
      stay hand-wired. Verify with `tst_command_tree`, `tst_adjustments_panel`,
      `tst_layers_panel` and `tst_panels_parity`, unchanged.
- [x] 3.4 Add `AdjustmentDialog::paramSpec(page, key)` (and the protected `spec(key)`), and remove the literal ranges from
      `makeAdjustmentDialog` and from the hand-laid dialogs: exposure,
      hue/saturation, levels, curves, colour balance, black & white, photo
      filter, channel mixer and gradient map. The spec's exemptions keep their
      logic. Add any missing spec to the Rust layout, covered by an
      `adjustment_params_tests` case. Exposure Offset's four decimals move into
      the layout, so the Properties panel shows four as well. Verify with
      `tst_image_adjustments` and
      `tst_properties_panel`, unchanged, plus the 3.1 range tests.

## 4. Tool catalogue (#274)

- [x] 4.1 Capture the golden from the current `kToolTable` and `toolHintEntries`,
      including the hint command ids. Write
      `tst_keyboard_shortcuts::toolCatalogueMatchesGolden` and
      `tst_keyboard_shortcuts::toolIdOrderMatchesCatalogue`, the latter with its
      own 71-entry enumerator→name table. Then write
      `pictura-script::tool_catalogue_rows_unique_and_grouped` and
      `pictura-script::tool_catalogue_implemented_set`.
- [x] 4.2 Add `pictura-script/src/tool_catalogue.rs`: 71 `ToolSpec` rows,
      `Option<char>` shortcuts, `ToolCursor`, and modifier hints with command
      ids. Add a `tool_catalogue.rs` bridge.
- [x] 4.3 Rebuild `tool_catalog.cpp` from the bridge rows, keeping `const char*`
      fields backed by cached storage. Delete `kToolTable`, the hand list in
      `allToolIds`, the unused `implementedToolIds` and the `toolHintEntries`
      switch, and map `ToolCursor` to `Qt::CursorShape` once. Keep `ToolId`,
      `kToolCount`, the handlers, options bars, toolbox rendering and shortcut
      overrides. Verify with the 4.1 Qt tests, `tst_keyboard_shortcuts`,
      `tst_tool_hint_bar`, `tst_search_bar`, `tst_options_bars_parity` and
      `tst_smoke`, unchanged, and with self-test codes 98, 116 and 117
      unchanged.
- [x] 4.4 Update the `kToolTable` references in `docs/GLOSSARY.md`,
      `docs/02-ui-ux/search-bar.md` and `docs/dev/refactor-tool-architecture.md`
      (`TASK-ALLOWS-DOCS` commit).

## 5. Deepen type_api (#403)

- [x] 5.1 Write `tst_type_tools::editSessionCommitsOnce`,
      `tst_type_tools::droppedEditSessionCancels`,
      `tst_warp_text::warpSessionCancelsOrCommitsOnce`,
      `tst_character_styles_panel::styleLibraryReadsInOneCall`,
      `tst_paragraph_styles_panel::styleLibraryReadsInOneCall` and
      `pictura-app::type_bridge_headers_stay_in_adapter`.
- [x] 5.2 Add `TypeSpec` and switch `LayerType` and the eleven tuple calls to it:
      `previewRect`, `caretStops`, `previewRgba`, `baselines`, `overflows`,
      `pathOverflowPoint`, `localBounds`, `commitLayer`, `commitMask`,
      `updateLayer` and `commitEdit`. Drop `isType`, `layerText`, `layerFont`
      and the `layer*Setting` getters in favour of `readLayer`.
- [x] 5.3 Add `EditSession` and `WarpSession`. Each holds a `QPointer`, cancels on
      destruction and ends on a failed commit. Move `tool_type.cpp` (Type tool
      member) and the Warp Text wiring in `frame_menus_type.cpp` (a shared
      session owned by the dialog's connections) onto them.
- [x] 5.4 Add Rust `type_character_styles`/`type_paragraph_styles` snapshot calls in
      `type_tools.rs`, replacing the nine getters there (the file shrinks), and the adapter's `characterStyles` and
      `paragraphStyles`. Move the panels and dialogs to them, then delete the
      count, name, font and setting index getters from the bridge and the
      adapter.
- [x] 5.5 Point `type_frame.h` and `panels/character_panel.h` at `api/type_api.h`,
      and move `layers_api.cpp`'s warp query to `type_api::isWarped`.
- [x] 5.6 Migrate every test that calls a removed function to the new surface,
      keeping each assertion. The suites are: `tst_character_paragraph_panels`,
      `tst_type_tools`, `tst_type_on_path`, `tst_character_styles_panel`,
      `tst_paragraph_styles_panel`, `tst_text_gamma_fonts`, `tst_paragraph_type`,
      `tst_type_placeholder`, `tst_type_panels`, `tst_type_preferences`,
      `tst_warp_text`, `tst_edit_refresh`, `tst_layers_panel_fixes`,
      `tst_move_feedback`, `tst_document_windows`, `tst_save_pdf` and
      `tst_transform_skew`. Migrate the production callers too, including
      `properties_panel.cpp`, `frame_menus*.cpp` and the panels.
- [x] 5.7 Record the deepen decision, the rejected fold-back and the single header
      exception in `docs/dev/refactor-view-seam.md` (`TASK-ALLOWS-DOCS` commit),
      and update the Serena memory if it describes `type_api`.

## 6. Integration and archive

- [ ] 6.1 (The single build passed locally, 984/984 with no warnings; `verify-full.sh`
      is left to CI by request.) Run the single build (`cmake --build build --parallel` within the
      80% CPU limit), then `python3 scripts/check-test-plan.py` and
      `bash scripts/verify-full.sh`.
      - Both headless self-test runs must exit with unchanged codes (98, 116,
        117, 536).
      - The self-test budget and file-size ceilings must not grow; tighten any
        allowlist ceiling a shrunk file now allows.
      - Every per-task verification above runs here.
- [x] 6.2 Update `docs/dev/STATE.md` (`TASK-ALLOWS-DOCS`), and `ROADMAP.md` only if
      an item is affected. After 6.1 passes, run
      `openspec archive rust-single-source-bundle`, then verify with
      `openspec validate --all --strict` and
      `scripts/check-openspec-archived.sh`. Leave the untracked
      `propose-draft.md` uncommitted.
- [x] 6.3 Open the PR titled `refactor: Rust single-source tables bundle (#269)`.
      The body references #271, #272, #274 and #403, lists the changed colour
      expectations and the mechanical test migrations, and carries the manual
      row (self-test codes) as a reviewer checklist.
