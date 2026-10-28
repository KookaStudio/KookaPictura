# Proposal: Layers panel thumbnail borders

## Why

The Layers panel draws a row's thumbnail card at the document's aspect ratio and
rings every non-group card with a 1 px black outline. That is right for a pixel
layer, whose content really is document-shaped, but wrong for the rows that paint
an icon or a fill instead of raster content (#982):

- An **adjustment layer** shows its kind icon stretched into the row slot, with
  the card, its black outline, and the active-target brackets all letterboxed to
  the document, so the icon reads non-square.
- A **fill layer**'s preview (solid swatch, gradient ramp, pattern tile) is
  letterboxed to the document rather than square.
- A row that shows only an icon — an adjustment card, a Smart Filters row, a
  layer-effects row — still gets the 1 px black outline, though it has no raster
  image to frame.

CS6 shows the adjustment/fill thumbnail as a square card. The outline frames a
raster thumbnail; an icon-only row has none.

## What Changes

- A row that paints an icon or fill content instead of a raster image draws a
  **1:1 square card**: type (already square), adjustment, fill, and the
  content-less synthetic rows (Smart Filters, layer effects). A pixel, background,
  or shape row keeps its document-aspect letterboxed card.
- The adjustment icon and the synthetic rows' `fx` badge are drawn into that
  square card rather than the taller row slot, so the glyph is not stretched.
- A fill row's preview image is **rendered square by the bridge** rather than at
  the document aspect, so a gradient ramp or pattern tile is not squashed into
  the card.
- An **icon-only row draws no black outline** — the outline is drawn only when
  the row carries a raster image thumbnail. A fill row (which has a raster
  preview) keeps its outline.
- An adjustment or fill row's **mask and vector-mask thumbnails use the same 1:1
  card** as its image thumbnail, keeping the spec's "same aspect ratio as the
  image thumbnail" invariant; their previews are drawn fit inside the card so the
  mask shape is not stretched.
- **BREAKING**: none.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `ui/layers-panel`: the canvas-aspect requirement, the thumbnail-layout
  requirement, and the row-delegate requirement — adjustment, fill, type, and
  content-less synthetic cards are 1:1; a fill preview is square; an icon-only
  row draws no outline.

## Impact

- `crates/pictura-app/cpp/panels/layers_panel_internal.h` — `thumbnailCard`, the
  mask/vector card rects, the icon/badge draws, the outline block, and the
  mask/vector image draws in `LayerRowDelegate`.
- `crates/pictura-app/src/cxxqt_object/helpers_composite.rs` — `fill_thumbnail_image`
  renders a square preview.
- `crates/pictura-app/src/cxxqt_object/impl_layers.rs` — the `fill_thumbnail_image`
  call loses the now-unused contents argument.
- Tests: `tst_layers_panel_fixes`, `tst_layers_snapshot`, `tst_layers_effects`,
  `tst_layers_smart_filters`.
- Docs: `ROADMAP.md`, `docs/dev/STATE.md`.
- No new dependencies.
