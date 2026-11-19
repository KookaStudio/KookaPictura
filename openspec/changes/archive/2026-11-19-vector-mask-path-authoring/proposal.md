# Proposal: vector-mask-path-authoring

## Why

Issue #261 (deferred from the Layers-panel parity program, #244 and #110)
records three gaps in vector-mask authoring:

- `Layer ▸ Vector Mask ▸ Current Path` reads only the document Work Path, so a
  saved path cannot become a vector mask.
- A vector mask on a pixel, group, fill, or adjustment layer has no Paths panel
  row, so its outline cannot be selected or edited with the path tools. Only a
  shape layer's outline can be edited.
- The Layers panel vector-mask thumbnail fills the mask's coverage in black and
  white. CS6 shows the revealed area white, the hidden area grey, and the path
  stroked over it.

`docs/02-ui-ux/panels/paths-panel.md` already specifies the vector-mask row
(anatomy item C, `LAY-005`) as a second entry point for editing the mask path.

## What Changes

- **Current Path** authors the path selected in the Paths panel (a saved path or
  the Work Path). The command is unavailable when no non-empty path is selected.
  `pictura_render::VectorMaskKind::CurrentPath` becomes `Path(&[Subpath])`; the
  app resolves the selected path.
- **Vector-mask row.** While one non-shape layer with a vector mask is selected,
  the Paths panel lists `<layer name> Vector Mask`. While that row is active
  (`ActivePath::VectorMask`), Pen, Freeform Pen, Path Selection, Direct
  Selection, Add/Delete Anchor, Convert Point, the path-mode shape tools, and
  Fill/Stroke/Load Path all act on the layer's vector mask. Every edit rewrites
  the mask block, recomposites, and records one undoable history state. Delete
  Path on the row deletes the vector mask.
- **Thumbnail click.** Clicking a non-shape layer's vector-mask thumbnail to make
  it active also activates that layer's vector-mask row; making it inactive
  deactivates the row.
- **Mask flags kept.** `set_layer_shape_paths` keeps the existing invert,
  disable, and unlink flags when it rewrites an outline. Before this change, an
  edit cleared them.
- **Thumbnail.** The vector-mask thumbnail shows the revealed area white and the
  hidden area grey, and strokes the path outline in a dark 1 px line.

## Capabilities

### New Capabilities

- `ui/vector-mask-path-authoring`: Current Path from the selected path, the
  Paths panel vector-mask row and its editing, and the outlined thumbnail.

### Modified Capabilities

<!-- None: `ui/layers-mask-commands` and `ui/properties-panel-vector-masks` keep
     their contracts; this change adds the authoring surface they leave open. -->

## Impact

- `crates/pictura-render` (`layer_ops/vector_masks.rs`, `layer_ops/shape_layer.rs`).
- `crates/pictura-core` (`path.rs`: an editing-subpath setter).
- `crates/pictura-app` Rust bridge (`paths/imp.rs`, `path_list/imp.rs`,
  `impl_layers/vector_masks/imp.rs`, `helpers_masks.rs`, `state.rs`,
  `document_session/session.rs`).
- `crates/pictura-app/cpp` (`panels/paths_panel.*`,
  `panels/layers_panel_events.cpp`, `menu_layer_ops.cpp`), plus the
  `tst_paths_panel` and `tst_layers_mask_commands` suites.
- No new dependency.

## Provenance

The behaviour follows the public CS6 Help text already recorded in
`docs/02-ui-ux/panels/paths-panel.md` ("Paths panel overview": the vector-mask
path, anatomy C; "Manage paths"). The thumbnail's grey value and stroke weight
are approximated from CS6 screenshots; no oracle exists for them.
