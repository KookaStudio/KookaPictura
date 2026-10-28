# Proposal: smart-filter-reorder-drag

## Why

Review of #984 (PR #1005): CS6 reorders Smart Filters by dragging their rows in
the Layers panel, and the higher row's effect applies over the lower ones. The
Smart Filter rows were not draggable; reorder existed only as an engine op.

## What Changes

- A Smart Filter row drags up or down within its own Smart Filters list; the
  drop moves the filter in the chain through the existing
  `reorder_layer_smart_filter` bridge (one "Reorder Smart Filters" state).
- A drop anywhere else (a layer row, another object's list, the group row) is
  refused. Rows list the newest filter on top and the chain applies bottom-up,
  so a higher row's effect shows over the lower ones.

## Capabilities

### Modified Capabilities

- `ui/layers-panel`: Smart Filter rows drag within their list.

## Impact

The Layers panel (`layers_panel_model.h`, `layers_panel.cpp`,
`layers_panel_menu.cpp`), `tst_smart_filter_apply`.
