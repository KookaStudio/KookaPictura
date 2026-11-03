# Design

## Context

See `proposal.md - Why`. Three producers feed a pixel row's thumbnail:

- `layer_thumbnail_positioned(layer, doc_w, doc_h, size)` in
  `helpers_composite.rs` already emits a document-aspect image and places the
  layer rect at its document offset.
- `mask_thumbnail_image(mask, size)` in `helpers_composite.rs` sizes from
  `mask.rect`, so it keeps the mask's own aspect (a 6395x1257 mask on a
  4934x1500 canvas yields 24x4, not 24x7).
- `vector_mask_thumbnail_image(mask, doc_w, doc_h, size)` already samples the
  document extent.

The delegate computes a document-aspect card with `letterboxedThumb` for every
non-square-card row and draws each thumbnail into it: the image with
`drawImage`, the mask and vector mask with `drawImageFit` (aspect-preserving).
The mask card therefore already has the document aspect; only its image does
not. `helpers_composite.rs` is at 1185 of the 1200-line code cap, so the new
function needs a module split rather than more lines there.

## Goals / Non-Goals

**Goals:**

- A raster mask thumbnail in Entire Document placement carries the document
  aspect and places the mask rect at its document-relative position, clipped to
  the document bounds.
- The empty area outside the mask rect reads as transparency (checkerboard),
  matching the layer image thumbnail.
- Layer Bounds keeps today's mask-rect image sizing; the card stays
  document-aspect, so the uncovered bands read as checkerboard.
- Regression coverage for both the mask (the fix) and the sub-canvas layer image
  (already correct), including a mask wider than the canvas.

**Non-Goals:**

- Changing the delegate's letterbox math, the mask card, the hit targets, or the
  vector-mask thumbnail.
- Fixing the pre-existing Layer Bounds image-thumbnail stretch.
- Any document, codec, compositing, or dependency change.

## Decisions

### Emit the document-aspect mask thumbnail from the bridge

Chosen: add `mask_thumbnail_positioned(mask, doc_w, doc_h, size)` that mirrors
`layer_thumbnail_positioned`: `aspect_thumbnail_extent` for the output size,
independent x/y scales for the mask offset and size, nearest-neighbour sampling
of the mask plane, `[v, v, v, 255]` inside the mask rect and transparent
outside. The destination rect is clipped to `tw`/`th`, so a mask wider than the
canvas (or one with a negative offset) is cropped rather than scaled down.
`impl_layers.rs` calls it when `entire_document` is true and keeps
`mask_thumbnail_image` for Layer Bounds.

Alternative considered: keep the mask-rect image and let the delegate draw it
aspect-preserving. Rejected: the delegate already does that, which is exactly
the current mismatch; it would also leave the image's aspect differing from the
card and from the vector-mask thumbnail.

### Transparent margins with a checkerboard under the mask card

Chosen: the mask image is transparent outside the mask rect and the delegate
draws `checkerTile()` under the mask card before `drawImageFit`, the same
treatment the image card gets. A full-document mask fills the card, so the
checker is invisible there; a sub-canvas mask shows it. In Layer Bounds the mask
image keeps the mask rect aspect and is letterboxed into the document-aspect
card, so its uncovered bands also show the checkerboard. That band behavior is
new and is stated in the spec delta.

Alternative considered: fill the outside with the mask's `default_color`.
Rejected: it hides the fact that the mask does not cover that area and diverges
from how the layer thumbnail shows empty space.

### Split the mask helpers into `helpers_masks.rs`

Move `mask_thumbnail_image` and `vector_mask_thumbnail_image` into a new
`cxxqt_object/helpers_masks.rs` and add the positioned mask function there. This
is a pure move plus the new function; `helpers_composite.rs` drops well under
the cap and the mask concern stays in one file. `impl_layers.rs` imports the new
module; the inline `bundle_tests` in `helpers_composite.rs` import the moved
function where they still use it. `cxxqt_object.rs` is 1202 against its 1204
allowlist ceiling, so it gains only `mod helpers_masks;`.

### Tests at both layers

- The mask unit test lives inline in `helpers_masks.rs` (`#[cfg(test)] mod
  tests`), not in `cxxqt_object/tests.rs`, which is 1399/1400. It pins the
  document aspect, the document offset, the transparent margin, and clipping for
  a mask wider than the canvas with a negative offset, plus the zero-rect and
  missing-data refusals.
- `tst_layers_snapshot::subCanvasLayerPlacement` pins the snapshot contract: a
  sub-canvas raster layer's image and mask thumbnails in Entire Document, and
  the mask thumbnail in Layer Bounds. The Entire Document image case is expected
  to pass today; the mask case fails until the fix.
- `tst_layers_panel_fixes::maskThumbnailKeepsDocumentAspect` pins the painted
  card: image and mask image aspect, mask pixel at its document offset, and the
  checkerboard in both cards' margins. It stays inside that file's 44-line
  headroom against the 1400-line test cap; the Layer Bounds assertion stays in
  the snapshot test to keep it short.

## Risks / Trade-offs

- [Layer Bounds letterbox bands now show the checkerboard around a
  mask-aspect image] -> Accepted and stated in the spec; the bands are empty
  space, and the image card already draws the checker there.
- [Rounding makes the mask image one pixel off the card] -> The delegate fits
  the image aspect-preserving into the card, so no stretch; a sub-pixel gap is
  invisible and the tests use exact 24x6 geometry.
- [A mask whose data length does not match its rect returns no thumbnail] ->
  Unchanged from today; the row shows an empty card rather than a distorted one.
  The unit test asserts the refusal.

## Migration Plan

None. Session state, document format, and saved files are unchanged.
