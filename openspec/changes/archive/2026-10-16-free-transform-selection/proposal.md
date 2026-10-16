# Proposal: free-transform-selection

## Why

Issue #188: Ctrl+T and `Edit > Free Transform` did nothing on an opened photo.
Opened JPEG/PNG files, and File > New documents since #200, have a locked
Background layer, and Free Transform refused a Background even with a selection.
CS6 transforms the **selected pixels** when a selection is active, on any pixel
layer including the Background
(`docs/08-selection/transform-selection.md`, "Interaction with Free
Transform"). Without a selection it still refuses the Background, as CS6 does.

## What Changes

- `pictura_render::lift_selection(doc, path, coverage)`: copies the covered
  pixels into a floating layer directly above the source, built at the
  selection bounds so memory follows the selection, and clears them from the source as Edit > Clear does (a
  Background clears to white).
- `pictura_render::merge_lifted(doc, path)`: composites the floating pixels
  back over the source's own channels and removes the floating layer, so the
  source keeps every other attribute (name, locks, Background flag, opacity,
  blend, mask, effects). A general Merge Down would bake opacity and blend and
  drop the mask.
- `pictura_render::can_lift_selection(layer)`: the shared refusal predicate.
- **Free Transform session:** with a non-empty selection on a liftable layer,
  beginning a session lifts the pixels and transforms the floating layer. A
  commit merges it back as one "Free Transform" state and drops the selection.
  A cancel or an identity commit restores the document exactly.
  `layer_can_free_transform` reports a liftable selection as transformable, so
  `Edit > Free Transform` and Skew/Distort/Perspective enable.

## Capabilities

### Modified Capabilities

- `document/free-transform`: adds selection-scoped Free Transform, and the
  Skew/Distort/Perspective enablement now counts it.

## Impact

- `pictura-render` (layer_ops `move_content.rs`) and `pictura-app` (transform
  session). No new dependency.
- ponytail: CS6 carries the selection border through the transform. Here it is
  dropped on commit. The hole in a Background is white (the existing
  Edit > Clear ceiling) rather than the background swatch.
