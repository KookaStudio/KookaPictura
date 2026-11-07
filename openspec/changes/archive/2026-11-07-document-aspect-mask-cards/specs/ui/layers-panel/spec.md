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
likewise carry the document aspect. A raster layer mask's thumbnail image SHALL
carry the document aspect in both Entire Document and Layer Bounds placement,
sampling mask coverage over the document bounds at its document coordinates.
Its image pixels SHALL be opaque grayscale throughout; zero-coverage margins
SHALL be black rather than transparent or checkerboard. A mask extending outside
the canvas SHALL be clipped only for the thumbnail's sampled view, not scaled
down or changed in storage. The mask card, outline, brackets and hit target SHALL follow the document
aspect for every owner, including adjustment, fill, type and group layers.
Raster and vector masks SHALL use document-aspect cards independently of the
left content thumbnail; adjustment and fill content cards remain square. The vector-mask thumbnail samples the
document extent in both placements. The thumbnail outline, the transparency
checkerboard, the active bracket, and the thumbnail hit-target SHALL all use
the letterboxed rect. A group's folder glyph keeps its square slot. A **type
card, an adjustment-kind icon card, a fill preview, and a content-less synthetic
row's card SHALL be drawn as a 1:1 square** instead of the letterboxed rect (see
the thumbnail-layout requirement), so an icon is not stretched and a gradient
ramp or pattern tile is not squashed.

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

- **WHEN** a raster mask on a pixel, adjustment, fill, type or group layer is shown on a
  non-square document with Entire Document thumbnail contents
- **THEN** the mask thumbnail image carries the document's aspect ratio, its
  coverage sits at its document-relative position with opaque black
  zero-coverage margins, independently of the left content thumbnail card
- **AND WHEN** the mask extends past the document bounds
- **THEN** the mask thumbnail is clipped to the document rather than scaled down,
  without changing the stored mask samples
- **AND WHEN** the same row is shown with Layer Bounds thumbnail contents
- **THEN** the mask image keeps its document aspect and opaque grayscale canvas
  coverage while its card retains the existing document-aspect layout

## REMOVED Requirements

### Requirement: Layers panel thumbnail layout

**Reason**: Its square-mask clause is superseded by universal document-aspect masks.
**Migration**: Replaced by Layers panel content and mask thumbnail layout, preserving adjacency, content cards, glyphs and bracket behavior.

### Requirement: Raster mask cards remain opaque after fitting

**Reason**: Its square-mask exception conflicts with the requested document aspect for every mask.
**Migration**: Replaced by Opaque document-aspect mask cards with retained opacity and edge coverage.

## ADDED Requirements

### Requirement: Layers panel content and mask thumbnail layout

Within a layer row the mask thumbnail SHALL sit immediately to the right of the
image thumbnail with a chain glyph between them (a broken-chain glyph when the
link is off), and a vector-mask thumbnail SHALL sit immediately to the right of
the mask thumbnail with the same chain glyph between them. The mask and vector
thumbnails SHALL follow the document aspect independently of the image
thumbnail and owning layer kind. The chain
glyph SHALL be drawn two pixels larger than its slot and rotated a quarter turn
so it reads vertically between the side-by-side thumbnails, in both the linked
and unlinked states. A row's active edit target SHALL be marked with brackets
around that thumbnail, and clicking a thumbnail SHALL make it the active target;
Alt+clicking a thumbnail SHALL also activate it in the composite. A group row
SHALL NOT draw brackets around its icon. An **adjustment row's image thumbnail
SHALL be a neutral card bearing the icon for its decoded adjustment kind**, with
no checkerboard; a **fill row's image thumbnail SHALL show its content preview**
(a flat solid swatch, a gradient ramp, or a scaled pattern tile) **in a 1:1
square card**, the preview image itself rendered square rather than at the
document aspect so a ramp or tile is not stretched into the card. Mask and vector-mask cards on adjustment, fill, type and group rows SHALL
use the document aspect; the left content card keeps its existing geometry.

#### Scenario: Mask thumbnail adjacency [uip_layers_mask_adjacency]

- **WHEN** a layer with a mask and a vector mask is shown
- **THEN** the mask thumbnail is directly right of the image thumbnail and the vector mask directly right of the mask, each with a chain glyph

#### Scenario: Active thumbnail brackets [uip_layers_active_thumb]

