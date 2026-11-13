# Spec Delta

## MODIFIED Requirements

### Requirement: Layer thumbnail preserves the canvas aspect ratio

A layer row thumbnail SHALL be drawn at the document's aspect ratio inside the
row's thumbnail box, letterboxed rather than stretched into a square, so a
document whose rounded extent already meets the bound is not distorted; the
drawn aspect SHALL never be more extreme than 3:1 or 1:3. A card whose rounded
proportional aspect would fall outside that range SHALL use the bound instead,
and its whole content SHALL be stretched to fill the clamped card, so no part of
the document is clipped or truncated and the card's aspect is never more extreme
than 3:1 or 1:3. The thumbnail image itself SHALL also carry the same
clamped aspect: a thumbnail built for Entire Document placement SHALL size its
output from the clamped document aspect (`size` on the document's long edge and
a short edge that is rounded proportionally, then raised when it is below one
third of the long edge; a rounded extent that already meets the bound keeps its
proportional round), placing the layer's content at its document-relative
position and scaling each axis into the clamped extent, so a sub-canvas layer
shows its content at its document position with transparent margins and a
full-width layer has no transparent padding strip. A Layer Bounds content thumbnail keeps its layer-bounds aspect;
its card still follows the clamped document aspect. A shape layer's thumbnail
SHALL likewise carry the clamped document aspect. A
raster layer mask's thumbnail image SHALL carry the clamped document aspect in
both Entire Document and Layer Bounds placement, sampling mask coverage over the
document bounds at its document coordinates. Its image pixels SHALL be opaque
grayscale throughout; zero-coverage margins SHALL be black rather than
transparent or checkerboard. A mask extending outside the canvas SHALL be
clipped only for the thumbnail's sampled view, not scaled down or changed in
storage. The mask card, outline, brackets and hit target SHALL follow the
clamped document aspect for every owner, including adjustment, fill, type and
group layers. Raster and vector masks SHALL use clamped-document-aspect cards
independently of the left content thumbnail; adjustment and fill content cards
remain square. A vector-mask thumbnail image SHALL also carry the clamped
document aspect in both placements, sampling the document extent. The thumbnail
outline, the transparency checkerboard, the active bracket, and the thumbnail
hit-target SHALL all use the letterboxed rect. A
group's folder glyph keeps its square slot. A **type card, an adjustment-kind
icon card, and a fill preview SHALL be drawn as a 1:1 square** instead of the
letterboxed rect (see the thumbnail-layout requirement), so an icon is not
stretched and a gradient ramp or pattern tile is not squashed. A synthetic
effect or filter row draws no thumbnail card at all.

#### Scenario: A non-square document is not stretched [lpr_thumb_aspect]

- **WHEN** a regular layer of a document well inside the range, such as 10x20
  or 20x10, whose rounded extent already meets the 3:1 bound, is shown
- **THEN** the drawn thumbnail's aspect ratio matches the document, with the
  unused part of the row's thumbnail box left empty

#### Scenario: An extreme document is clamped and stretched [lpr_thumb_aspect_clamp]

- **WHEN** a document wider than 3:1, taller than 1:3, or at the bound (such as
  300x100) is shown
- **THEN** a regular layer's drawn card and its thumbnail image use the clamped
  aspect (no more extreme than 3:1 or 1:3) rather than the document's own aspect
- **AND THEN** the thumbnail image's short edge is rounded and then raised to at
  least one third of its long edge, so 300x100 at 16 px gives 16x6, and the card
  rounds the same bound into the row slot
- **AND THEN** the document content fills the whole clamped card by stretching,
  with no clipping, truncation, or unused transparent strip
- **AND THEN** a raster mask and a vector mask on the same layer use the same
  clamped card

#### Scenario: A full-width layer has no transparent strip [lpr_thumb_source_aspect]

- **WHEN** a layer spanning the full width of a tall document is shown with
  Entire Document thumbnail contents
- **THEN** the thumbnail image has the clamped document aspect and its rightmost
  column is layer content rather than transparent padding

#### Scenario: An icon or fill row keeps a square card [lpr_thumb_square_card]

- **WHEN** an adjustment, fill, or type row is shown in
  a document whose width and height differ, including one outside the 3:1
  clamp range
- **THEN** its thumbnail card is 1:1 (width equals height), while a pixel
  layer's thumbnail is still letterboxed to the clamped document aspect

#### Scenario: A sub-canvas layer is placed in the document [lpr_thumb_sub_canvas_layer]

- **WHEN** a raster layer smaller than the canvas is shown on a non-square
  document with Entire Document thumbnail contents
- **THEN** its thumbnail image carries the clamped document aspect, the layer
  content sits at its document-relative position, and the surrounding margin is
  transparent and painted as the checkerboard

#### Scenario: A raster mask uses the document aspect [lpr_thumb_mask_aspect]

- **WHEN** a raster mask on a pixel, adjustment, fill, type or group layer is shown on a
  non-square document with Entire Document thumbnail contents
- **THEN** the mask thumbnail image carries the clamped document aspect, its
  coverage sits at its document-relative position with opaque black
  zero-coverage margins, independently of the left content thumbnail card
- **AND THEN** a vector-mask image on the same layer carries the same clamped
  aspect
- **AND WHEN** the mask extends past the document bounds
- **THEN** the mask thumbnail is clipped to the document rather than scaled down,
  without changing the stored mask samples
- **AND WHEN** the same row is shown with Layer Bounds thumbnail contents
- **THEN** the mask image keeps its clamped document aspect and opaque grayscale
  canvas coverage while its card retains the clamped-document-aspect layout
