# Spec Delta

## ADDED Requirements

### Requirement: Nested effect and filter list anatomy

A layer's synthetic Smart Filters and layer-effects lists SHALL sit inside the
layer's own cell rather than in a second group cell. A layer that owns effects,
smart filters, or both SHALL expose exactly one list toggle on its right edge,
immediately left of the lock badge and right of the `fx` badge. Opening the
toggle SHALL show every synthetic group the layer owns; closing it SHALL hide
them all, group headers included. The toggle SHALL point down when the list is
closed and up when it is open. The synthetic group headers SHALL NOT carry a
disclosure, and the synthetic rows SHALL draw no `fx` thumbnail. Each nested
row's eye SHALL start at the owning layer's image-thumbnail right edge — the
leftmost thumbnail, before any mask or vector thumbnail — and SHALL NOT indent
further by depth; the row's name SHALL follow its eye with half the normal name
gap. A hidden nested row SHALL paint the eye-off glyph at 50% opacity while
keeping the eye slot as its toggle target. A synthetic list row SHALL keep a
minimum 4 px vertical padding around its content. Clicking a synthetic row
outside its eye SHALL select and highlight the owning layer, not the synthetic
row. The layer's fixed left eye cell SHALL span the layer row and its list,
keeping its 1 px right border, and the right cell SHALL span the same block; no
horizontal rule SHALL be drawn between the list rows or between the Effects and
Smart Filters groups, and the block's bottom rule SHALL close the block after
its last row. A group (folder) row SHALL keep its left nested-layers chevron,
which is independent of the list toggle. A smart-filter row SHALL remain
draggable within its own list, and the synthetic group header SHALL NOT drag.

#### Scenario: One toggle controls both lists [lpr_one_list_toggle]

- **WHEN** a layer carries both layer effects and smart filters
- **THEN** its row draws exactly one right-edge list toggle, opening it shows
  both the `Effects` and `Smart Filters` groups, and closing it hides both

#### Scenario: The effects list toggle sits on the layer row [lpr_fx_list_toggle]

- **WHEN** a styled layer's effects list is shown
- **THEN** the layer row draws the single list toggle at its right edge beside
  the `fx` badge, pointing up while the list is open and down while it is
  closed, and the `Effects` header draws no disclosure

#### Scenario: The smart filters toggle sits on the layer row [lpr_sf_list_toggle]

- **WHEN** a smart object with smart filters is shown
- **THEN** its row draws the single list toggle at its right edge, pointing up
  while the list is open and down while it is closed, and the `Smart Filters`
  header draws no disclosure

#### Scenario: A closed list hides the whole group [lpr_list_closed]

- **WHEN** the list toggle is closed
- **THEN** every synthetic group header and list row the layer owns is hidden,
  and the layer row alone remains

#### Scenario: The nested rows form one cell [lpr_nested_cell]

- **WHEN** the effects or smart-filters list is open
- **THEN** the fixed left eye cell's 1 px right border and the right cell run
  through the layer row and every list row, no horizontal rule is drawn between
  the list rows or between the groups, each list row keeps its minimum vertical
  padding, and the block's bottom rule closes the block

#### Scenario: A synthetic row has no fx thumbnail [lpr_synthetic_no_thumb]

- **WHEN** a synthetic `Effects`, `Smart Filters`, effect, or filter row is
  shown
- **THEN** it draws no `fx` thumbnail and no thumbnail outline

#### Scenario: The nested eye aligns with the image thumbnail [lpr_nested_eye]

- **WHEN** a synthetic list is open on a layer that also has a mask
- **THEN** the group header's eye starts at the owning layer's image-thumbnail
  right edge (not after the mask), each child row sits one indent step deeper,
  and the row's name follows its eye with half the previous eye-to-name spacing

#### Scenario: A hidden nested row paints a faded eye [lpr_nested_hidden]

- **WHEN** a synthetic effect or filter row is hidden
- **THEN** its eye slot shows the eye-off glyph at 50% opacity and clicking the
  slot still toggles that row's visibility

