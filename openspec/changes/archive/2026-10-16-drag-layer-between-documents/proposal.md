# Proposal: drag-layer-between-documents

## Why

Issue #231: a layer could not be dragged from one document into another. CS6
duplicates a layer into another document when it is dragged there
(`docs/05-layers/layer-management-ui.md`: Duplicate "optionally into another
… document"). Copy/paste carries pixels only and loses the layer's properties.

## What Changes

- Engine: `copy_path_to_document` deep-copies a layer node (children, masks,
  effects, attributes) from one document into another. It uses the New Layer
  insertion rule, keeps the name and position, and releases a Background copy.
  A document of another color mode or bit depth is refused.
- Bridge: `copy_layer_from_document` records one "Duplicate Layer" state.
- Shell: a Layers-panel drag carries its source document. Hovering another
  document's tab brings it forward, and a drop on that tab or its canvas copies
  the dragged layer in and selects the copy. The panel ignores a drag from
  another document, so after the switch its rows and strip buttons never act on
  the destination's layer at the same path.
- Move tool: a canvas drag that reaches another document's tab becomes the
  same layer drag; the source layer keeps its position.

## Capabilities

### New Capabilities

- `ui/layers-panel`: drag a layer into another document.

## Impact

- `pictura-render` (`layer_ops/transfer.rs`), `pictura-app` (clipboard bridge,
  `FileDropRouter`, frame, layers panel). No new dependency.
- ponytail: CS6 converts the copy into the destination's mode and depth; a
  mismatched pair is refused for now.