- **WHEN** a layer thumbnail is clicked to become the edit target
- **THEN** brackets mark that thumbnail and a group row never shows brackets on its icon

#### Scenario: The chain glyph is larger and rotated [uip_layers_link_glyph]

- **WHEN** a masked or vector-masked row is shown, linked or unlinked
- **THEN** its chain glyph is drawn two pixels larger than the slot and rotated
  a quarter turn, so its ink is taller than it is wide

#### Scenario: An adjustment row shows its kind icon [lpr_thumb_adjustment_icon]

- **WHEN** an Invert, Levels, or other adjustment layer is shown
- **THEN** its thumbnail is a neutral card with that adjustment kind's icon and no checkerboard

#### Scenario: A fill row shows its content preview [lpr_thumb_fill_preview]

- **WHEN** a Solid Color, Gradient, or Pattern fill layer is shown
- **THEN** its thumbnail shows the fill's solid colour, gradient ramp, or scaled pattern tile instead of a generic checkerboard

#### Scenario: An adjustment row's card is 1:1 [lpr_thumb_adjustment_square]

- **WHEN** an adjustment layer is shown in a document whose width and height
  differ
- **THEN** its image thumbnail card is 1:1 (width equals height) rather than
  letterboxed to the document

#### Scenario: A fill preview is a 1:1 card [lpr_thumb_fill_square]

- **WHEN** a Solid Color, Gradient, or Pattern fill layer is shown in a document
  whose width and height differ
- **THEN** its card is 1:1, the preview image the bridge supplies is square, and
  the preview fills the card without a document-aspect margin or distortion

#### Scenario: A mask thumbnail follows the document beside a square card [lpr_thumb_mask_square]

- **WHEN** an adjustment or fill layer carries a layer mask or a vector mask
- **THEN** the mask and vector-mask cards follow the document aspect while the
  left content thumbnail keeps its square card

### Requirement: Opaque document-aspect mask cards

The Layers delegate SHALL cover the entire document-aspect raster mask card
with its opaque grayscale preview, including when independently rounded image
and card sizes differ. A second aspect fit SHALL NOT expose the checkerboard
along any card edge. Mask cards on group, type, adjustment and fill rows SHALL also follow the
document aspect and stay filled by their opaque previews. Their geometry SHALL
be independent of the left content card.

#### Scenario: A portrait PSD opens without an edge strip [mtp_portrait]

- **WHEN** portrait and landscape PSD masks are opened at each thumbnail size
- **THEN** their source thumbnails are opaque and their document-aspect cards contain grayscale coverage through every interior edge, with a black outline and no checkerboard strip, including at fractional display scale

#### Scenario: Mask cards use the document shape for every owner [mtp_square]

- **WHEN** mask previews are displayed on adjustment or fill rows in a portrait document
- **THEN** the mask cards follow the document aspect and their opaque previews fill those cards, while the adjustment or fill content cards stay square

### Requirement: Plain layer-row clicks activate content by default

A plain click on a real layer row outside its dedicated controls SHALL activate
that row's left content thumbnail, regardless of the layer kind or a previously
active raster or vector mask. It SHALL clear raster-mask editing and solo view
without changing pixels or recording history. Explicit mask-thumbnail clicks
SHALL continue to activate the clicked mask. Creating an adjustment layer with
a mask SHALL select the new layer and activate its mask; this creation policy
SHALL NOT be replaced by the plain-row-click default.

#### Scenario: Clicking the current row returns to content [mcd_row_same]

- **WHEN** a raster mask is active or soloed and its layer's row body is clicked
- **THEN** the left content thumbnail is active, raster mask editing and solo are cleared, and history is unchanged

#### Scenario: Revisiting a row defaults to content [mcd_row_other]

- **WHEN** a pixel, adjustment, fill, type or group row previously had a raster or separately displayed vector mask active, another row is selected, and its row body is clicked again
- **THEN** the revisited row's left content thumbnail is active instead of a remembered mask target

#### Scenario: A direct mask click activates the mask [mcd_explicit_mask]

- **WHEN** a raster or vector mask thumbnail is clicked directly
- **THEN** the clicked mask remains the active thumbnail rather than the default content thumbnail

#### Scenario: Adjustment creation activates its mask [mcd_creation]

- **WHEN** a new adjustment layer with a mask is created while another layer's content thumbnail is active
- **THEN** the new adjustment is selected and its mask thumbnail is active in the normal single creation history state
