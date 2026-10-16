# Proposal: layer-row-drop-bands

## Why

Issue #232: layers could not be reordered by dragging. The tree treated only the
2 px at a row's top or bottom edge as above/below. Everything else was a drop
*into* the row, and the engine refuses that for anything but a group. A real
drag released anywhere inside a layer row was therefore refused, so reordering
only worked at a pixel-exact edge. The synthesized-drop checks always dropped
1 px from an edge, so they never saw it.

## What Changes

- Layers panel: a non-group row is a sibling target across its whole height. The
  upper half drops above it and the lower half below it.
- A group row keeps a centre drop-into band. Its upper and lower quarters (at
  least 2 px) drop above and below it.

## Capabilities

### Modified Capabilities

- `ui/layers-panel`: layer row drop bands.

## Impact

- `pictura-app` (`panels/layers_panel_internal.h`, the tree's drop resolver).
  No engine change, no new dependency.
