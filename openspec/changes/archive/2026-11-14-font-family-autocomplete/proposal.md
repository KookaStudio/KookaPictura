# Proposal

## Why

Issue #925 reports that typing a lowercase prefix in a font family field does
not complete mixed-case family names. Qt 6.11's default `QComboBox` completer is
already case-insensitive, so the reported cause is not the cause: the typed
prefix is appended to the family already shown (for example `Liberation
Sansnoto`), and a per-keystroke family reset replaces the field with an
unrelated family: the Type options bar's sync does this on every keystroke, and
the Character panel's deferred refresh does it on its timer. The completer
therefore receives a prefix that can never match.

## What Changes

- Typing in any font family field starts a new search: the first printable key
  replaces the family already shown instead of appending to it.
- Navigation and deletion keys keep the shown text editable instead of starting
  a replacement, so a family name can be edited in place.
- The Type options bar's option sync and the Character panel's refresh stop
  writing the family back into a field whose current text already matches,
  so they no longer reset the field mid-typing.
- Family completion keeps matching case-insensitively (`noto` offers
  `Noto Sans`) in the Character panel, Type options bar, Paragraph Style
  dialog, and Glyphs panel.
- Exact-case family selection from the popup is unchanged.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `ui/font-family-picker`: adds the requirement that typing a family prefix
  starts a new, case-insensitive completion in every family input, and that
  programmatic option syncs do not interrupt typing.

## Impact

- `crates/pictura-app/cpp/font_combo.cpp` — shared family-field behavior.
- `crates/pictura-app/cpp/options_bar_type.cpp` — guarded family sync.
- `crates/pictura-app/cpp/panels/character_panel.cpp` — guarded refresh sync.
- Qt Test suites covering the four family inputs: `tst_type_session_panels`,
  `tst_paragraph_styles_panel`, and `tst_type_panels`.
- No new dependencies; no data or file-format changes.
