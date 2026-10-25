# Proposal: Layers panel smoke-test follow-ups

## Why

The manual smoke pass over `assets/layers_panel_smoke.psd` found three classes of
defect that the archived layers-panel bundle did not cover:

- A mask thumbnail clicked on a non-active row loses the mask edit target after
  the tree changes selection, so the Brush paints the layer while the mask thumb
  still shows the active border.
- Ctrl-click on a mask thumbnail falls through to the tree's Ctrl-click row
  toggle instead of loading the mask as a selection.
- `filter_target_ready()` rejects an active mask target on a fill, adjustment,
  or group row, so Ctrl-I and Image adjustments do nothing even though the mask
  operation path already supports them.

The same pass found CS6 row-menu gaps: fill/adjustment rows had no mask, vector
mask, clipping, or merge rows; type rows had no text controls; group rows hid
export and Blending Options. Group effects are not rendered yet, so this change
enables the group Blending Options page only and leaves effect pages disabled.

## What Changes

- Preserve the mask edit target when a mask thumbnail activates a non-active row;
  consume mask-thumbnail clicks instead of letting the tree clear the target.
- Make Ctrl-click on a raster mask thumbnail load its coverage as a selection.
- Treat an active mask target as a valid Image-adjustment/filter target.
- Add the fixture's missing default masks (Invert and Solid Fill rows).
- Extend the row menu:
  - per-kind Duplicate/Delete labels;
  - fill/adjustment raster-mask, vector-mask, clipping, merge, export, and
    Convert-to-Smart-Object rows;
  - type text controls (orientation, anti-aliasing, point/paragraph, Warp
    Text…), export, and Convert to Smart Object;
  - group export, vector-mask rows, and Blending Options;
  - visible disabled Copy CSS/Copy SVG, Artboard/Frame, Create Work Path, and
    Convert to Shape placeholders.
- Allow `blending.*` Layer Style writes on groups; keep effect writes refused
  until group effects render (#939).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/layers-panel`: per-kind row-menu follow-ups and labels.
- `compositing/layer-masks`: mask-target activation, Ctrl-click selection, and
  mask-target adjustment routing.
- `ui/layer-style-dialog`: group Blending Options with disabled effect pages.
- `compositing/layer-style-authoring`: group `blending.*` writes.

## Impact

- C++ panel: `layers_panel_events.cpp`, `layers_panel_menu.cpp`,
  `layers_panel_type_menu.cpp`, `layer_style_dialog.cpp`, `layers_panel.h`.
- Rust: `filter_tools.rs`, `layer_style.rs` (app and render),
  `layer_style.rs` write guard.
- Fixture: `crates/pictura-codec/examples/make_smart_fixtures.rs` and
  `assets/layers_panel_smoke.psd`.
- Tests: `tst_mask_edit`, `tst_layers_row_menu`, `tst_layer_style`,
  `tst_last_layer`.
- Docs: `docs/02-ui-ux/panels/layers-panel.md`,
  `docs/05-layers/layer-management-ui.md`, `docs/dev/STATE.md`.
- Follow-up: #939 renders group effects.
