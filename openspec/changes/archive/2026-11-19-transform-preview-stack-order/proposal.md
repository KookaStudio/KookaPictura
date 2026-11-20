# Transform preview keeps the layer stack order

## Why

Free Transform and the Move tool's controls box draw the transformed layer over
the live canvas. When the target sits below other layers, resizing it over them
hides those layers for the length of the gesture, so the preview disagrees with
the committed document (#1445). The Move tool already draws the layers above its
moving group back over the moving image; the transform preview does not.

## What Changes

- Build the layers above a transform's hidden set when a transform session
  begins, and draw them back over the transformed image, clipped to its
  footprint, so a lower layer stays below the layers above it while it is
  resized, rotated, skewed, distorted, or warped.
- Reuse the existing above-layer image and bridge fields, so the transform
  preview and the Move preview share one stacking-order seam.
- A transform of the topmost layer builds no above-layer image.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `document/free-transform`: the live transform preview keeps the document's
  layer stacking order.

## Impact

- `crates/pictura-app/src/cxxqt_object/impl_transform/session.rs` and its
  preview module: build the above-layer image at session start.
- `crates/pictura-app/cpp/image_view*` and `tool_transform.cpp`: carry and paint
  the above-layer image for a transform.
- `crates/pictura-app/cpp/tests/tst_transform_controls.cpp`: regression checks
  for a lower layer under an upper layer and for the topmost layer.
- No dependency, document-model, or commit-path change.
