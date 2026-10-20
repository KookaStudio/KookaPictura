# Keep the box's projective frame in Warp mode

## Why

Review of the photocraft Warp port (#751) found defects that change image
results or can end the app:

- Entering Warp after Distort or Perspective changed the image before any edit.
  The box was baked into one Bezier patch, which reproduces an affine map but
  not a homography.
- A press on a preset's hidden handle, off the warped surface, grabbed it and
  turned the preset into a custom mesh.
- A layer mask that reached past the layer collapsed onto the box edge.
  Triangulating the mask then failed, and the pixels moved while the old mask
  stayed in place.
- A mesh inside the coordinate limit could still ask for a result of billions
  of pixels.
- The handles of a dragged anchor stayed shown after release.
- The fold test did not check which cell draws on top.

## What Changes

- The warp mesh stays in the source box. The box's quad is kept as a projective
  frame and applied after the warp on display, hit testing, drags, preview and
  commit, as Photoshop keeps `Trnf` apart from `warp`.
  `transform_layer_warp` takes that quad.
- Only a custom warp's points can be grabbed. A press on a preset has to land
  on its surface.
- Mask points outside the warp box move with the nearest box point. The op
  refuses when a mask with area cannot follow the pixels.
- The warp's result rect has the same pixel-area ceiling as the other transform
  ops.
- Releasing a drag refreshes the canvas overlay.

## Capabilities

- Modified: `document/transform-session-warp`, `document/transform-warp-engine`.

## Impact

- Code: `pictura-render` `layer_ops/{warp,warp_raster,warp_tests}.rs`;
  `pictura-app` `impl_transform/{warp,session,distort}.rs`, `state.rs`; C++
  `tools.cpp`, `tests/tst_transform_warp.cpp`.
