# Proposal

## Why

`Layer ▸ Duplicate Layer…` silently does nothing when the active layer is nested
inside a group, and it duplicates only one layer when several rows are selected
(#742). The handler calls the index-based `duplicate_layer` with
`LayersPanel::currentLayer()`, which returns `-1` for any nested path; the
command stays enabled, so the click looks like it worked. The Layers panel's row
menu and flyout already duplicate the whole selection through the path-based
`duplicate_layers`.

## What Changes

- Route the Layer menu command through the panel's selection-aware
  `duplicateSelection`, the same path the panel's row menu and flyout use.
- Enable `Layer ▸ Duplicate Layer…` only when the active document has a
  non-empty Layers panel selection.
- Add Qt Test coverage for a nested layer, a two-layer selection, and the empty
  selection.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/layers-panel`: the Layer menu duplicates the panel selection by path,
  including nested layers and multi-selections, in one undoable step.

## Impact

- `crates/pictura-app/cpp/frame_menus.cpp` — handler and enablement provider.
- `crates/pictura-app/cpp/panels/layers_panel.h` — expose the existing
  `duplicateSelection` seam next to `groupSelection`/`deleteSelection`.
- `crates/pictura-app/cpp/tests/tst_layer_commands.cpp` — a new Qt Test suite
  with one case per scenario, registered in the tests `CMakeLists.txt`.
- No engine, bridge, dependency, or data-format changes.
- No `design.md`: the selection-aware operation already exists; this is a
  one-call handler reroute.
