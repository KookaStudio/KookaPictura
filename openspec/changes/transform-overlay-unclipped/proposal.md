# Proposal: transform-overlay-unclipped

## Why

Issue #308: dragging a layer in Free Transform (or a placed object's session)
past the canvas edge crops or hides the layer's bounding box and handles,
because the overlay is painted inside the document-rect clip. CS6 keeps the full
transform box and handles visible however far the layer extends past the
document edge; only the rasterised result is clipped to the canvas.

## What Changes

- The Free Transform overlay — the bounding quad, its eight handles, the rotate
  knob, and the reference point — SHALL be painted without clipping to the
  document rectangle, so an off-canvas corner keeps its handle.
- Only the rasterised preview content SHALL stay clipped to the canvas.

## Capabilities

### Modified Capabilities

- `document/free-transform`: a new requirement fixes the overlay's clipping:
  full box outside the document edge, preview content clipped.

## Impact

- `crates/pictura-app/cpp/image_view.cpp` (paint order),
  `image_view_overlays.cpp` (`paintTransformOverlay`), `image_view.h`; new
  `tst_transform_overlay` Qt Test. No new dependency.
- The Move tool's controls box shares the overlay path, so it is covered too.
