# Spec Delta — ui/layers-panel

## MODIFIED Requirements

### Requirement: Layer thumbnail preserves the canvas aspect ratio

A layer row thumbnail SHALL be drawn at the document's aspect ratio inside the
row's thumbnail box, letterboxed rather than stretched into a square, so a wide
or tall document is not distorted. The thumbnail image itself SHALL also carry
the document's aspect ratio: a thumbnail built for Entire Document placement
SHALL size its output from the document aspect (`size` on the document's long
edge and the proportional length on the short edge), placing the layer's content
at its document-relative position, so a full-width layer has no transparent
padding strip and is not squashed. A shape layer's thumbnail SHALL likewise
carry the document aspect. The thumbnail outline, the transparency checkerboard,
the active bracket, and the thumbnail hit-target SHALL all use the letterboxed
rect. A group's folder glyph keeps its square slot. A **type card, an
adjustment-kind icon card, a fill preview, and a content-less synthetic row's
card SHALL be drawn as a 1:1 square** instead of the letterboxed rect (see the
thumbnail-layout requirement), so an icon is not stretched and a gradient ramp
or pattern tile is not squashed.

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

### Requirement: Layers panel thumbnail layout

Within a layer row the mask thumbnail SHALL sit immediately to the right of the
image thumbnail with a chain glyph between them (a broken-chain glyph when the
link is off), and a vector-mask thumbnail SHALL sit immediately to the right of
the mask thumbnail with the same chain glyph between them. The mask and vector
thumbnails SHALL have the same aspect ratio as the image thumbnail. The chain
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
document aspect so a ramp or tile is not stretched into the card. An adjustment
or fill row's mask and vector-mask thumbnails SHALL use that same 1:1 card,
with their preview drawn fit inside the card.

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

#### Scenario: A mask thumbnail matches the 1:1 card [lpr_thumb_mask_square]

- **WHEN** an adjustment or fill layer carries a layer mask or a vector mask
- **THEN** the mask and vector-mask thumbnails use the same 1:1 card as the
  image thumbnail

### Requirement: Layer row badges, labels, and delegate

