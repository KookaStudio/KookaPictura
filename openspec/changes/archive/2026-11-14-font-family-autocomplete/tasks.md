# Tasks

## 1. Add the failing Qt tests

- [x] 1.1 Add `tst_type_session_panels::fontFamilyTypeAheadReplacesShownFamily`: show the Character panel (hidden by default), put `characterFamily` on `Liberation Sans` explicitly, focus it, type `noto`, and assert the field starts from the typed prefix and the completion set contains `Noto Sans` (`ffp_typed_prefix`, `ffp_prefix_case`); build and run the suite, confirming the case fails before the fix.
- [x] 1.2 Add `tst_type_session_panels::fontFamilyExactSelectionUnchanged`: select `Noto Sans` from `characterFamily` and assert the exact family is shown and applied (`ffp_exact_pick`); confirm it passes before the fix.
- [x] 1.3 Add `tst_type_session_panels::fontFamilyTypeAheadDuringSession`: start an uncommitted text session with a canvas drag (`makeTypeLayer` commits), show the Character panel, type `noto` into `characterFamily`, then call `panel->refresh()` (or wait out the 120 ms panel refresh), assert the completion set survives, then type ` s` and assert `Noto Sans` (`ffp_live_sync`); confirm it fails before the guard.
- [x] 1.4 Add `tst_paragraph_styles_panel::optionsTypeFamilyTypeAhead`: activate `ToolId::HorizontalType` and type `noto` into that page's visible `optionsTypeFamily` combo (four pages share the object name), then assert the prefix survives the option sync and `Noto Sans` is offered (`ffp_all_inputs`, `ffp_live_sync`); confirm it fails before the fix.
- [x] 1.5 Add `tst_paragraph_styles_panel::paragraphStyleFamilyTypeAhead`: from a polling `QTimer` callback while the modal dialog's event loop runs, type `noto` into `paragraphStyleFamily` in an open Paragraph Style dialog and assert `Noto Sans` is offered (`ffp_all_inputs`); confirm it fails before the fix.
- [x] 1.6 Add `tst_type_panels::glyphsFamilyTypeAhead`: type `noto` into `glyphsFamily` with the Glyphs panel open and assert `Noto Sans` is offered (`ffp_all_inputs`); confirm it fails before the fix.
- [x] 1.7 Add `tst_type_session_panels::fontFamilyNavigationKeepsShownText`: focus `characterFamily` showing `Liberation Sans`, press End, type ` x`, and assert the family is kept and ` x` appended (`ffp_nav_keeps_text`); confirm it passes before the fix.

## 2. Shared family-field typing

- [x] 2.1 Add the fresh-search event filter to `useFastFontPreviews()` in `font_combo.cpp`: the first printable key after focus or an option write selects the field; arrows, Home/End, PageUp/PageDown, and Backspace/Delete disarm replacement while modifier keys do not; an input-method commit replaces while the search is fresh; `currentIndexChanged`, `activated`, and `editingFinished` re-arm it; verify 1.1, 1.5, and 1.6 pass and 1.2 still passes.
- [x] 2.2 Run the existing `tst_paragraph_styles_panel`, `tst_character_paragraph_panels`, and `tst_type_session_panels` suites offscreen and confirm typed, popup-picked, and uninstalled family names still work.

## 3. Keep the typed prefix through option syncs

- [x] 3.1 Guard `OptionsBar`'s `sync()` in `options_bar_type.cpp`: skip `setCurrentFont` when `family->currentText()` already equals the option's family; verify 1.4 passes.
- [x] 3.2 Guard `CharacterPanel::refresh()` in `character_panel.cpp` the same way; verify 1.3 passes.
- [x] 3.3 Run `tst_type_session_panels`, `tst_character_paragraph_panels`, `tst_paragraph_styles_panel`, `tst_type_panels`, and `tst_fonts` offscreen and confirm no family, popup, or preview regressions.

## 4. Verify and archive

- [x] 4.1 Run `bash scripts/verify-fast.sh` and confirm fmt, clippy, the unified test report, file-size, guard, and OpenSpec gates pass.
- [x] 4.2 Archive with `openspec archive font-family-autocomplete` and run `openspec validate --all --strict`; confirm only the archived copy of the change remains under `openspec/changes/archive/`.
