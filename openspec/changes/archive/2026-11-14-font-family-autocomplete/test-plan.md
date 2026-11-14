# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ffp_typed_prefix` | Select a type layer, open the Character panel with `characterFamily` on a non-Noto family, focus the field, type `noto` | The field text starts from `noto` and is not `<old family>noto` | `tst_type_session_panels::fontFamilyTypeAheadReplacesShownFamily` |
| `ffp_nav_keeps_text` | Focus `characterFamily` showing `Liberation Sans`, press End, type ` x` | The field keeps `Liberation Sans` and appends ` x`; the text is edited, not replaced | `tst_type_session_panels::fontFamilyNavigationKeepsShownText` |
| `ffp_prefix_case` | Type lowercase `noto` into `characterFamily`, then ` s` | The completion set contains `Noto Sans`; after ` s` the field selects `Noto Sans` | `tst_type_session_panels::fontFamilyTypeAheadReplacesShownFamily` |
| `ffp_all_inputs` | Type `noto` into `characterFamily`, `optionsTypeFamily`, `paragraphStyleFamily`, and `glyphsFamily` | Each field offers `Noto Sans` case-insensitively | `tst_type_session_panels::fontFamilyTypeAheadReplacesShownFamily`, `tst_paragraph_styles_panel::optionsTypeFamilyTypeAhead`, `tst_paragraph_styles_panel::paragraphStyleFamilyTypeAhead`, `tst_type_panels::glyphsFamilyTypeAhead` |
| `ffp_live_sync` | Type `noto` into `characterFamily` during a live text session, call `panel->refresh()` (or wait for the panel refresh), assert the completion set survives, then type ` s`; type `noto` into the active HorizontalType page's visible `optionsTypeFamily` | Each field keeps the typed prefix and match set instead of jumping to an unrelated family; after ` s` the Character panel selects `Noto Sans` | `tst_type_session_panels::fontFamilyTypeAheadDuringSession`, `tst_paragraph_styles_panel::optionsTypeFamilyTypeAhead` |
| `ffp_exact_pick` | Select `Noto Sans` in `characterFamily` by its exact-case list entry | `Noto Sans` is shown and applied exactly | `tst_type_session_panels::fontFamilyExactSelectionUnchanged` |