#### Scenario: Clicking a synthetic row selects its layer [lpr_synthetic_click]

- **WHEN** a synthetic `Effects`, `Smart Filters`, effect, or filter row is
  clicked outside its eye
- **THEN** the owning layer becomes current and selected, and the synthetic row
  is not left highlighted

#### Scenario: A group row keeps its nested-layers chevron [lpr_group_chevron_kept]

- **WHEN** a group row with a child and a styled layer row are shown
- **THEN** the group row keeps the left nested-layers chevron, and the styled
  layer row draws the right-edge list toggle

## MODIFIED Requirements

### Requirement: Layer row visibility gutter and content spacing

For a layer row, the visibility toggle SHALL be centred horizontally inside its
fixed left gutter, with equal padding on its left and right sides. The gutter
SHALL be a flat fill one shade lighter than the row surface, not a gradient or a
darker recess. The
delegate SHALL draw a 1 px separator in a slightly darker grey at the gutter's
right edge, between the visibility toggle and the row content. The
expand/collapse chevron slot SHALL be reserved only for rows that are
expandable, so a regular (non-group) layer's thumbnail begins immediately after
the gutter instead of leaving an empty chevron-width gap. The eye hit-target,
the label fill, and the thumbnail/name hit-targets SHALL all mirror this layout.
A synthetic effect or filter row SHALL instead use the nested eye column defined
by the nested-list anatomy requirement: its eye aligns with the owning layer's
image-thumbnail right edge, each child row sits one indent step deeper, the
fixed left cell's 1 px
right border continues through it, and it reserves no chevron slot. A hidden row
SHALL paint the eye-off glyph at 50% opacity while keeping the eye rect as the
toggle target.

#### Scenario: The eye is centred in its gutter [lpr_eye_gutter]

- **WHEN** a layer row is shown at any depth
- **THEN** the eye glyph's own rect has equal left and right padding inside the
  fixed visibility gutter

#### Scenario: The eye cell is flat and lighter [lpr_eye_gutter_flat]

- **WHEN** a layer row is shown
- **THEN** its visibility cell is a flat fill one shade lighter than the row
  surface, with no gradient

#### Scenario: A separator divides the gutter from the content [lpr_eye_sep]

- **WHEN** a layer row is shown
- **THEN** a 1 px darker-grey vertical separator is drawn at the gutter's right
  edge between the eye and the content

#### Scenario: A non-group thumbnail sits close to the gutter [lpr_thumb_gap]

- **WHEN** a regular non-expandable layer and a group row are shown at the same
  depth
- **THEN** the regular layer's thumbnail starts immediately after the gutter,
  while the group row still reserves the chevron slot for its disclosure icon


### Requirement: Smart Filters tree rows

When a smart-object layer carries one or more smart filters, the Layers panel SHALL show a `Smart Filters` parent row directly under that layer, with one child row per filter carrying that filter's name, the most recently added filter on top (the reverse of the apply order), as CS6 stacks them. The parent row SHALL expose a visibility toggle for the group enable flag and each child row SHALL expose a visibility toggle for that filter's enable flag. Toggling either SHALL update the composite, record one history state, and persist the flag to the layer's preserved smart-object descriptor so it survives a write and re-read; the synthetic rows SHALL be non-editable and SHALL NOT appear as selectable layers to layer operations. A filter row SHALL drag up or down within its own Smart Filters list and nowhere else; the drop SHALL move that filter in the chain and record one history state, and because the chain applies bottom-up, a filter dragged above another SHALL apply after it, so its effect shows over the lower one. The Smart Filters parent row SHALL NOT drag. Double-clicking a filter row SHALL re-edit that filter instead of renaming it.
The layer row SHALL expose the lists through one right-edge toggle: the toggle
points down when the lists are closed and up when they are open, and closing it
SHALL hide the whole Smart Filters group and any layer-effects group on the same
layer. The Smart Filters parent row SHALL NOT carry its own disclosure. The
synthetic rows SHALL draw no `fx` thumbnail; each row's eye SHALL align with the
owning layer's image-thumbnail right edge, each child row sits one indent step
deeper, and a hidden row
SHALL paint the eye-off glyph at 50% opacity while keeping the eye slot
clickable. Clicking a synthetic row outside its eye SHALL select the owning
layer. The layer's left eye cell and right cell SHALL span the layer + list
block, and no horizontal rule SHALL be drawn between the nested rows or between
the Smart Filters and Effects groups.

