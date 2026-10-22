# Draw the layer under a Free Transform rotate or scale

## Why

While rotating or scaling with Free Transform, Scale, or Rotate, the box and
its handles turned but the layer stayed where it was until the commit (#734).
Since #747 the canvas draws the moving layer through an image-local matrix,
and that matrix was only produced for the projective modes. A similarity
session got none, and the canvas fell back to identity.

## What Changes

- `transform_preview_local_matrix_extents` also returns the similarity map, in
  the moving image's own coordinates, so the canvas draws the layer, a shape
  included, under the live rotation, scale, and translation.

## Capabilities

- Added: `document/free-transform-preview`

## Impact

- Code: `pictura-app` `cxxqt_object/impl_transform/bridge.rs`; a test accessor
  on `ImageView`; Qt Test `tests/tst_shape_transform.cpp`.
