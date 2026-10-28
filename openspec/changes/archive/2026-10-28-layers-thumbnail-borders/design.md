# Design: Layers panel thumbnail borders

## Context

`LayerRowDelegate` (`crates/pictura-app/cpp/panels/layers_panel_internal.h`) lays
out one square slot per row, `thumbSlot()`, that is `thumbnailSize_` wide and the
row's full height minus a small pad tall (24×32 at the default Medium size). It
maps that slot to the drawn card through `thumbnailCard()`:

- a **type** row is squared to `min(width, height)`;
- every other non-group row is passed to `letterboxedThumb()`, which shrinks the
  slot to the document aspect;
- a **group** row keeps the slot (its folder glyph is drawn centred).

`paint()` walks that card and then, for every non-group/non-type row that is not
a layer-effect row, rings it with a 1 px black outline. Adjustment rows
(`kind == "adjustment"`, which covers both adjustments and fills) and the
content-less synthetic rows (Smart Filters, layer effects) therefore get a
document-aspect card and an outline they should not have.

## Decisions

### One predicate for the 1:1 card

A row draws a 1:1 card when it is a type row, an adjustment row, or a
content-less row with no raster image thumbnail:

```cpp
bool squareThumbCard(const QModelIndex& index) const
{
    const QString kind = index.data(KindRole).toString();
    if (kind == QLatin1String("group")) {
        return false;
    }
    return kind == QLatin1String("adjustment")
        || index.data(ThumbnailRole).value<QImage>().isNull();
}
```

- A **type** row has a null `ThumbnailRole`, so it is covered by the null test.
- An **adjustment or fill** row has `kind == "adjustment"`; a fill row carries a
  raster preview, so it needs the kind test, not the null test.
- A **Smart Filters / layer-effects** row is synthetic with a null thumbnail, so
  the null test covers it.
- A **pixel, background, or shape** row carries a raster image thumbnail and is
  not an adjustment, so it stays letterboxed.
- A **group** is excluded so its slot (and its hit rectangle) is unchanged.

`thumbnailCard()` uses it for the square branch, and the mask and vector-mask
card rects switch from `letterboxedThumb()` to `thumbnailCard()` so all three
cards in an adjustment/fill row share the square. For a pixel row
`thumbnailCard()` still returns the letterboxed rect, so the mask/vector layout
is unchanged there.

### The outline frames a raster image

The outline is drawn only when the row carries a raster image thumbnail:

```cpp
if (!index.data(ThumbnailRole).value<QImage>().isNull()) {
    painter->drawRect(shaped.adjusted(0, 0, -1, -1));
}
```

This is exactly the issue's rule ("skip the outline when the row has no raster
thumbnail"). It drops the outline from adjustment icon cards and from Smart
Filters rows (layer-effects rows already had none) while keeping it on pixel,
background, shape, and fill rows. Type and group rows have a null thumbnail, so
they keep drawing none, as before.

### Icons fill the square card

The adjustment icon and the Smart Filters / layer-effects `fx` badge were drawn
into `box` (the 24×32 slot) and so were stretched vertically. Both now draw into
`shaped` (the square card), matching the type "T" branch. Shape and smart-object
corner badges still anchor to `box`, unchanged.

### The fill preview is square at the source

`fill_thumbnail_image()` sized its preview from the document (or layer-bounds)
aspect. Squaring only the card would scale that document-aspect image into the
square, so the bridge now renders the preview at `(size, size)` instead. A solid
swatch is unaffected; a gradient is re-rendered as a square ramp
(`gradient_rgba` takes the output dimensions); a pattern is sampled over the
layer rect into the square card. The panel option (Entire Document / Layer
Bounds) no longer changes a fill preview's shape — a fill preview is a square
swatch, which is what the 1:1 card wants. The function's now-unused
`entire_document` argument is dropped.

### Mask and vector previews are drawn fit inside the square card

A mask preview is document-shaped coverage, so squaring it would misrepresent the
mask. The card is square (matching the image card, per the layout requirement)
and the preview is drawn centred with `QSize::scaled(..., KeepAspectRatio)`. For
a pixel row the image aspect already equals the card aspect, so the fit is an
exact fill and pixel-row mask rendering is unchanged.

## Ceilings

- `ponytail:` a square mask card with a document-aspect mask leaves the card's
  side margins showing the row surface. That is the no-distortion choice; CS6's
  own mask-cell geometry is unconfirmed.
- A fill preview no longer honours Entire Document / Layer Bounds for its shape;
  it is always square. The panel option still governs pixel and shape rows.
- `ponytail:` a pattern fill's tile is sampled over the layer rect into the
  square card, so a non-square layer rect scales the tile with the card rather
  than preserving the tile's own aspect.