#### Scenario: The parent and one child per filter appear

- **WHEN** `tst_layers_smart_filters::treeShowsGroupAndToggles` shows a smart-object layer with a filterFX chain
- **THEN** the panel shows a `Smart Filters` parent with one child row per filter, named for each filter

#### Scenario: The newest filter is on top [lp_sf_order]

- **WHEN** `tst_smart_filter_apply::filterRowsShowNewestFirst` applies Gaussian Blur and then Emboss to a Smart Object
- **THEN** the Emboss row is above the Gaussian Blur row

#### Scenario: Toggling a filter eye updates the composite

- **WHEN** `pictura-app::app_setter_changes_the_composite_and_records_one_state` flips a child row's visibility toggle
- **THEN** that filter's enable flag flips, the composite updates, and one history state is recorded

#### Scenario: Toggling the group eye bypasses the chain

- **WHEN** `pictura-app::disabled_group_bypasses_the_chain` flips the parent row's visibility toggle off
- **THEN** the group enable flag is cleared and the composite falls back to the unfiltered source

#### Scenario: A toggle survives a write and re-read

- **WHEN** `pictura-app::toggles_persist_on_a_preserved_smart_object` writes and reads back a filtered document whose filter or group visibility was toggled
- **THEN** the re-read smart object reports the toggled flag

#### Scenario: Synthetic rows are not real layers

- **WHEN** `tst_smart_filter_apply::doubleClickReopensTheFilterDialog` double-clicks a smart-filter row
- **THEN** the row is not renamed, no layer is added or moved, and the filter's dialog opens instead

#### Scenario: Dragging a filter row reorders the chain [lp_sf_drag]

- **WHEN** `tst_smart_filter_apply::dragReordersSmartFilters` drags the lower of two filter rows above the upper one, then undoes, then drops a filter row on a layer row
- **THEN** the dragged filter moves to the top row and applies last, undo restores the order, and the drop on a layer row is refused


### Requirement: Layer row surface and two-column layout

Each layer row's background SHALL be one shade step lighter than the list/base
background. Each layer row SHALL be split into two columns — a fixed visibility-toggle
column and a content column holding the rest of the row — so a click in the
visibility column toggles visibility and never selects or highlights the row.
A synthetic effect or filter row SHALL sit inside the layer's content column:
its eye aligns with the owning layer's image-thumbnail right edge, each child
row sits one indent step deeper, and the
fixed left column continues as the layer's spanning left cell, which is not a
toggle target for that synthetic row. The
selected-row highlight SHALL be a lighter grey painted directly by the row
delegate rather than the palette's blue highlight, and the delegate SHALL be the
single source of that selection colour with no scoped stylesheet selection rule
and no style-drawn focus tint for the tree view; a focused selected row SHALL
match an unfocused one. The content column SHALL carry a 2 px left padding so the
thumbnail and name are not flush against the visibility gutter; a synthetic
row's content begins after its aligned eye. The visibility
column's flat lighter fill SHALL take the row's selection surface when the row
is selected, and its 1 px gutter separator SHALL remain unchanged by selection;
the gutter separator SHALL stay 1 px.

#### Scenario: The row background steps lighter than the list [lpr_row_surface]

- **WHEN** a layer row is shown against the list background
- **THEN** the row background is one shade step lighter than the list background

#### Scenario: The visibility toggle cannot be highlighted [lpr_eye_no_select]

- **WHEN** a click lands in a layer row's visibility-toggle column
- **THEN** visibility toggles and the row is neither selected nor highlighted

#### Scenario: Selection uses a lighter grey [lpr_selection_grey]

