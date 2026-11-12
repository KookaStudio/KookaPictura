# Tasks

## 1. Combined list state and toggle

- [x] 1.1 Replace `effectsExpanded_` / `filtersExpanded_` with one `listsExpanded_`; include a layer's synthetic groups only while the set contains the layer path.
- [x] 1.2 Seed `listsExpanded_` for a newly styled layer when **Expand New Effects** is on and for a first-seen smart filter; both auto-open the combined list.
- [x] 1.3 Draw one right-edge `listToggleRect` for effects, filters, or both; remove the separate effects/filters toggle rects and reserve one chevron in the right-edge walk.
- [x] 1.4 Route the fx badge, the list toggle, the row-control hit test, and the double-click guard through the single toggle; verify `tst_layers_smart_filters::oneToggleControlsBothLists`.

## 2. Nested geometry and gutter

- [x] 2.1 Anchor `nestedEyeColumn` to the owning layer's `imageSlotRight`, not `thumbsRight`, and indent each child row one `kIndent` step deeper.
- [x] 2.2 Use a narrow synthetic eye column (eye width + 2 px) so the eye-to-name gap halves; keep the name gap at half the normal value.
- [x] 2.3 Fill the fixed eye cell with a flat `Theme::shade(Window, 2)` and centre the layer eye in the gutter; keep the 1 px separator.
- [x] 2.4 Make the lock badge side 80% of its previous two-thirds-thumbnail side; shrink the fx badge and clipping glyph by 20% and widen the clip gap to 4 px.
- [x] 2.5 Verify `tst_layers_effects::nestedEyeAlignsWithImageThumb`, `tst_layers_smart_filters::nestedEyeAlignsWithImageThumb`, `tst_layers_panel::eyeGutterIsCentredAndSeparated`, `tst_panels_parity::eyeGutterIsFlatAndLighter`, and `tst_panels_parity::lockBadgeShrunk`.

## 3. Block painting and selection

- [x] 3.1 Suppress the bottom rule for every synthetic row whose next visible row is synthetic; draw the closing rule after the last list row. This removes the line between Effects and Smart Filters.
- [x] 3.2 In `LayersTreeView`, redirect a left press on a synthetic row to the owning layer after the base press, while storing the pressed smart-filter path for `startDrag`.
- [x] 3.3 Keep the smart-filter drag/reorder behavior; verify `tst_smart_filter_apply::dragReordersSmartFilters`.
- [x] 3.4 Verify `tst_layers_effects::clickingEffectRowSelectsLayer`, `tst_layers_smart_filters::clickingFilterRowSelectsLayer`, `tst_layers_effects::nestedRowsFormOneCell`, and `tst_layers_smart_filters::noRuleBetweenGroups`.

## 4. Existing tests, hooks, and self-test

- [x] 4.1 Update `clickListToggleForTest` to take only a path; update the effects and smart-filter tests.
- [x] 4.2 Keep `eyeGutterCentredForTest` and the centred `lpr_eye_gutter` self-test check in `selftest_layers_round3.cpp`.
- [x] 4.3 Update the faded-eye tests to sample the gutter right of the centred glyph.
- [x] 4.4 Update `test-plan.md` for every new/renamed scenario and test.

## 5. Verification and archive

- [x] 5.1 Build the app once and run the relevant Qt suites plus `tst_layers_panel_fixes`.
- [x] 5.2 Run `bash scripts/check-file-size.sh`, `python3 scripts/check-test-plan.py`, and `openspec validate --all --strict`.
- [x] 5.3 Archive the change with `openspec archive smart-filters-effects-disclosure` and verify the main spec contains the new requirements.
