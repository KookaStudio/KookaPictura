# Transform preview anti-aliases resampled edges

## Why

A live Free Transform preview resamples the moving layer in the canvas, but its
CPU warp clamps bilinear taps at the source rectangle's edge. A solid pixel layer
therefore keeps an alpha-255 boundary while it is resized, so the edge looks
hard and pixelated during the gesture; the committed resample pads the same edge
with transparent samples and comes out anti-aliased (#1445 report follow-up).

## What Changes

- Sample the preview warp inside the transform's source rectangle only, treating
  taps outside it as transparent, matching the engine's resample padding.
- Let a destination pixel on the transformed edge sample just outside the source
  rectangle so it can receive partial coverage.
- A resized solid layer's live edge SHALL match the committed edge's
  anti-aliasing.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `document/free-transform`: the live transform preview anti-aliases resampled
  layer edges.

## Impact

- `crates/pictura-app/cpp/image_view_transform_warp.cpp`: the preview's bilinear
  sampler and source bounds.
- `crates/pictura-app/cpp/tests/tst_transform_controls.cpp`: a regression check
  comparing the live edge with the committed edge.
- No document model, commit path, or dependency change.
