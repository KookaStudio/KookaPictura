# Proposal

## Why

Layer row thumbnails scale with the document's aspect ratio without an upper
bound, so a document around 10:1 or 1:10 draws as a very thin strip in the
Layers panel (issue #1287). The letters are unreadable and the row's thumbnail
slot is mostly empty.

## What Changes

- Clamp the drawn thumbnail card aspect so it is no more extreme than 3:1 or
  1:3: the proportional short edge is rounded first and raised when it falls
  below one third of the long edge, and thumbnails whose rounded extent already
  meets the bound keep rendering as today.
- Clamp the emitted thumbnail images to the same range and stretch their
  content to fill it: Entire Document content, shape, raster-mask, and
  vector-mask images are scaled independently on each axis into the clamped
  aspect, so nothing is clipped or truncated and no transparent strip is left
  to be stretched at paint time. Layer Bounds content images keep their
  layer-bounds sizing; only their card follows the clamp.
- Share one clamp rule between the Rust thumbnail producers and the Qt row
  delegate. The emitted image uses `size` on its long edge; the card letterboxes
  the clamped aspect into the row's slot, so the two carry the same clamped
  aspect and differ only by whole-pixel rounding (for a 1:3 document at the
  default 24 px thumbnail the image is 8x24 and the card 11x32).
- Row layout, hit-target math, group glyphs, type cards, adjustment icons, and
  fill previews are unchanged; the `thumbRect`, mask, and vector-mask hit rects
  are the cards and therefore follow the clamped card. In-range layer, shape,
  and raster-mask thumbnails whose rounded extent already meets the bound render
  as before; vector-mask images, which previously floor-divided the short edge,
  now use the shared rounded extent, a change of at most 1 px in range.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `ui/layers-panel`: the "Layer thumbnail preserves the canvas aspect ratio"
  requirement changes from "drawn at the document aspect, never distorted" to
  "drawn at the document aspect, clamped so it is no more extreme than 3:1 /
  1:3, with the document content stretched to fill the clamped card". The
  image-aspect clauses for regular, shape, raster-mask, and vector-mask
  thumbnails gain the same clamp.

## Impact

- `crates/pictura-app/src/cxxqt_object/helpers_composite.rs` —
  `aspect_thumbnail_extent` (shared by the positioned layer, shape, and raster
  mask producers); `layer_thumbnail_image` keeps its Layer Bounds sizing.
- `crates/pictura-app/src/cxxqt_object/helpers_masks.rs` —
  `vector_mask_thumbnail_image` switches from its inline extent to
  `aspect_thumbnail_extent`; `mask_thumbnail_positioned` inherits the clamp.
- `crates/pictura-app/src/cxxqt_object/shapes/imp.rs` — `shape_thumbnail`
  inherits the clamp.
- `crates/pictura-app/cpp/panels/layers_panel_internal.h` —
  `LayerRowDelegate::letterboxedThumb` (image, mask, and vector-mask cards).
- Tests that use documents beyond 3:1 and assert the old aspect:
  `crates/pictura-app/cpp/tests/tst_layers_panel_fixes.cpp`,
  `crates/pictura-app/cpp/tests/tst_canvas_mask_domain.cpp`, plus new
  extreme-aspect regressions for the layer, shape, mask, vector-mask, and Qt
  Test layers.
- No document, codec, or compositing behavior changes; no new dependencies.
