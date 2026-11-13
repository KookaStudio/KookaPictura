# Proposal

## Why

In the Layers panel, clicking a vector-mask thumbnail currently replaces the
raster mask's selection and clears its edit target, so a layer that carries both
masks can never show the two mask thumbnails selected together. CS6 treats the
vector mask as an additional selection alongside one companion thumbnail, and
the Layers panel is the only way to see and switch that state.

## What Changes

- Replace the single per-row active thumbnail with a three-bit selection:
  content, raster mask, and vector mask, with exactly one companion (content or
  raster mask) alongside an optional vector mask.
- On the active layer:
  - clicking the content thumbnail selects content and preserves an active
    vector mask, clearing the raster-mask edit target;
  - clicking the raster-mask thumbnail selects it and preserves an active vector
    mask, making the raster mask the edit target;
  - clicking the vector thumbnail toggles it while keeping the current
    companion, and the raster-mask edit target follows that companion.
- Clicking a thumb on an inactive layer first activates that layer and starts
  from the content companion: content only, raster mask only, or content plus
  vector.
- Selecting another layer clears the previous layer's mask and vector selection.
- Modifier clicks keep the current companion and vector selection: `Shift`
  toggles the mask's enabled state, `Ctrl` loads the raster-mask coverage, and
  `Alt`+`Shift` toggles the rubylith overlay without moving the selection;
  `Alt`-clicking the raster mask still selects it as the companion and target.
- Clicking the row body stays content-only, as the existing row-click
  requirement specifies.
- Add Qt coverage for the active-layer toggle sequence, the companion switches,
  inactive-row clicks, the vector-only row, and the bracket rendering.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/layers-panel`: "Layer mask row indicators" now defines the companion-based
  multi-thumbnail selection, its target routing, the inactive-row starting
  state, clearing on layer change, and modifier-click preservation; "Layers
  panel content and mask thumbnail layout" now states that the content and
  raster-mask thumbnails are the active edit target, while the vector-mask
  thumbnail is an additional bracketed selection.

## Impact

Layers panel model/delegate/event handling and the active-thumb state in
`crates/pictura-app/cpp/panels/`; Qt Test suites
`tst_canvas_mask_domain`, `tst_panels_parity`, `tst_layers_panel`,
`tst_mask_edit`, `tst_quick_mask`, `tst_layers_panel_fixes`, and
`tst_layers_snapshot`. No engine, codec, dependency, or file format changes.
