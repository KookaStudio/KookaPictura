# Design

## Scope

| Surface | Kinds |
|---|---|
| Properties Preset menu | Levels, Curves, Photo Filter, Selective Color, Vibrance |
| `Save Preset…` / `Load Preset…` | Levels, Curves, Exposure, Hue/Saturation, Black & White, Channel Mixer, Selective Color |
| Dialog Preset menu | the five above plus Exposure, Hue/Saturation, Black & White, Channel Mixer |

The menu is the issue's five. The save/load gate is the CS6 whitelist; the four
whitelist kinds outside the issue keep their dialog Preset menus and gain the
panel menu commands, so a saved preset can always be re-applied. Photo Filter
and Vibrance are not in the whitelist, so their panel menu offers `Default`,
built-ins where they exist, and `Custom`, but `Save Preset…` stays disabled.

## Engine (`pictura-render`)

`adjustment_presets.rs` owns the facts that are not Qt:

- `kind_of(block) -> Option<&'static str>` maps a decoded `Adjustment` to its
  catalogue key. This is the same mapping `helpers::adjustment_kind` uses for
  the Layers panel; the store keys on it.
- `blocks_match(a, b)` compares decoded adjustments, so two encodings of the
  same settings match.
- `is_saveable(kind)` is the seven-kind whitelist.
- `supports_menu(kind)` is the issue's five.
- `saved_names(kind)`, `saved_block(kind, name)`, `save(kind, name, block)`,
  and `delete(kind, name)` read and write the user store.

### Store

- Root: `$XDG_DATA_HOME/kooka-pictura/Presets/Adjustments/`; when
  `XDG_DATA_HOME` is unset, `$HOME/.local/share/kooka-pictura/Presets/
  Adjustments/`. No `QSettings`.
- One file per kind: `<kind>.presets`, UTF-8, one record per line:
  `<name>\t<hex block bytes>`. Hex keeps the format dependency-free and
  byte-exact; the block includes its four-byte adjustment key.
- Names are trimmed, control characters become spaces, and the reserved names
  `Default` and `Custom` are rejected. A malformed line is skipped; an
  unreadable file reads as empty.
- Writes go to `<kind>.presets.tmp` in the same directory and are renamed over
  the target, so a crash cannot leave a half-written store.
- `kind` is checked against the catalogue before it becomes a path component.

Tests call the directory-taking helpers directly, so they do not mutate the
process environment; the bridge reads the XDG variables at call time.

## Bridge (`cxxqt_object`)

A small `adjustment_presets.rs` bridge exposes the store and kind facts to C++:

- `adjustment_preset_kind(block)`
- `adjustment_preset_saved_names(kind)`
- `adjustment_preset_saved_block(kind, name)`
- `adjustment_preset_save(kind, name, block)`
- `adjustment_preset_delete(kind, name)`
- `adjustment_preset_can_save(kind)`
- `adjustment_preset_supports(kind)`
- `adjustment_preset_matches(a, b)`

`adjustment_edit.rs` gains `adjustment_block(view, path)` and
`adjustment_set_block(view, path, block)`. The latter replaces the layer's block
through `edit_adjustment`, so a preset application previews and then commits as
one state like a slider gesture.

## Shell (`cpp/adjustment_presets.{h,cpp}`)

- `BuiltinPreset { name, build(current) -> block, matches(block) }`.
- `builtinPresets(kind)` returns the established tables:
  - Levels: the table currently in `levels_dialog.cpp`.
  - Curves: the community names from `docs/04-image-ops/adjustments/curves.md`
    (Color Negative, Cross Process, Darker, Increase Contrast, Lighter, Linear
    Contrast, Medium Contrast, Negative, Strong Contrast). The control points
    are an approximation; the design records that rather than claiming parity.
  - Photo Filter: the Wratten/colour filter table already in
    `photo_filter_dialog.cpp`; each preset changes only `color`.
  - Channel Mixer, Selective Color, Vibrance: no built-ins.
- `PresetCombo` is a `QComboBox` that lists `Default`, built-ins, a separator,
  saved presets, a separator, and `Custom`. It emits the block to apply and
  updates its current item by matching the layer's block. Reserved names and
  separators are not selectable.
- The Levels, Exposure, Hue/Saturation, and Black & White dialogs keep their
  existing built-in tables and append the saved names; the other dialogs use
  `PresetCombo`.

`ponytail:` the Curves point values are inferred from the named looks, not
captured from CS6; the built-in list is the community-sourced set. A CS6
capture can replace the table without changing the mechanism.

## Properties panel

- `showAdjustment` asks the active `LayerRow.adjustment_kind` whether the menu
  is supported and shows a `PresetCombo` above the controls. The combo is
  hidden for every other kind.
- Choosing a preset calls `adjustment_set_block`, marks the panel dirty, and
  commits immediately (`Modify <title> Layer`).
- `performPanelMenuAction("savePreset")` prompts for a name (a test provider
  replaces the modal), reads the live block with `adjustment_block`, saves it,
  and reloads the combo.
- `performPanelMenuAction("loadPreset")` prompts with the saved names and
  applies the chosen block.
- The panel-menu hook enables `Save Preset…` only for a saveable active kind and
  `Load Preset…` only when that kind has saved presets.

## Dialogs

`AdjustmentDialog` gains:

- `virtual void reloadFromBlock()` — subclasses reload their controls after a
  whole block is applied; the generic page reloads `AdjustmentControls`.
- `setBlock` calls `reloadFromBlock()`.
- `addPresetRow(layout)` — the shared `PresetCombo` for a dialog, connected to
  `setBlock`.

The generic `AdjustmentDialog` adds the row when the block's kind is supported,
which covers Selective Color. `SliderDialog` (Vibrance) and the hand-laid
Curves, Photo Filter, Exposure, and Channel Mixer pages call `addPresetRow`.
Levels, Hue/Saturation, and Black & White append saved names to their existing
combos and apply a saved block through `setBlock`.

## Verification

- Rust unit tests cover store round-trip, reserved names, malformed lines, kind
  mapping, decoded matching, and the two kind sets.
- A new `tst_adjustment_presets` Qt suite covers the menu, application history,
  `Custom`, save/load, the whitelist, dialog loading, built-ins, Photo Filter
  density, restart, and a malformed store.
- `openspec validate --all --strict`, `cargo fmt`, `cargo clippy`, and
  `scripts/verify-fast.sh` are the gates.