The system SHALL draw each row with a delegate that paints, in CS6 order, the
visibility toggle, the thumbnail (a folder glyph for a group), the name, a
clipping-mask indicator for a clipped layer, the clipping indentation and base
underline, the layer-mask thumbnail when a mask is present, an adjustment badge
when adjustment content is present, a layer-style `fx` badge when the layer's
style carries at least one effect, and a compact
**`Blend If` text chip** when the layer's advanced blending is customised — a
non-default `Blend If` view, or a raw `blending_ranges` block present without a
typed view. The chip SHALL be a text-only badge (no icon asset) drawn in the
row's right-edge badge run left of the `fx` and lock badges, and its advance
SHALL be reserved by the same right-edge walk that lays out the mask thumbnails
and the name rect so nothing overlaps. A layer whose lock
state has any flag set SHALL also show a lock badge at the right side of its
row, except that the Transparency and Image locks a type or shape layer is
forced to carry SHALL NOT by themselves show one (CS6 shows them only in the
Lock strip); an unlocked layer SHALL show none. The visibility toggle SHALL be an eye
icon (`layers.eyeOn`/`layers.eyeOff`) drawn slightly inset from the panel's left
edge and at the same x for every row, independent of nesting depth; the nesting
indentation SHALL apply to the thumbnail and name, not to the visibility toggle.
A layer whose color label is not `None` SHALL fill the **whole visibility
column** (the full 30 px column, at the row's full height) with that label
color at full opacity behind the eye glyph. The eye glyph SHALL remain legible
against the fill, a selected labeled row SHALL keep the label fill in its
visibility column rather than the selection surface, and the label color
elsewhere on the row SHALL fall back to the row background. The fill is
paint-only and SHALL NOT alter the visibility column's hit target. The labeled
row's thumbnail, name, mask, link, and vector hit rectangles SHALL match an
unlabeled row's x, with no color-label chip or chip-reserved space before the
thumbnail. The row's 1 px bottom rule and the gutter's 1 px separator SHALL
remain drawn over the fill. A group with at least one child
SHALL show a disclosure icon — right when collapsed, down when expanded — at its
indented position, and clicking that icon SHALL expand or collapse the group. A
**regular (non-group) layer's thumbnail SHALL be drawn over a cached two-tone
checkerboard** so transparency reads under it, while a group keeps its folder
glyph and no checkerboard. A **type layer's thumbnail SHALL be drawn as a
square (1:1) white card bearing a "T"**, sized to the thumbnail's side and
centred in its slot, regardless of the document's aspect ratio; it SHALL NOT be
letterboxed to the document, and its hit rectangle SHALL be that square card.
An **adjustment row, a fill row, and a content-less synthetic row (Smart
Filters, layer effects) SHALL likewise draw a 1:1 card** (see the
thumbnail-layout requirement), and an icon-only row's hit rectangle SHALL be
that card. Every other non-group layer's thumbnail SHALL be laid out in a box
`thumb` wide and the row's full height minus a small pad, and SHALL be letterboxed to the
document's aspect ratio inside that box, so a tall document's thumbnail fills the
row vertically rather than floating in a square box. **Every non-group row whose image thumbnail is a
raster image SHALL carry a 1 px black outline**; an icon-only row — a type
card, an adjustment-kind icon card, or a content-less synthetic row — and a
group's folder glyph SHALL draw no outline, and **when exactly one
layer is active** (the same singular
active-layer resolution tool edits use) its thumbnail SHALL additionally show
white 1 px corner brackets drawn one pixel outside the thumbnail card (for a
regular layer, outside the outline); with zero or
multiple active layers no brackets are drawn. A layer whose row reports the
smart-object projection SHALL paint the `layers.kindSmartObject` badge on the
thumbnail's lower-right corner, whether the smart object is embedded or
placed/external; the badge SHALL be omitted when the asset is missing. Row
typography SHALL derive from the layer: a `background` layer's name SHALL be
italic/cursive, every other name normal, and a layer that is a member of the
frame's link set or a placed/external smart object SHALL be underlined through
the `LayerRowLinkedRole` and `LayerRowPlacedRole` projections. The delegate SHALL
report a row height of at least **28 px** through one named constant used by both
`sizeHint` and the vertical centring math. If an expected icon asset is
unavailable, the delegate SHALL omit that badge while keeping the row legible
rather than fail.

#### Scenario: The visibility toggle is an eye icon [lpr_eye]

- **WHEN** a layer row is shown
- **THEN** its visibility toggle is drawn from the eye icon asset, and a hidden
  layer uses the off variant

#### Scenario: The eye is left-anchored for every depth [lpr_eye]

- **WHEN** a nested layer is shown under its group
- **THEN** its eye icon is at the same x as a top-level row's, while its
  thumbnail and name are indented

#### Scenario: The color label fills the visibility cell [lpr_label_tint]

- **WHEN** a layer with a non-`None` color label is shown
- **THEN** the entire visibility column is filled with that label color behind
  the eye glyph, the glyph stays legible against the fill, no separate chip is
  painted, and the rest of the row background is unchanged

#### Scenario: An unlabeled layer keeps the normal gutter [lpr_label_tint_none]

- **WHEN** a layer with color label `None` is shown
- **THEN** its whole visibility column uses the normal gutter surface and no
  tint or chip is painted

#### Scenario: The eye glyph stays legible on every label [lpr_label_glyph]

- **WHEN** a row carries each of the seven color labels
- **THEN** the eye glyph is drawn with a dark tint on bright fills and a light
  tint on dark fills, so it stays readable against the fill

#### Scenario: A smart object shows the badge [lpr_smart_badge]

- **WHEN** an embedded or placed smart-object layer is shown
- **THEN** its thumbnail draws the smart-object badge on the lower-right corner

#### Scenario: A non-smart layer shows no smart badge [lpr_smart_badge_absent]

- **WHEN** an ordinary pixel layer is shown
- **THEN** no smart-object badge is drawn on its thumbnail

#### Scenario: A group shows a disclosure icon [lpr_chevron]

- **WHEN** a group with children is shown collapsed and then expanded
- **THEN** it shows the right-pointing icon when collapsed and the down-pointing
  icon when expanded, and clicking the icon toggles it

#### Scenario: A masked layer shows a mask thumbnail [m39_badges]

- **WHEN** a layer carries a layer mask
- **THEN** its row shows a mask thumbnail and the mask-presence flag is set

#### Scenario: A clipped layer shows the clipping indicator [lpc_clip]

- **WHEN** a layer is clipped to the layer below it
- **THEN** its row draws the clipping-mask indicator in addition to the
  indentation and the base layer's underline

#### Scenario: A group row is expandable [m39_badges]

- **WHEN** a group has children
- **THEN** its row is marked expandable and shows an expand/collapse control

#### Scenario: A locked layer shows a right-side lock badge [lpc_lockbadge]

- **WHEN** a layer has any lock flag set
- **THEN** its row draws a lock badge on the right side, and an unlocked layer
  draws no lock badge

#### Scenario: A type layer's forced locks show no badge [lpc_forced_lock_badge]

- **WHEN** `tst_type_tools::typeLayerRowHasNoLockBadge` commits a type layer, then also locks its position
- **THEN** the layer carries the Transparency and Image locks and its row has no lock badge; with Position locked too the badge shows

#### Scenario: A regular thumbnail shows a checkerboard [lpr_thumb_checker]

- **WHEN** a regular (non-group) layer with translucent pixels is shown
- **THEN** its thumbnail is drawn over a two-tone checkerboard, while a group
  row keeps its folder glyph and draws no checkerboard

#### Scenario: Thumbnails are outlined and the active layer is bracketed [lpr_thumb_bracket]

- **WHEN** a regular layer row is shown with a 1 px black thumbnail outline
- **THEN** the singular active layer's thumbnail additionally shows white
  corner brackets one pixel outside the outline, and a multi- or zero-selection
  row shows no brackets

#### Scenario: A group icon has no outline [lpr_group_no_outline]

- **WHEN** a group row is shown
- **THEN** its folder glyph carries no 1 px black outline, while a regular
  layer's thumbnail keeps its outline

#### Scenario: A type card is square with no outline [lpr_type_thumb_card]

- **WHEN** a type layer row is shown in a document of any aspect ratio
- **THEN** its thumbnail is a square white card with a "T" whose width equals
  its height, it is not letterboxed to the document, it carries no 1 px black
  outline, and the thumbnail's hit rectangle is that same square card

#### Scenario: A tall thumbnail fills the row [lpr_thumb_fill]

- **WHEN** a portrait document's layer row is shown
- **THEN** its letterboxed thumbnail is taller than it is wide and spans the
  row's full height minus the thumbnail pad

#### Scenario: Row typography follows the layer kind [lpr_row_fonts]

- **WHEN** the rows for the Background, a linked layer, and a placed smart
  object are inspected
- **THEN** the Background name is italic/cursive, ordinary names are normal, and
  the linked and placed rows are underlined through their roles

#### Scenario: The row height has a floor [lpr_row_height]

- **WHEN** a row's size hint is read
- **THEN** its height is at least 28 px from the single named row-height
  constant

#### Scenario: A missing badge asset does not break the row [m39_badges]

- **WHEN** the adjustment badge's icon asset is missing
- **THEN** the row is still drawn and the badge is simply omitted

#### Scenario: A styled layer shows the fx badge

- **WHEN** a layer whose style carries at least one effect is shown
- **THEN** its row draws the `layers.fx` badge at its right edge beside the lock
  and mask badges, and a layer with no style, or whose style carries no effect,
  draws no fx badge

#### Scenario: An adjustment row shows no fx badge [lpr_fx_absent_adjustment]

- **WHEN** an adjustment or fill layer whose style carries no effect is shown
- **THEN** its row draws no `fx` badge and reserves no fx space, while the same
  layer once it carries an effect draws one

#### Scenario: A customised Blend If shows the badge [lpr_blendif_badge]

- **WHEN** a layer whose advanced blending is customised (its Blend If view is
  not the full `(0, 65535)` default, or a raw `blending_ranges` block is
  present without a typed view) is shown
- **THEN** its row draws a compact `Blend If` text chip in the right-edge badge
  run, and the name elides before the chip so no rect overlaps

#### Scenario: A default Blend If shows no badge [lpr_blendif_badge_absent]

- **WHEN** a layer with no Blend If view and no `blending_ranges` block, or one
  whose every range is the full default, is shown
- **THEN** no `Blend If` chip is drawn on its row and the name keeps the full
  right-edge run

#### Scenario: A raw blending-ranges block without a typed view still badges [lpr_blendif_raw]

- **WHEN** a layer carries a non-empty `blending_ranges` block that cannot be
  parsed into a typed Blend If view
- **THEN** its row still draws the `Blend If` chip

#### Scenario: An icon-only row draws no outline [lpr_thumb_icon_no_outline]

- **WHEN** an adjustment, type, Smart Filters, or layer-effects row with no
  raster image thumbnail is shown
- **THEN** its card carries no 1 px black outline, while a pixel or fill row
  with a raster image thumbnail keeps its outline
