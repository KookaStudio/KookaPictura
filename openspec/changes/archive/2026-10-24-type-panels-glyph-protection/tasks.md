# Tasks

## 1. Test plan

- [x] 1.1 Write `test-plan.md` with one row per scenario in every delta spec, naming the automated test or `manual: <reason>` in the Coverage column; verify the row count equals the `#### Scenario:` count across `specs/**/spec.md`; where two requirements share a scenario title, write one exact-title row per requirement
- [x] 1.2 Add the planned Qt Test and Rust test names to the plan before implementation so each later group can check its own rows off; verify `scripts/check-test-plan.py` passes once the named tests exist

## 2. Model, codec, glyph identity, and preferences plumbing

- [x] 2.1 Add `CharacterAttrs.tsume`, `ParagraphAttrs.direction`, and the `TextDirection` enum to `pictura-core`, merge them through the sparse override path, and verify `pictura-core::type_tool::tests::language_tsume_and_direction_round_trip` passes
- [x] 2.2 Map tsume and direction in the app bridge's character/paragraph conversions (language is already mapped) and verify the existing bridge unit tests plus `pictura-app::tsume_and_direction_cross_the_bridge` cover both directions
- [x] 2.3 Author `/Tsume`, `/Language`, and `WritingDirection` (parameterize `type_shape::rendered` instead of its hardcoded 0) and extract them with documented defaults; language falls back to the style-sheet default like the other character keys. Verify `pictura-codec::type_write::tests::tsume_language_and_direction_survive_author_and_read`, `pictura-codec::type_write::tests::existing_direction_survives_unrelated_reauthor`, `pictura-codec::engine_data::tests::tsume_language_and_direction_decode`, and `pictura-codec::engine_data::tests::language_falls_back_to_sheet_default` pass
- [x] 2.4 Add `face`, `cluster`, and `x_scale` to `ShapedGlyph` and `PlacedGlyph`, add the face-id palette to `TextLayout`, and thread them through every constructor, test, and layout consumer (`text_render`, `type_layer`, `type_path`, `type_warp`); verify `cargo test -p pictura-core` and `cargo test -p pictura-render --lib` pass
- [x] 2.5 Add the process-wide type preferences (glyph protection in `pictura-render`, East Asian and Middle Eastern in the app session) across `preferences_dialog.{h,cpp}`, `frame_preferences.cpp`, and `session.{h,cpp}`, persist them in the session store, and add the Preferences signals that push them to the panels and the renderer. Add `tst_type_preferences` to `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify `tst_type_preferences::defaults` and `tst_type_preferences::persists` pass
- [x] 2.6 Update the parity register and `ROADMAP.md` for the new modelled attributes, marking ungrounded PSD keys as approximations; verify `python3 scripts/check-doc-claims.py` (and `bash scripts/guard.sh` with `TASK_ALLOWS_DOCS=1`) and use `TASK-ALLOWS-DOCS` in the docs commit

## 3. #517 Character panel

- [x] 3.1 Register style-specific faces: key the renderer registry by family and style with an insertion-ordered list, resolve exact → family regular → bundled, make `opentype_features`/`font_has_outlines`/`font_postscript_name`/`font_family` style-aware, update every `face_for` call site (`type_layer`, `type_paragraph`, `type_path`, `type_warp`, `type_caret`) and `pdf_text`'s `type_pens` consumer, and update `typeFontBytes`/`registerTypeFont` plus the `type_register_font` bridge/`type_api::registerFont` signature, register the layer's family+style when a type layer or session opens (`tool_type`, `character_panel`, `paragraph_panel`), and update every C++ caller (`paragraph_styles_panel`, `frame_pdf`, tests); verify `pictura-render::fonts::tests::style_faces_resolve_and_fall_back` passes using the repository's Noto Sans Regular/Bold faces
- [x] 3.2 Add the Character panel's font-style combo, its population from the selected family, and its commit path, including `copyPanelFields`/refresh so the new field is not dropped by another edit; verify `tst_character_paragraph_panels::fontStyleApplies` passes
- [x] 3.3 Add the Language combo and its commit path, including `copyPanelFields`/refresh; verify `tst_character_paragraph_panels::languageRoundTrips` passes
- [x] 3.4 Add the Set Tsume field, shown only with the East Asian preference, and its commit path, including `copyPanelFields`/refresh; verify `tst_character_paragraph_panels::tsumeFollowsAsianPreference` passes and the stored value survives toggling the preference
- [x] 3.5 Add the Character panel menu (triangle) with a checkable Fractional Widths item (including `copyPanelFields`/refresh) and round advances in shaping when it is off; verify `tst_character_paragraph_panels::fractionalWidthsMenu` and `pictura-render::text_render::tests::fractional_widths_off_rounds_advances` pass
- [x] 3.6 Apply tsume in shaping for CJK glyphs and verify `pictura-render::type_style::tests::tsume_compresses_a_full_width_advance` passes with the documented approximation; the test feeds a synthetic full-width advance because no bundled face carries CJK
- [x] 3.7 Confirm the already-shipped Faux Bold and Underline paths still pass (`tst_character_paragraph_panels::styleButtonAppliesToggle`, `tst_character_paragraph_panels::characterPanelEditsNewText`), then update the parity register (PAN-9) and `ROADMAP.md` for the completed Character panel controls; use `TASK-ALLOWS-DOCS` in the docs commit

## 4. #519 Character Styles panel

- [x] 4.1 Add the character-style read bridge (`count`, `name`, `font`, `character`), a preview call, and an edit call that takes a new name and rewrites every applying layer's style reference on rename; expose them through `crates/pictura-app/cpp/api/type_api.{h,cpp}`. Verify `pictura-app::character_style_reads_resolve_defaults`, `pictura-app::renaming_an_applied_style_keeps_it_applied`, and the existing style model tests pass
- [x] 4.2 Build `CharacterStylesPanel` from the Paragraph Styles panel pattern: list, click-to-apply, footer create/delete, double-click edit; register `tst_character_styles_panel` in `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify `tst_character_styles_panel::listsAndApplies`, `createsAndDeletes`, `editsAndReapplies`, and `renameKeepsApplied` pass
- [x] 4.3 Add the character-only mode to the style options dialog (title "Character Style Options", the three character pages) and wire Preview through the new preview bridge; verify `tst_character_styles_panel::dialogPages`, `dialogRoundTrips`, and `dialogPreviewRestoresOnCancel` pass
- [x] 4.4 Wire `Window > Panels > Character Styles` and `Type > Panels > Character Styles`, register the panel in `CMakeLists.txt`, `command_ids.h`, `frame.h`, `frame_includes.h`, `frame_build.cpp`, `frame_menus.cpp`, `frame_session.cpp`, `frame_workspaces.cpp`, `search_index.cpp`, and the panel column, and verify `tst_character_styles_panel::menusToggle` passes
- [x] 4.5 Add the `window.panels.characterStyles` icon (Lucide-derived or Lucide-style), qrc entry, provenance record, and icon-asset assertion; verify `tst_icon_assets::characterStylesGlyphResolves` passes
- [x] 4.6 Update the parity register (PAN-11) and `ROADMAP.md`; use `TASK-ALLOWS-DOCS` in the docs commit