- **WHEN** one or more layer rows are selected
- **THEN** the row delegate paints the selected-row background as a lighter grey
  rather than the blue highlight, with no separate stylesheet rule or focus tint
  overriding it

#### Scenario: A focused selected row keeps the flat highlight [lpr_selection_focus]

- **WHEN** a selected layer row's tree gains focus and then loses it
- **THEN** the row uses the same flat grey highlight in both states, with no
  focus tint or gradient

#### Scenario: The content column has a 2 px left padding [lpr_content_padding]

- **WHEN** a layer row is shown
- **THEN** the content column begins 2 px to the right of the visibility gutter,
  with the gutter separator still 1 px


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
Lock strip); an unlocked layer SHALL show none. The lock badge's side SHALL be
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
the owning layer's image-thumbnail right edge, each child row sits one indent
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
  thumbnail and name are indented; a synthetic effect or filter row instead
  aligns its eye with the owning layer's image-thumbnail right edge, and each
  child row sits one indent step deeper

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

- **WHEN** an adjustment or type row with no raster image thumbnail is shown
- **THEN** its card carries no 1 px black outline, while a pixel or fill row
  with a raster image thumbnail keeps its outline; a synthetic Smart Filters or
  layer-effects row draws no card at all


### Requirement: Layer effects disclosure rows

The Layers panel SHALL list a styled layer's effects nested under its row: one
synthetic **Effects** group row at the layer's depth plus one, and one synthetic
child row per effect the layer currently carries at the layer's depth plus two,
named with the effect's CS6 title in the layer-style list order (Bevel & Emboss
through Drop Shadow). The layer row SHALL expose the group through its single
right-edge list toggle beside its `fx` badge: the toggle points down when the
combined list is closed and up when it is open, closing it SHALL hide every
synthetic group on the layer, and clicking the `fx` badge SHALL also toggle the
combined list. The **Effects** group row SHALL carry an eye that toggles the
layer's master effect switch (`fx.visible`) and SHALL NOT carry its own
disclosure; each effect row SHALL carry an eye that toggles that effect's
enabled state (`<effect>.on`), while the effect stays listed so it can be
switched back on. The synthetic rows SHALL draw no `fx` thumbnail; each row's
eye SHALL align with the owning layer's image-thumbnail right edge with no
depth indent, and a hidden row SHALL paint the eye-off glyph at 50% opacity
while keeping the eye slot clickable. Clicking a synthetic row outside its eye
SHALL select the owning layer. The layer's left eye cell and right cell SHALL
span the layer + list block, and no horizontal rule SHALL be drawn between the
nested rows or between the Effects and Smart Filters groups. A newly styled
layer SHALL start expanded when the panel option **Expand New Effects** is on,
and a layer with no effects SHALL show no group row. Each eye toggle SHALL be
one undoable history state. A synthetic effects row SHALL NOT be renamed,
dragged, dropped on, or made the document's layer target.

#### Scenario: A styled layer shows its effects nested [lpr_fx_rows]

- **WHEN** a pixel layer carries a drop shadow and is shown in the panel
- **THEN** its row is followed by a synthetic `Effects` group row and a `Drop
  Shadow` effect row at the next two depths

#### Scenario: The Effects eye toggles the master switch [lpr_fx_group_eye]

- **WHEN** the `Effects` group row's eye is clicked while the layer's effects are shown
- **THEN** `fx.visible` becomes false, the composite shows the layer unstyled,
  and the row's eye reads hidden

#### Scenario: An effect eye toggles that effect [lpr_fx_effect_eye]

- **WHEN** a `Drop Shadow` effect row's eye is clicked
- **THEN** `dropShadow.on` flips and the composite changes, while the effect row
  stays listed

#### Scenario: A new style starts expanded [lpr_fx_expand_new]

- **WHEN** a layer with no style gains one while **Expand New Effects** is on
- **THEN** the layer's `Effects` group and its effect rows are shown expanded

#### Scenario: An unstyled layer shows no group [lpr_fx_no_group]

- **WHEN** a layer that carries no layer style is shown
- **THEN** no `Effects` row follows it

