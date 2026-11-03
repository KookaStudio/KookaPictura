# Proposal

## Why

The Properties panel has no adjustment Preset menu. Only Color Lookup exposes a
preset parameter (`crates/pictura-render/src/adjustment_params.rs`); every other
adjustment either hard-codes a few built-ins inside its Image > Adjustments
dialog or offers nothing, and no adjustment can save its settings as a named
preset that survives a restart (#576, parity gap `ADJ-6`).

CS6 moves the preset list from the Adjustments panel into the Properties panel
and lets Levels, Curves, Exposure, Hue/Saturation, Black & White, Channel Mixer,
and Selective Color save settings as presets (`docs/04-image-ops/
adjustments-overview.md`).

## What Changes

- **Properties preset menu.** A Preset menu appears for the issue's five kinds
  (Levels, Curves, Photo Filter, Selective Color, Vibrance). It lists `Default`,
  the kind's built-ins, saved presets, and `Custom`.
- **Save / Load Preset.** The Properties panel menu gains `Save Preset…` and
  `Load Preset…`, gated to the CS6 save whitelist. Saved presets are appended to
  the kind's list and applied as one history state.
- **Dialogs.** The relevant Image > Adjustments dialogs list the same saved
  presets in their Preset menu and apply them to the scratch block.
- **Built-ins only where sourced.** The existing Levels/Exposure/Hue-Saturation/
  Black & White tables, the community Curves list, and the Photo Filter filter
  colours are reused. No new built-in looks are invented for Channel Mixer,
  Selective Color, or Vibrance.
- **Persistent store.** User presets live in the XDG user data directory, one
  file per kind, written atomically by the Rust engine.

## Capabilities

### Added Capabilities

- `imaging/adjustment-presets`: the preset menu, save/load commands, built-in
  catalogue, kind mapping, and persistent store.

## Impact

- `crates/pictura-render/src/adjustment_presets.rs`: kind mapping, decoded
  block matching, save whitelist, and the user preset store.
- `crates/pictura-app/src/cxxqt_object/adjustment_presets.rs` and
  `adjustment_edit.rs`: the C++ bridge.
- `crates/pictura-app/cpp/adjustment_presets.{h,cpp}`: built-in registry and the
  reusable `PresetCombo`.
- `crates/pictura-app/cpp/panels/properties_panel.{h,cpp}`,
  `adjustment_dialog.{h,cpp}`, and the hand-laid adjustment dialogs: the menu,
  the Save/Load actions, and reload-on-preset.
- `ROADMAP.md` and `docs/dev/doc-code-parity-gaps.md`: tick `#576` and close
  `ADJ-6`.
- Non-goals: external preset-file import/export (the CS6 file format is
  undocumented), deleting presets, Gradient Map presets (#574), Vibrance Auto
  (#564), the Selective Color composite gate (#577), and the shared Auto buttons
  (#578).
