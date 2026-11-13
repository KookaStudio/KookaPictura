# Design

## Context

See `proposal.md - Why`. The document-aspect thumbnails currently have no
bound:

- `aspect_thumbnail_extent(width, height, size)` in
  `crates/pictura-app/src/cxxqt_object/helpers_composite.rs` sizes the emitted
  RGBA image. Three producers call it: `layer_thumbnail_positioned`,
  `shape_thumbnail`, and `mask_thumbnail_positioned`. All three already apply
  independent x and y scales when mapping document coordinates into the output.
  `vector_mask_thumbnail_image` in `helpers_masks.rs` had its own inline extent
  (it floor-divided the short edge) and becomes the fourth caller of the shared
  rounded rule; an in-range 16:9 vector image is 24x14 instead of 24x13.
- `LayerRowDelegate::letterboxedThumb(box, index)` in
  `crates/pictura-app/cpp/panels/layers_panel_internal.h` computes the drawn
  card. The image, raster-mask, and vector-mask cards all call it through
  `thumbnailCard`, `maskThumbRect`, and `vectorMaskThumbRect`.

Layer Bounds content thumbnails come from `layer_thumbnail_image`, which sizes
from the layer bounds rather than the document and is deliberately unchanged.
The Rust extent uses `size` on the long edge, while the delegate letterboxes the
document aspect into the row's thumbnail slot (24x32 at the default 24 px
thumbnail: `rowHeight()` is 36 and the slot keeps a 2 px vertical pad). The two
therefore agree on the aspect but differ in pixel size: an 8x24 image is drawn
into an 11x32 card for a 1:3 document. `drawImage(card, image)` in the delegate
stretches the image into the card because Qt's `drawImage(QRect, QImage)` ignores
the source aspect.

## Goals / Non-Goals

**Goals:**

- One clamp rule, no more extreme than 3:1 or 1:3, applied on the Rust and C++
  sides so the image and card carry the same clamped aspect, with whole-pixel
  rounding.
- Content stretched, not clipped: each axis scales independently, exactly as the
  producers already do.
- Thumbnails whose rounded extent already meets the bound render pixel-identically,
  except vector-mask images, which now use the shared round instead of floor (at
  most 1 px in range).

**Non-Goals:**

- Changing row height, slot width, hit-target math, or the left-to-right
  thumbnail walk.
- Clamping the square cards (type, adjustment icon, fill preview) or the group
  folder glyph.
- Changing Layer Bounds image sizing; only its card inherits the clamp, as
  today it inherits the document aspect.

## Decisions

### Round the extent, then raise the short edge to the bound

Both sides first compute the existing proportional extent: `size` on the long
edge and the rounded proportional length on the short edge. If the rounded short
edge is below one third of the long edge, it is raised to
`long_edge.div_ceil(3)` (Rust) or `(long_edge + 2) / 3` (delegate). This keeps
the existing round for every layer, shape, and raster-mask thumbnail that
already meets the bound, fixes in-range documents whose rounding would fall just
outside it (300x100 at 16 px gives 16x6, not 16x5), and covers documents outside
the range without a separate clamped-dimensions branch. For a 10:1 document the long edge stays at
`size` (24x8), rather than shrinking the extent to `3 * short x short`.

The vector-mask producer previously floor-divided its short edge; it now uses
the shared rounded extent, so an in-range 16:9 document's vector image is 24x14
instead of 24x13 (at most 1 px).

### Clamp every document-aspect producer, not only the card

Clamping only `letterboxedThumb` would leave the emitted image at the true
document aspect and rely on paint-time stretching; for a 10:1 document the image
is only 2-3 px tall, so the delegate would magnify the strip into an 8 px card.
Clamping `aspect_thumbnail_extent` too samples the content directly at the card
resolution. The same argument covers `vector_mask_thumbnail_image`, whose inline
extent is folded into `aspect_thumbnail_extent`, so every document-aspect
thumbnail image carries the clamp. Layer Bounds content images are the
deliberate exception: they keep their layer-bounds sizing and have always been
stretched into their card.

### Keep the clamp constants aligned, not shared

The 3:1 bound lives once per language (`aspect_thumbnail_extent` for Rust,
`letterboxedThumb` for C++): after the vector-mask switch, the Rust function is
the single funnel for every document-aspect thumbnail image, and the delegate is
the single funnel for the cards. A cross-language constant would add FFI surface
for one integer. Both round the proportional extent first and then raise a short
edge below one third of the long edge, so the image and card carry the same
clamped aspect and differ only by whole-pixel rounding because the card's long
edge is the slot, not `size`.

### Leave square cards alone

`thumbnailCard` short-circuits type, adjustment-icon, and synthetic rows to a
1:1 card before calling `letterboxedThumb`; group glyphs bypass the card walk.
Those paths are untouched, so icons and fill previews keep their square
presentation even on an extreme-aspect document.

## Risks / Trade-offs

- **Tests pinned to documents beyond 3:1** → update `maskThumbnailKeepsDocumentAspect`
  (400x100: content image, mask image, and both cards at 24x8; the reveal-all mask
  is white at (3,2)) and keep `allMaskCardsFollowDocumentAspect` on 400x120 /
  120x400 but compare each card against the clamped effective canvas. Add
  extreme-aspect regressions for the layer, shape, mask, and vector-mask Rust
  producers and the Qt delegate, plus boundary regressions (300x100 at 16 px
  gives 16x6) and a 16:9 vector-mask rounding regression (1600x900 at 24 px
  gives 24x14).
- **Deliberate distortion on extreme documents** → the spec now says content is
  stretched into the clamped card; the checkerboard, outline, brackets, and hit
  target all follow the same card, so cursor and paint agree.
- **Rounding drift between languages** → both formulas round the proportional
  extent first and then raise a short edge below one third of the long edge.
  The vector-mask producer now shares that round (it previously floor-divided),
  a change of at most 1 px in range. The Rust tests assert the image extent and
  the Qt Test asserts the card separately: for 1:3 at 24 px the image is 8x24
  and the card 11x32.
