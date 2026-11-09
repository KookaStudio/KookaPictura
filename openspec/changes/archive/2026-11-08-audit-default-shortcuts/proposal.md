# Proposal

## Why

Issue #465 asks for a shortcut audit against Photoshop. The project targets
Photoshop CS6, so `docs/02-ui-ux/keyboard-shortcuts.md` and
`docs/02-ui-ux/menus.md` are the reference. The tool letters and workspace keys
already match CS6; three default menu bindings do not:

1. `Ctrl+Y` is claimed by both `Edit > Redo` and `View > Proof Colors`. CS6
   gives it to Proof Colors and has no separate Redo binding: `Ctrl+Z` toggles
   Undo/Redo, `Ctrl+Shift+Z` steps forward, `Ctrl+Alt+Z` steps back.
2. `Ctrl+Alt+Shift+E` is claimed by both `File > Export As…` and
   `Layer > Merge > Stamp Visible`. CS6 gives it to Stamp Visible; Export As is
   a CC-era extension with no CS6 binding.
3. Brush `F5` and Actions `F9` are bound, but the CS6 Color `F6`, Layers `F7`,
   and Info `F8` panel toggles are missing.

An ambiguous chord makes neither action fire reliably, and the shortcut editor
shows two owners. This change removes the ambiguity and fills the missing CS6
defaults.

## What Changes

- **Panel function keys.** `Window > Panels > Color`, `Layers`, and `Info` gain
  the CS6 defaults `F6`, `F7`, and `F8`; Brush `F5` and Actions `F9` stay.
- **Chord ownership.** `View > Proof Colors` keeps `Ctrl+Y` and `Edit > Redo`
  loses its default; `Layer > Merge > Stamp Visible` keeps
  `Ctrl+Alt+Shift+E` and `File > Export As…` loses its default.
- **Undo/Redo contract.** The edit-history requirement is corrected to CS6:
  Undo carries `Ctrl+Z` and performs redo when no undo state remains; Redo has
  no default binding.
- **Regression tests.** A catalogue test proves default chords are unique and
  pins the CS6 owners, and a Qt test proves the F6/F7/F8 panel keys toggle the
  panels. The quick-mask regression that redid with `Ctrl+Y` now uses
  `Ctrl+Shift+Z` (Step Forward).

## Capabilities

### New Capabilities

- `ui/default-shortcuts`: the CS6 default binding set for implemented commands,
  including unique chord ownership and the function-key panel toggles.

### Modified Capabilities

- `document/edit-history`: the Undo control's CS6 toggle behaviour and the
  removal of the Redo default binding.

## Impact

- `pictura-script`: five rows in `command_catalogue/entries.rs` and the matching
  `tests/golden/catalogue.tsv` rows; new catalogue tests.
- `pictura-app`: one Qt regression test updated and one added.
- No behavior change to the engines, the document format, or any golden image.
- Out of scope (recorded in `design.md`): brush-preset cycling (`,` `.` `<`
  `>`), the `F2`/`F3`/`F4` function-key alternates for Cut/Copy/Paste, and
  missing-feature shortcuts tracked by #508, #523, and #540.
