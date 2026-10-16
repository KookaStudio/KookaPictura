# Proposal: move-auto-select-and-refine-enablement

## Why

Issue #384 (Move and Select options-bar manual-test findings, refs #252): the
Move options bar's three-dots menu button renders a text `…` glyph instead of an
icon; the `Auto-Select` control is only a Group/Layer combo whose recorded value
is unused (a `ponytail:` note in `options_bar_move.cpp`), so the option has no
effect; and the selection tools' `Select and Mask…` button stays enabled with no
selection, where its click silently does nothing because
`refine_edge_available` refuses (the `Select ▸ Refine Edge…` menu row is already
disabled there).

Issue #385 (Eraser options-bar parity) was verified against the current tree:
`buildEraserPage` already offers the shared brush tip and preset picker, Mode
Brush/Pencil/Block, Opacity, Flow, and Erase to History, and the
`eraser_tool` self-test check passes. No change is needed; the issue is closed
with that evidence.

## What Changes

- **Move Auto-Select checkbox**: add an `Auto-Select` checkbox to the left of
  the Group/Layer combo, unchecked by default. The combo is enabled only while
  the checkbox is on.
- **Real Auto-Select**: while it is on, a Move press with no active selection
  hit-tests the topmost visible pixel layer whose content covers the press point
  and makes it (or, in Group mode, its top-level ancestor group) the active
  layer; a press on uncovered canvas keeps the active layer.
- **Ellipsis icon**: the Move menu button uses the Lucide `ellipsis` asset
  instead of the text glyph.
- **Refine Edge button enablement**: the `Select and Mask…` button is enabled
  exactly when `Select ▸ Refine Edge…` is, so a click can never be a silent
  no-op.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/tool-framework`: the Move options-bar controls requirement gains the
  Auto-Select checkbox and the hit-test behaviour, and the ellipsis-icon menu
  button; a new requirement makes an options-bar action button mirror its
  command's enablement.

## Impact

- `pictura-app` (`options_bar_move.cpp`, `tool_options.h`, `tool_services.h`,
  `tools.{h,cpp}`, `tool_move.cpp`, `frame_menus_align.cpp`, `frame_build.cpp`,
  `options_bar.{h,cpp}`, the Rust bridge `impl_layers/auto_select.rs` +
  `impl_layers.rs` + `helpers_composite.rs`), `assets/icons/ellipsis.svg`,
  `assets/pictura.qrc`, `assets/icons/lucide-map.json`, and the Qt Test suites
  `tst_options_bars_parity` / `tst_refine_edge`. No new dependency.
- Ceilings (`ponytail:`): the hit test reads a layer's own alpha and ignores
  layer masks, clipping, effects, and live type/shape rendering, so a masked-out
  or unmaterialized pixel is still a hit; a hit that resolves to a group selects
  it but the move preview still needs a single pixel layer, so dragging the
  group does nothing (as for any group active layer today).
