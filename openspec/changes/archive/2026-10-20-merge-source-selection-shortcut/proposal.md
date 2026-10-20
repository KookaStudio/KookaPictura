# Proposal

## Why

Manual testing of the context-sensitive merge command in #744 found three gaps
against Photoshop CS6 / CC:

- An adjustment layer above a pixel layer cannot be merged down. Adobe Help
  allows adjustment and fill layers as merge inputs and requires only the target
  layer to be raster; the engine currently rejects both sides.
- Merge Visible discards the merged node's path. When the pre-merge selection
  disappears, the Layers panel falls back to row 0 and can select a surviving
  hidden layer instead of the merged layer.
- `Ctrl+E` uses the QAction shortcut, whose enabled state is refreshed when a
  menu opens or the document changes, but not when the Layers panel selection
  changes. Selecting the Background first and then extending the selection leaves
  the action disabled until the Layer menu is opened.

## What Changes

- Allow an adjustment or fill-content layer to be a merge source. Keep the merge
  target rule: the lower layer for Merge Down and the bottommost selected layer
  for Merge Layers must be raster.
- Return the merged path from `merge_layers` and `merge_visible`, rebuild the
  panel rows, and select the merged path as the sole selection.
- Refresh the command registry when the Layers panel selection changes, so
  selection-dependent labels and enabled states, including `Ctrl+E`, are live.
- Add engine and Qt coverage for adjustment sources, the remaining target
  refusal, merge-result selection, and shortcut enablement.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/layer-management`: adjustment and fill layers are valid merge
  inputs; only the target remains restricted.
- `ui/layers-panel`: a successful Merge Down, Merge Layers, or Merge Visible
  selects the merged layer and nothing else.
- `document/command-registry`: command labels and enablement follow Layers panel
  selection changes without waiting for a menu open.

## Impact

- `crates/pictura-render/src/document_ops/layer_ops/merge.rs` and
  `merge_tests.rs` (guard split and engine tests)
- `crates/pictura-app/src/cxxqt_object/impl_layers_merge.rs` and
  `cxxqt_object.rs` (merged path return)
- `crates/pictura-app/cpp/frame_menus.cpp`,
  `panels/layers_panel_menu.cpp`, and `frame_build.cpp` (selection and live
  enablement)
- `crates/pictura-app/cpp/selftest_layers_controls.cpp` (adapted return type)
- `crates/pictura-app/cpp/tests/tst_merge_commands.cpp` (new cases)
- New OpenSpec delta and test plan under
  `openspec/changes/merge-source-selection-shortcut/`

No documentation change: `docs/05-layers/merge-and-flatten.md` and
`docs/05-layers/adjustment-layers.md` already state that only the target is
restricted and that source-side merging rasterizes the adjustment.
