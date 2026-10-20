# Distort preview warp

## Why

At some Distort angles the live preview loses most of the shape, leaving a
triangle, while Photoshop and the committed result show the full shape (#748
follow-up). The preview draws the layer image under a projective `QTransform`;
Qt's raster engine can drop one of the two triangles it splits the image into.
Our raster, mask, vector-mask, and clipped-fill tests with the reported quad all
render fully through the committed engine, so the preview's Qt projective draw
is the remaining difference.

## What Changes

- Render the live projective preview by inverse-mapping the visible destination
  region through the projective matrix and sampling the preview layer image on
  the CPU, instead of drawing it under a projective `QTransform`.
- Keep the Qt projective draw as a fallback when the warp has nothing to draw.
- Keep the front-side-of-the-horizon rule in the warp.
- Add regression tests using the exact reported quad on raster, layer-mask,
  vector-mask, and clipped-fill layers, plus a direct warp test for the horizon.

## Impact

- Capability: `document/free-transform` (ADDED requirement).
- Code: `crates/pictura-app/cpp/image_view.{h,cpp}`,
  `crates/pictura-app/cpp/image_view_transform_warp.cpp`, `CMakeLists.txt`.
- Tests: `tst_transform_distort`.
