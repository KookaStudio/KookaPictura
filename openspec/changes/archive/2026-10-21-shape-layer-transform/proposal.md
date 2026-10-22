# Move and transform shape layers as vectors

## Why

A shape layer could not be moved with the Move tool or changed with any
`Edit > Transform` command (#734). A shape layer is a fill layer cut to a
vector mask, and the Move and transform paths only accepted raster layers, so
they refused it. The Layers panel also drew a second, vector-mask thumbnail
beside the shape's own, which made the new layer look as if it carried a mask;
CS6 shows a shape layer as one thumbnail with the shape badge.

## What Changes

- The Move tool drags a shape layer: the outline moves, a live shape stays
  live, and the move records one `Move Layer` state.
- Free Transform and the `Edit > Transform` commands (Scale, Rotate, Skew,
  Distort, Perspective, Warp, the exact rotations and flips, Again) accept a
  shape layer. They map its outline's anchors and handles instead of
  resampling pixels, so the layer stays a vector shape layer. The box hugs the
  outline, not the canvas-sized fill.
- The Move tool's transform controls box hugs a shape's outline.
- A shape layer's Layers row has no vector-mask thumbnail: its outline is
  already its one thumbnail.

## Capabilities

- Added: `tools/shape-layer-transform`
- Modified: `ui/layers-shape-badge` (an added requirement)

## Impact

- Code: `pictura-render` `document_ops/crop.rs`,
  `document_ops/layer_ops/{shape_layer,transform,warp,move_content}.rs` and
  the re-exports; `pictura-app` `cxxqt_object/impl_layers.rs`,
  `cxxqt_object/impl_transform/{mod,session}.rs`; Qt Test
  `tests/tst_shape_transform.cpp` and the test CMake list.
