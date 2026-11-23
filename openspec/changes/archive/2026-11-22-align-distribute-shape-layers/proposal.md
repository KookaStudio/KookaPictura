# Proposal: align-distribute-shape-layers

## Why

Issue #1531: Layer ▸ Align, Align Layers To Selection, Distribute, and the Move
tool's matching options-bar buttons are disabled when the selected layers are
shape layers. `layer_ops/align.rs` treats every layer with adjustment content as
contentless, and a shape layer carries its fill as adjustment content, so the
commands find no layers to line up. CS6 aligns and distributes shape layers by
their vector outline.

## What Changes

- `pictura-render` `layer_ops/align.rs`: a shape layer counts as content by its
  outline box (`shape_layer_rect`) instead of being skipped, and moving one
  translates the outline (`map_shape_layer`) instead of offsetting its
  canvas-sized fill rectangle.
- `tst_align_distribute` gains a shape-layer case: the menu commands and Move
  tool buttons enable for shape layers and align/distribute their outlines.
- `align` unit tests cover shape content bounds, enablement, and movement.

## Capabilities

### Modified Capabilities

- `document/align-distribute`: shape layers participate in Align and Distribute
  by their vector outline bounds, and the commands translate the outline.

## Impact

- `pictura-render` (`layer_ops/align.rs`) and `pictura-app` tests. No new
  dependency, no codec or document-format change.

## Provenance

Shape bounds and translation reuse the existing `shape_layer_rect` and
`map_shape_layer`; the CS6 behavior is recorded in
`docs/05-layers/align-and-distribute.md`.
