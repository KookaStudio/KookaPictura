# Design: move-auto-select-and-refine-enablement

## Context

The Move tool's options bar (`options_bar_move.cpp`) builds an
`optionsMoveAutoSelect` combo (Group/Layer) whose `MoveOptions::autoSelect`
value is otherwise unused; there is no checkbox. The Move handler
(`tool_move.cpp`) presses on the active layer only. The selection tools' pages
share `OptionsBar::addRefineEdgeButton`, which always stays enabled, while the
frame's `Select ▸ Refine Edge…` handler checks `refine_edge_available` (a
document with a non-empty selection) and returns silently when unavailable.

## Decisions

### Auto-Select state

`MoveOptions` (the Move handler's `ToolOption<MoveOptions>` group, read and
written by the options bar) gains `autoSelectOn` (default off) alongside the
existing `autoSelect` Group/Layer choice. The bar writes the group; the handler
reads it on press.

### Hit test

`auto_select_layer(view, x, y, group)` (a new bridge in
`impl_layers/auto_select.rs`, a free function so the `PictureView` declaration
list does not grow) walks the document's layer tree topmost-first:

- a hidden or zero-opacity layer (and its subtree) is skipped;
- a group recurses into its children and, on a hit, returns the group's own
  path when `group` is set;
- an adjustment layer is skipped (it has no pixels);
- a pixel layer is hit when the document point falls inside its rect and the
  layer's own alpha at that point is non-zero (`layer_pixel`).

The walk lives in `helpers_composite::auto_select_path`, a free function over a
`Document`, so it is unit-testable without the QObject. Ceilings: masks,
clipping, effects, and live type/shape rendering are ignored — the layer's own
channel alpha decides.

### Selecting the hit layer

`ToolServices` gains `selectLayer(path)`, implemented by `ToolController` as a
`layerSelectionRequested` signal that the frame connects to the existing
`PicturaMainWindow::selectLayerPath` (panel refresh + row selection, which sets
the view's active layer). This keeps the panel and the engine in sync instead of
calling `set_active_layer` behind the panel's back.

### Menu button icon

A new `assets/icons/ellipsis.svg` (Lucide `ellipsis`, vendored style, mapped in
`lucide-map.json`) replaces the `…` text; the button becomes icon-only.

### Refine Edge enablement

`OptionsBar` keeps the refine buttons it built and exposes
`setRefineEdgeEnabled(bool)`. The frame calls it from `updateAlignControls()` —
reached from `refresh()` (a `PictureView::changed`, so selection commits and
Select menu commands, plus document switches) and the Layers panel's selection
changes — with `view && refine_edge_available(*view)`, the same predicate the
menu command's enabled provider uses.
