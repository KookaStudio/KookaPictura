# Proposal: Layers panel interaction bundle

## Why

Six open Layers-panel issues share the same row model, target, and thumbnail
code, and the OpenSpec corpus pins several of the broken behaviors in place:
the canvas keeps an opaque backdrop when the Background is hidden (#900), new
adjustment and fill layers land at the top of the stack (#898), group masks are
created zero-sized so their thumbnail is unreachable (#897), adjustment and
fill rows paint a generic checkerboard (#894), the mask defaults conflate two
CS6 preferences and the created adjustment does not target its mask (#895a),
and the row context menu is a flat table with the wrong block order, missing
CS6 rows, and the full generic command set on the Background (#720).

The behavioral contract already describes the intended behavior:
`docs/05-layers/adjustment-layers.md:55-62`, `docs/05-layers/fill-layers.md:37-47`,
`docs/02-ui-ux/panels/layers-panel.md:52-56`, and
`docs/02-ui-ux/panels/layers-panel.md:38`.

## What Changes

- **#900** — remove the canvas-opaque special case. The composite covers the
  checkerboard when a Background is visible; hiding the Background reveals the
  transparency checkerboard.
- **#898** — adjustment and fill creation insert directly above the active
  layer (or as a selected group's topmost child) and select the created node.
- **#897** — a group's Reveal All / Hide All / From Transparency mask is
  document-sized, so its thumbnail exists and mask painting/editing works.
- **#894** — adjustment rows show their adjustment type icon; Solid Color,
  Gradient, and Pattern fill rows show a content preview (solid swatch,
  gradient ramp, scaled pattern tile).
- **#895a** — split `Add Mask by Default` (Adjustments panel, adjustments) from
  `Use Default Masks on Fill Layers` (Layers Panel Options, fills). With the
  preference on, a new adjustment or fill gets a white document-sized mask when
  no selection exists, and a created adjustment makes its mask the active
  target.
- **#720** — rebuild the row context menu in CS6 block order, add the missing
  row-menu commands, and give the Background a brief dedicated menu.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/layers-panel`: layer operations (anchored adjustment/fill creation),
  Panel Options (split mask preferences and default white mask), Panel and row
  menus (block order, new rows, Background brief menu), and the thumbnail
  layout (adjustment icon and fill preview).
- `compositing/layer-masks`: group masks are sized to the document rectangle.
- `ui/adjustments-panel`: `Add Mask by Default` becomes a working checkable
  preference.
- `ui/application-shell`: the transparency checkerboard is the only canvas
  backdrop; a hidden Background reveals it.

## Impact

- Engine: `crates/pictura-render/src/document_ops/layer_ops/layer_masks.rs`,
  `create.rs`, `crates/pictura-app/src/cxxqt_object/impl_filters.rs`,
  `impl_layers_rasterize.rs`, `impl_layers.rs`, `helpers_composite.rs`.
- Panel UI: `crates/pictura-app/cpp/panels/layers_panel_menu.cpp`,
  `layers_panel_events.cpp`, `layers_panel.cpp`, `layers_panel_model.h`,
  `layers_panel_internal.h`, `panel_group_menu.cpp`, `layers_panel_actions.cpp`.
- Canvas: `crates/pictura-app/cpp/image_view.{h,cpp}`, `tools.cpp`.
- Tests: `tst_command_tree`, `tst_layers_panel`, `tst_layers_panel_fixes`,
  `tst_layers_snapshot`, `tst_layers_mask_commands`, plus a new row-menu suite
  and canvas-backdrop coverage. Two existing test files are at the 1400-line
  test cap, so row-menu cases move to a new `tst_layers_row_menu.cpp`.
- Docs: `docs/02-ui-ux/panels/layers-panel.md`,
  `docs/05-layers/layer-management-ui.md` (Copy CSS is CC-only), and
  `ROADMAP.md`.
- Related: #749 closes as covered by the brief Background menu; #262/#599
  retain the pattern preset library and `Edit > Define Pattern`.
