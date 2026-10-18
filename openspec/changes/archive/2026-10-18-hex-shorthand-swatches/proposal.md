# Proposal: hex-shorthand-swatches

## Why

The Color Picker's hex field accepts exactly six bare hex digits, so pasting
`#aabbcc` or `0xAABBCC` or typing the CS6 three-digit shorthand `#123` does
nothing. The dialog's Add to Swatches button is disabled even though the
Swatches panel exists and can already accept colours (issue #603, parity
register row `CLR-6`).

## What Changes

- The hex field accepts a leading `#` or `0x` (case-insensitive) and three-digit
  shorthand, expanding `#123` to `#112233`. Six-digit values apply while typing;
  shorthand applies on Enter or focus loss. Invalid input leaves the colour
  alone and restores the field to the current colour on commit.
- Add to Swatches opens a name prompt (default `Swatch N`, matching the issue's
  screenshots), appends the picker's current colour to the Swatches panel, and
  leaves the picker open.
- `SwatchGrid` gains `nextSwatchName()`; the panel's New action and the picker's
  prompt both use the generated `Swatch N` name.
- `ColorPickerDialog` takes an optional `SwatchGrid*` and exposes a default
  grid; `PicturaMainWindow` registers its Swatches grid so every picker entry
  point can add to the panel.
- A new `tst_color_picker` Qt Test suite and one new `tst_swatches_panel` case.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `ui/color-picker`: hex shorthand/prefix parsing and the Add to Swatches action.
- `ui/color-swatches-panel`: swatches added from the picker and generated
  `Swatch N` names.

## Impact

- `crates/pictura-app/cpp/color_picker_dialog.{h,cpp}` — parsing, the name
  prompt, and the default grid.
- `crates/pictura-app/cpp/panels/swatches_panel.{h,cpp}` — generated names.
- `crates/pictura-app/cpp/frame_build.cpp` — register the panel's grid.
- `crates/pictura-app/cpp/tests/` — new `tst_color_picker`, one new
  `tst_swatches_panel` case, and the CMake test list.
- `docs/dev/doc-code-parity-gaps.md` and `docs/dev/STATE.md`.
- No Rust change; no new dependencies.

## Ceilings

`ponytail:` added swatches are session-only. Preference persistence and
`.aco`/`.ase` libraries remain with #605/#489. Four- and eight-digit alpha
shorthand, the Color panel's hex field, and the CC-only "Add to my current
library" checkbox are out of scope.

## Provenance

Issue #603, spawned by the docs↔code parity audit (#486). Behaviour follows
`docs/07-color-painting/color-picker.md` (acceptance criteria 9–10) and the
owner's screenshots on the issue.
