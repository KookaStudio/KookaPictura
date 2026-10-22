# Antialias vector-mask and shape edges

## Why

A shape layer's edges were stair-stepped: any slanted or curved edge, and every
edge after a rotate, skew, or distort, came out as hard pixels (#734). The
compositor sampled a vector mask with hard 0/255 coverage at each pixel's
centre, a known shortcut (`ponytail:` in `vector_mask.rs`). A shape is a path;
it should render smooth.

## What Changes

- `VectorMask::coverage` gives a pixel's antialiased coverage: the share of the
  pixel the path covers, measured along 16 sample lines per pixel row, with
  the covered length exact along each line. It is cached per pixel row as a
  step function, bounded to the clip like the existing row table.
- The compositor's vector-mask coverage (`vector_mask::coverage`) uses it, so
  shape layers, vector-masked layers, their thumbnails, the transform preview,
  and Rasterize Vector Mask all get smooth edges. The pixel-centre
  `VectorMask::inside` test is unchanged for its other callers.

## Capabilities

- Added: `compositing/vector-mask-antialiasing`

## Impact

- Code: `pictura-core` `vector.rs`; `pictura-render` `vector_mask.rs`; Qt Test
  `tests/tst_shape_transform.cpp`.
- Cost: the 4-shape composite profile goes from about 54 ms to 72 ms.
