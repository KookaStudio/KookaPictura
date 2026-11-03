# Spec Delta

## MODIFIED Requirements

### Requirement: Layer thumbnail preserves the canvas aspect ratio

A layer row thumbnail SHALL be drawn at the document's aspect ratio inside the
row's thumbnail box, letterboxed rather than stretched into a square, so a wide
or tall document is not distorted. The thumbnail image itself SHALL also carry
the document's aspect ratio: a thumbnail built for Entire Document placement
SHALL size its output from the document aspect (`size` on the document's long
edge and the proportional length on the short edge), placing the layer's content
at its document-relative position, so a sub-canvas layer shows its content at
its document position with transparent margins and a full-width layer has no
transparent padding strip and is not squashed. A shape layer's thumbnail SHALL
likewise carry the document aspect. A layer mask's thumbnail SHALL likewise
carry the document aspect in Entire Document placement: it SHALL size its output
from the document aspect, place the mask rect at its document-relative position
clipped to the document bounds, and leave the area outside the mask rect
transparent so the transparency checkerboard shows. In Layer Bounds placement
the mask image SHALL keep sizing from the mask rect while its card stays
letterboxed to the document, so the uncovered bands show the checkerboard. The
vector-mask thumbnail samples the document extent in both placements. The
thumbnail outline, the transparency checkerboard, the active bracket, and the
thumbnail hit-target SHALL all use the letterboxed rect. A group's folder glyph
keeps its square slot. A **type card, an adjustment-kind icon card, a fill
preview, and a content-less synthetic row's card SHALL be drawn as a 1:1
square** instead of the letterboxed rect (see the thumbnail-layout requirement),
so an icon is not stretched and a gradient ramp or pattern tile is not squashed.

#### Scenario: A non-square document is not stretched [lpr_thumb_aspect]

- **WHEN** a regular layer of a document whose width and height differ is shown
- **THEN** the drawn thumbnail's aspect ratio matches the document, with the
  unused part of the row's thumbnail box left empty

#### Scenario: A full-width layer has no transparent strip [lpr_thumb_source_aspect]

- **WHEN** a layer spanning the full width of a tall document is shown with
  Entire Document thumbnail contents
- **THEN** the thumbnail image has the document's aspect ratio and its rightmost
  column is layer content rather than transparent padding

#### Scenario: An icon or fill row keeps a square card [lpr_thumb_square_card]

- **WHEN** an adjustment, fill, type, or content-less synthetic row is shown in
  a document whose width and height differ
- **THEN** its thumbnail card is 1:1 (width equals height), while a pixel
  layer's thumbnail is still letterboxed to the document

#### Scenario: A sub-canvas layer is placed in the document [lpr_thumb_sub_canvas_layer]

- **WHEN** a raster layer smaller than the canvas is shown on a non-square
  document with Entire Document thumbnail contents
- **THEN** its thumbnail image carries the document's aspect ratio, the layer
  content sits at its document-relative position, and the surrounding margin is
  transparent and painted as the checkerboard

#### Scenario: A raster mask uses the document aspect [lpr_thumb_mask_aspect]

- **WHEN** a layer-sized raster mask is shown on a non-square document with
  Entire Document thumbnail contents
- **THEN** the mask thumbnail image carries the document's aspect ratio, the
  mask content sits at its document-relative position with the checkerboard
  outside it, and its aspect matches the layer image thumbnail
- **AND WHEN** the mask extends past the document bounds
- **THEN** the mask thumbnail is clipped to the document rather than scaled down
- **AND WHEN** the same row is shown with Layer Bounds thumbnail contents
- **THEN** the mask image is sized from the mask rect while its card stays
  document-aspect, with the checkerboard in the uncovered bands
