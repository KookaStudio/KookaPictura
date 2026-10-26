# Proposal

## Why

The Text Color picker is modal. Changing the color in it leaves the selected type
layer (or the text being typed) at the old color until the picker closes, so the
user cannot judge the color against the artwork (#957). The engine already has a
no-history preview (`type_preview_layer`) and the Type tool already implements
`TextSession::previewTextColor`, but nothing calls it: the picker has no live
hook.

## What Changes

- `ColorPickerDialog::getColor` takes an optional live callback, called with each
  color the user picks while the dialog is open (Adobe and System pickers).
- The options-bar text color swatch and the Character panel color swatch preview
  each picked color on the type layer or text session with no history state.
- OK commits one history state; Cancel puts the original color back with no state.

## Capabilities

### New Capabilities

- `tools/type-color-live-preview`: live preview of the Text Color picker.

### Modified Capabilities

None.

## Impact

- `crates/pictura-app/cpp/color_picker_dialog.{h,cpp}` — live callback.
- `crates/pictura-app/cpp/options_bar_type.cpp` — swatch previews and restores.
- `crates/pictura-app/cpp/panels/character_panel.{h,cpp}` — swatch previews and restores.
- `crates/pictura-app/cpp/tests/tst_type_color_preview.cpp` — new suite.
- No dependency, data-format, or API changes.

Closes #957.