## 5. #518 Paragraph panel

- [x] 5.1 Add the Paragraph panel menu (triangle) with `Justification…`, opening a dialog for word, letter, glyph min/desired/max and auto leading; verify `tst_character_paragraph_panels::justificationDialogRoundTrips` and `tst_character_paragraph_panels::justificationCancelWritesNothing` pass
- [x] 5.2 Implement the CS6 spacing order in `type_justify` with per-field bounds, desired values on every line, and glyph x-scaling; verify `pictura-render::type_justify::tests::word_spacing_absorbs_the_leftover` and `pictura-render::type_justify::tests::bounds_clamp_and_leave_ragged` pass
- [x] 5.3 Thread paragraph auto-leading through `leading_px` and every line-measuring caller (`layout_type`, `text_box_any`, `type_caret`, `type_paragraph`); verify `pictura-render::type_layer::tests::auto_leading_uses_paragraph_value` passes
- [x] 5.4 Add the `unicode-bidi` dependency to `crates/pictura-render/Cargo.toml`, add bidi run ordering and RTL shaping, then add the Paragraph panel direction control behind the Middle Eastern preference; verify `pictura-render::text_render::tests::rtl_line_reverses_visual_order`, `pictura-render::text_render::tests::mixed_direction_keeps_runs`, and `tst_character_paragraph_panels::directionFollowsMiddleEasternPreference` pass
- [x] 5.5 Place and move the caret in reading order for RTL text; verify `pictura-render::type_caret::tests::rtl_caret_zero_is_at_right_edge` and `tst_character_paragraph_panels::rtlDirectionApplies` pass
- [x] 5.6 Verify RTL paragraph wrapping and overflow with `pictura-render::type_paragraph::tests::rtl_line_starts_at_right_edge` and `pictura-render::type_paragraph::tests::rtl_wraps_at_same_words` pass
- [x] 5.7 Update the parity register (PAN-10 and TOOL-17) and `ROADMAP.md`; use `TASK-ALLOWS-DOCS` in the docs commit

## 6. #887 Missing Glyph Protection

- [x] 6.1 Wire the Type preferences checkbox to the process-wide renderer setting; verify `tst_type_preferences::glyphProtectionTogglesFallback` passes
- [x] 6.2 Add face coverage queries and deterministic fallback selection to `fonts.rs` over an ordered fallback list; verify `pictura-render::fonts::tests::fallback_prefers_active_then_registration_order` passes
- [x] 6.3 Split shaping into coverage runs that carry per-glyph faces, and use those faces for advances, wrapping, caret stops, path layout, warped outlines, and rasterization; make `pdf_live_type` refuse live text when a glyph needs fallback so PDF rasterizes it. Verify `pictura-render::type_style::tests::fallback_run_uses_second_face_advance`, `pictura-render::type_paragraph::tests::mixed_line_wrap_uses_fallback_advance`, and `pictura-render::type_caret::tests::mixed_line_caret_matches_fallback_advance` pass
- [x] 6.4 Add the app-side fallback registration (document open, type-session open, and a newly typed character) and verify the bundled Noto Sans substitutes U+0370, which Liberation Sans lacks, in `tst_type_tools::missingGlyphFallbackRegisters`
- [x] 6.5 Update the parity register (TOOL-17) and `ROADMAP.md`; use `TASK-ALLOWS-DOCS` in the docs commit

## 7. Integration

- [x] 7.1 Run `bash scripts/verify-fast.sh` and fix every failure; confirm `openspec validate --all --strict`, `scripts/check-test-plan.py`, and the file-size and selftest-budget guards pass
- [x] 7.2 Run the CMake build and `ctest --test-dir build -R '^tst_' --output-on-failure`, then `./build/pictura --headless --self-test`
- [x] 7.3 Archive the change with `openspec archive type-panels-glyph-protection` and verify only `openspec/changes/archive/` remains and `openspec validate --all --strict` passes
