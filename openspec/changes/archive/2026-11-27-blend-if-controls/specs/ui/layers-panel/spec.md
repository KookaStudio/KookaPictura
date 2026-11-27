# Layers panel

## MODIFIED Requirements

### Requirement: Layer row badges, labels, and delegate

The system SHALL draw each row with a delegate that paints, in CS6 order, the
visibility toggle, the thumbnail (a folder glyph for a group), the name, a
clipping-mask indicator for a clipped layer, the clipping indentation and base
underline, the layer-mask thumbnail when a mask is present, an adjustment badge
when adjustment content is present, a layer-style `fx` badge when the layer's
style carries at least one effect, and a compact
**`Blend If` icon** when the layer's advanced blending is customised — a
non-default `Blend If` view, or a raw `blending_ranges` block present without a
typed view. The icon SHALL be an original, provenance-recorded asset drawn in the
row's right-edge badge run left of the `fx` and lock badges, and its advance
SHALL be reserved by the same right-edge walk that lays out the mask thumbnails
and the name rect so nothing overlaps. A layer whose lock
state has any flag set SHALL also show a lock badge at the right side of its
row, except that the Transparency and Image locks a type or shape layer is
forced to carry SHALL NOT by themselves show one (CS6 shows them only in the
Lock strip). The Background SHALL show the lock badge regardless of its lock
flags, since it is locked by kind; an unlocked ordinary layer SHALL show none.
The lock badge's side SHALL be
80% of its previous two-thirds-thumbnail side, so it reads as a smaller badge
and the lock stays the rightmost badge. The `fx` badge's side SHALL be 80% of
the row thumbnail's badge side, and the clipping-mask glyph's side SHALL be 80%
of its previous thumbnail-minus-8 side, with a 4 px gap before the thumbnail, so
both read smaller without colliding with the lock or the list toggle. For a
layer row, the visibility toggle SHALL be an
eye icon (`layers.eyeOn`) centred in its fixed left gutter at the same
x for every layer row, independent of nesting depth; the nesting indentation
SHALL apply to the thumbnail and name, not to the visibility toggle. A hidden
row SHALL paint the `layers.eyeOff` glyph at 50% opacity, keeping the eye rect
as the toggle target. A synthetic effect or filter row SHALL align its eye with
the owning layer's image-thumbnail right edge; each child row sits one indent
step deeper.
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
remain drawn over the fill. Inside an open synthetic list, the left cell's 1 px
right border continues through the list rows, no bottom rule is drawn between
the list rows, and the block's bottom rule closes the block after its last row. A group with at least one child
SHALL show a disclosure icon — right when collapsed, down when expanded — at its
indented position, and clicking that icon SHALL expand or collapse the group. A
**regular (non-group) layer's thumbnail SHALL be drawn over a cached two-tone
checkerboard** so transparency reads under it, while a group keeps its folder
glyph and no checkerboard. A **type layer's thumbnail SHALL be drawn as a
square (1:1) white card bearing a "T"**, sized to the thumbnail's side and
centred in its slot, regardless of the document's aspect ratio; it SHALL NOT be
letterboxed to the document, and its hit rectangle SHALL be that square card.
An **adjustment row and a fill row SHALL draw a 1:1 card** (see the
thumbnail-layout requirement), and an icon-only row's hit rectangle SHALL be
that card. A synthetic Smart Filters or layer-effects row SHALL draw no
thumbnail card and SHALL NOT reserve a thumbnail slot. Every other non-group layer's thumbnail SHALL be laid out in a box
`thumb` wide and the row's full height minus a small pad, and SHALL be letterboxed to the
document's aspect ratio inside that box, so a tall document's thumbnail fills the
row vertically rather than floating in a square box. **Every non-group row whose image thumbnail is a
raster image SHALL carry a 1 px black outline**; an icon-only row — a type
card or an adjustment-kind icon card — and a group's folder glyph SHALL draw
no outline, and **when exactly one
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
- **THEN** its visibility toggle is drawn from the eye icon asset; a hidden row
  paints the eye-off glyph at 50% opacity, and the eye slot remains the toggle
  target

#### Scenario: The eye keeps one x for every depth [lpr_eye]

- **WHEN** a regular nested layer is shown under its group
- **THEN** its eye icon is at the same x as a top-level row's, while its
  thumbnail and name are indented; a synthetic group header instead aligns its
  eye with the owning layer's image-thumbnail right edge, and each child row
  sits one indent step deeper

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

#### Scenario: The lock badge is 80% of its previous side [lpr_lock_size]

- **WHEN** a locked layer is shown
- **THEN** the lock badge's side is 80% of the old two-thirds-thumbnail side and
  smaller than the thumbnail

#### Scenario: The fx and clip badges shrink with the lock [lpr_badge_sizes]

- **WHEN** a styled layer with a clipping mask is shown
- **THEN** the fx badge's side is 80% of the row thumbnail's badge side and the
  clipping-mask glyph's side is 80% of its old thumbnail-minus-8 side, with a
  4 px gap before the thumbnail, so neither collides with the lock or the list
  toggle

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
- **THEN** its row draws a compact `Blend If` icon in the right-edge badge
  run, and the name elides before the icon so no rect overlaps

#### Scenario: A default Blend If shows no badge [lpr_blendif_badge_absent]

- **WHEN** a layer with no Blend If view and no `blending_ranges` block, or one
  whose every range is the full default, is shown
- **THEN** no `Blend If` icon is drawn on its row and the name keeps the full
  right-edge run

#### Scenario: A raw blending-ranges block without a typed view still badges [lpr_blendif_raw]

- **WHEN** a layer carries a non-empty `blending_ranges` block that cannot be
  parsed into a typed Blend If view
- **THEN** its row still draws the `Blend If` icon

#### Scenario: An icon-only row draws no outline [lpr_thumb_icon_no_outline]

- **WHEN** an adjustment or type row with no raster image thumbnail is shown
- **THEN** its card carries no 1 px black outline, while a pixel or fill row
  with a raster image thumbnail keeps its outline; a synthetic Smart Filters or
  layer-effects row draws no card at all
