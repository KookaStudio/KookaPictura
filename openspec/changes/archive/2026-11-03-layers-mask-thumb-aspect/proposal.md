# Proposal

## Why

On a non-square document a pixel layer's raster mask thumbnail is sized from the
mask's own rect instead of the document, so it does not share the aspect ratio
that the layer, shape, and vector-mask thumbnails use. On the reference PSD the
affected layer's mask is wider than the canvas, which yields a 24x4 mask
thumbnail inside a 24x7 document-aspect card, while its layer and vector-mask
thumbnails are 24x7. The issue's layer-image claim does not reproduce:
`layer_thumbnail_positioned` already emits the document aspect. The remaining
defect is the raster mask thumbnail.

## What Changes

- Entire Document mask thumbnails carry the document aspect and place the mask
  rect at its document-relative position, clipped to the document bounds, with
  the area outside the mask rect left transparent so the checkerboard shows.
  Layer Bounds keeps sizing the mask image from the mask rect; its card stays
  document-aspect, so the uncovered bands show the checkerboard.
- The delegate draws the checkerboard under the mask thumbnail card, matching the
  image thumbnail.
- The raster and vector mask thumbnail helpers move from `helpers_composite.rs`
  to a new `helpers_masks.rs` so the code-size cap holds after the new function.
- Add regression tests for the sub-canvas raster layer image thumbnail (already
  correct; pins the behavior) and the layer-sized mask thumbnail (the fix),
  including a mask wider than the canvas.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/layers-panel`: the "Layer thumbnail preserves the canvas aspect ratio"
  requirement gains a raster-mask clause (document aspect and document-space
  placement clipped to the canvas in Entire Document, mask-rect image sizing in
  Layer Bounds) and scenarios for a sub-canvas raster layer and a layer-sized
  mask.

## Impact

- `crates/pictura-app/src/cxxqt_object/helpers_masks.rs` (new; moved mask
  thumbnail helpers, the positioned mask thumbnail, and its unit test).
- `crates/pictura-app/src/cxxqt_object/helpers_composite.rs` (move out; the
  inline `bundle_tests` import the moved function).
- `crates/pictura-app/src/cxxqt_object.rs` (declare `mod helpers_masks;`; the
  file is 1202/1204 in the size allowlist, so this is the only line it gains).
- `crates/pictura-app/src/cxxqt_object/impl_layers.rs` (choose positioned vs
  bounds mask thumbnail from `entire_document`).
- `crates/pictura-app/cpp/panels/layers_panel_internal.h` (checker under the mask
  card).
- `crates/pictura-app/cpp/tests/tst_layers_snapshot.cpp` and
  `tst_layers_panel_fixes.cpp` (regression tests; the latter has 44 lines of
  headroom against the 1400-line test cap).
- No document, codec, compositing, or dependency changes.
