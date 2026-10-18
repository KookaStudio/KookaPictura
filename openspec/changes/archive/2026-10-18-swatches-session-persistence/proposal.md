# Proposal: swatches-session-persistence

## Why

The owner asked that swatches added in the Color Picker survive closing and
reopening Pictura. The Swatches panel is currently session-only, so a swatch
added through the picker (or the panel's New action) disappears on restart.
`CLR-003` / `PAN-011` already specify that new colours persist between editing
sessions.

## What Changes

- `SessionState` carries a `swatches` array (`{name, color}`) and a transient
  `swatchesLoaded` flag: an absent key means the default palette, while an
  intentionally empty panel stays empty.
- `PicturaMainWindow::saveSession` writes the live grid; `buildPanels` restores
  it before wiring `SwatchGrid::countChanged` to the session save, so adding,
  deleting, or resetting a swatch persists immediately.
- An unrelated settings save preserves the stored swatches: the key is only
  rewritten when the state was loaded with it, or when the frame supplies the
  live grid.
- Tests: session round-trip, unrelated-save preservation, and two-window
  restart tests for an added swatch and an emptied panel.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `ui/color-swatches-panel`: the panel's contents persist across sessions.

## Impact

- `crates/pictura-app/cpp/session.{h,cpp}` — the persisted array and flag.
- `crates/pictura-app/cpp/frame.h`, `frame.cpp`, `frame_build.cpp`,
  `frame_session.cpp` — restore and save.
- `crates/pictura-app/cpp/tests/tst_session_store.cpp` and
  `tst_swatches_panel.cpp`.
- No Rust change; no new dependencies.

## Ceilings

`ponytail:` `.aco`/`.ase` libraries and a dedicated preferences file (rather
than the session store) remain with #605/#489. The store keeps the whole
current grid, including the built-in palette; Reset restores and persists the
defaults.
