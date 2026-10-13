# Spec Delta

## MODIFIED Requirements

### Requirement: Layer property editing

The system SHALL apply visibility, name, blend mode, opacity, fill, lock, and
color-label edits to the active document through the bridge, addressed by the
layer's path (a `/`-separated sequence of bottom-first child indices). Every
edit SHALL mark the document modified and be undoable. A blend-mode edit SHALL
record one undo state whose label is `Blending Change: <mode name>`, using the
mode's CS6 display name (for example `Blending Change: Color Burn`). When more
than one layer is selected, a header edit SHALL apply to every selected layer in
a single undo step, and a layer that the CS6 contract excludes SHALL be skipped
rather than fail the edit. The system SHALL report and edit opacity and fill
through the bridge as a value in `0..=255`, where `255` is fully opaque; the
panel SHALL present both as percentages per the Opacity and Fill percent
controls requirement. A layer's lock state SHALL be reported and edited per flag
(transparency, image pixels, position, nesting, or all). A layer's color label
SHALL be one of `None`, `Red`, `Orange`, `Yellow`, `Green`, `Blue`, `Violet`, or
`Gray`. The system SHALL refuse, rather than apply, an edit that the CS6
contract excludes: a group has no Fill control, and the Background layer SHALL
expose neither opacity nor fill and SHALL refuse lock and color-label edits. A
fully locked layer SHALL refuse opacity and fill edits; its lock flags SHALL
remain editable so it can be unlocked. A refused edit SHALL leave that layer
unchanged.

#### Scenario: Change opacity

- **WHEN** the user changes a layer's opacity in the panel
- **THEN** the document's layer opacity changes, the composite updates, and the document reports modified

#### Scenario: Change blend mode

- **WHEN** the user selects a blend mode for a layer
- **THEN** the layer's blend mode changes and the composite updates

#### Scenario: A blend change records its mode name [lpr_blend_history]

- **WHEN** a layer's blend mode is set to Color Burn
- **THEN** one undo state is recorded labelled `Blending Change: Color Burn`

#### Scenario: Rename a layer [m39_rename]

- **WHEN** the user edits a layer's name
- **THEN** the layer name changes and the tab title reflects an untitled-name change where applicable

#### Scenario: Change fill opacity

- **WHEN** the user changes a layer's fill opacity to 128 in the panel
- **THEN** the layer's fill becomes 128, the composite updates with the layer's pixels at half strength, and the edit is undoable as one step

#### Scenario: Toggle a lock flag

- **WHEN** the user enables Lock Position for a layer and then enables Lock All
- **THEN** the layer reports the position bit and then all four lock bits, the panel disables opacity and fill, and each toggle is one undo state

#### Scenario: Set a color label

- **WHEN** the user picks Red from the row's color-label menu
- **THEN** the layer's color label becomes Red and the row reflects it

#### Scenario: Fill is unavailable for a group

- **WHEN** a group is selected
- **THEN** the Fill control is disabled and a fill edit is refused, while Opacity remains editable

#### Scenario: Background and fully locked layers refuse edits

- **WHEN** the Background layer or a fully locked layer is selected and opacity or fill is changed
- **THEN** the edit is refused, the document is unchanged, and the control is disabled

#### Scenario: A multi-selection edit applies to the eligible layers in one step [m39_multi]

- **WHEN** several layers are selected and a blend mode is chosen
- **THEN** every selected layer that is not the Background is changed, an
  ineligible layer is left unchanged, and the whole edit is one undo step

#### Scenario: A nested layer is edited by path [m39_tree]

- **WHEN** a layer inside a group is renamed
- **THEN** only that nested layer changes, addressed by its path, and the
  operation is one undo step

### Requirement: Layer row badges and delegate

The system SHALL draw each row with a delegate that paints, in CS6 order, the
visibility toggle, the thumbnail (a folder glyph for a group), the name, a
clipping-mask indicator for a clipped layer, the clipping indentation and base
underline, the layer-mask thumbnail when a mask is present, an adjustment badge
when adjustment content is present, a layer-style `fx` badge when the layer
carries a layer style (an `lfx2` or legacy `lrFX` block), and a compact
**`Blend If` text chip** when the layer's advanced blending is customised — a
non-default `Blend If` view, or a raw `blending_ranges` block present without a
typed view. The chip SHALL be a text-only badge (no icon asset) drawn in the
row's right-edge badge run left of the `fx` and lock badges, and its advance
SHALL be reserved by the same right-edge walk that lays out the mask thumbnails
and the name rect so nothing overlaps. A layer whose lock
state has any flag set SHALL also show a lock badge at the right side of its
row; an unlocked layer SHALL show none. The visibility toggle SHALL be an eye
icon (`layers.eyeOn`/`layers.eyeOff`) drawn slightly inset from the panel's left
edge and at the same x for every row, independent of nesting depth; the nesting
indentation SHALL apply to the thumbnail and name, not to the visibility toggle.
A layer whose color label is not `None` SHALL tint the **visibility toggle's own
background** (`eyeRect`) with that label color behind the eye glyph, and SHALL
additionally paint a small color-label **chip** — a short bar of the label
color, one row tall — at the start of the row content immediately before the
thumbnail, with the thumbnail and name laid out after the chip so no rect
overlaps; the tint SHALL keep the eye glyph and any selection highlight legible,
and the label color elsewhere on the row SHALL fall back to the row background.
The chip is a paint-only affordance and SHALL NOT alter hit-testing beyond the
thumbnail and name rects accounting for it. A group with at least one child
SHALL show a disclosure icon — right when collapsed, down when expanded — at its
indented position, and clicking that icon SHALL expand or collapse the group. A
**regular (non-group) layer's thumbnail SHALL be drawn over a cached two-tone
checkerboard** so transparency reads under it, while a group keeps its folder
glyph and no checkerboard. The thumbnail SHALL be laid out in a box `thumb` wide
and the row's full height minus a small pad, and SHALL be letterboxed to the
document's aspect ratio inside that box, so a tall document's thumbnail fills the
row vertically rather than floating in a square box. **Every non-group thumbnail
SHALL carry a 1 px black outline**; a group's folder glyph SHALL draw no
outline, and **when exactly one layer is active** (the same singular
active-layer resolution tool edits use) its thumbnail SHALL additionally show
white 1 px corner brackets drawn one pixel outside the outline; with zero or
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

#### Scenario: The color label tints only the eye toggle [lpr_label_tint]

- **WHEN** a layer with a non-`None` color label is shown
- **THEN** the eye toggle's background is filled with that label color behind
  the eye glyph and a chip of the same color is painted at the row content edge
  before the thumbnail, and the rest of the row background is unchanged

#### Scenario: The label chip sits before the thumbnail [lpr_label_chip]

- **WHEN** a labeled row is shown
- **THEN** the chip is drawn between the disclosure/clipping glyph and the
  thumbnail, and the thumbnail's left edge follows the chip so the two do not
  overlap

#### Scenario: An unlabeled layer has no gutter tint [lpr_label_tint_none]

- **WHEN** a layer with color label `None` is shown
- **THEN** its eye gutter uses the normal row background and no chip is painted

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

- **WHEN** a layer that carries a layer style is shown
- **THEN** its row draws the `layers.fx` badge at its right edge beside the lock
  and mask badges, and a layer with no style draws no fx badge

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

### Requirement: Layers panel header layout

The panel SHALL stack its header controls in this order, top to bottom: the
filter/search row; a row containing the blend-mode popup and a labeled Opacity
field; a row containing the five lock toggles and a labeled Fill field; the
layer list; and the action strip. The Opacity and Fill fields SHALL each carry a
leading text label, and each field's popup slider SHALL open centred
horizontally under the field rather than aligned to its right edge. The Opacity
field's label SHALL be right-aligned against its value box; the Fill field's
label SHALL keep its left alignment. The panel SHALL NOT provide its own menu
button; the wired Layer commands SHALL remain reachable from the panel-group
widget menu. The five lock toggles SHALL all sit in the single lock row above
the layer list, not below it.

#### Scenario: The header order is filter / blend+Opacity / locks+Fill / list [lpc_order]

- **WHEN** the Layers panel is shown
- **THEN** the filter row is above the blend and Opacity row, which is above the
  lock and Fill row, which is above the layer list, with the action strip below
  the list

#### Scenario: Opacity and Fill are labeled [lpc_labels]

- **WHEN** the Layers panel is shown
- **THEN** the Opacity field and the Fill field each have a text label directly
  to their left

#### Scenario: The Opacity label is right-aligned [lpc_opacity_label_right]

- **WHEN** the Layers panel header is laid out
- **THEN** the Opacity field's label text is right-aligned against its value box
  and the Fill field's label stays left-aligned

#### Scenario: The percent picker opens centered [lpc_popup]

- **WHEN** the user opens the Opacity or Fill slider popup
- **THEN** the popup is centered horizontally under its field

#### Scenario: The panel has no separate menu button [lpc_nomenu]

- **WHEN** the Layers panel header is inspected
- **THEN** it contains no panel-menu button, and the Layer commands are offered
  by the panel-group widget menu

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
SHALL NOT draw brackets around its icon.

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

### Requirement: Layers panel folder rows

A folder row SHALL be vertically shorter than a layer row, its folder icon SHALL
be about three quarters of the row thumbnail size, and its right column SHALL use
less left-side padding without changing child-layer visibility controls. A group
row SHALL show its expand/collapse chevron even when it has no children.

#### Scenario: Empty group is expandable [uip_layers_folder_chevron]

- **WHEN** a group with no children is shown
- **THEN** the row shows its chevron and can be expanded

#### Scenario: The folder glyph is near thumbnail size [uip_layers_folder_icon]

- **WHEN** a group row is shown
- **THEN** its folder glyph is about three quarters of the row thumbnail size,
  larger than half

## ADDED Requirements

### Requirement: Layers panel combo affordances

The Layers panel's blend-mode and layer-filter combo boxes SHALL draw a
drop-down caret — a small down-pointing triangle in the UI text colour, matching
the design system's `.combo .caret` — so their selectable nature is visible, and
SHALL cycle their options with the Up and Down arrow keys while the field holds
keyboard focus. A value change SHALL NOT drop the field's focus, so repeated
arrow keys keep cycling until focus moves elsewhere.

#### Scenario: A combo shows a drop-down caret [lpr_combo_arrow]

- **WHEN** a Layers panel combo box is shown
- **THEN** it draws a small down-pointing caret at its right edge

#### Scenario: Arrow keys cycle the focused combo [lpr_combo_arrows]

- **WHEN** a Layers panel combo box has keyboard focus and Down is pressed
- **THEN** its current option advances by one, and Up moves it back

#### Scenario: The field keeps focus across a change [lpr_combo_focus]

- **WHEN** a focused combo changes its value (its blend edit refreshes the panel)
- **THEN** the field still holds focus and the next arrow key cycles it again

### Requirement: Group expansion survives a structural change

A group's expanded/collapsed state SHALL survive a structural change that shifts
the group's index-based path — adding, removing, or moving a layer, or dropping a
layer into the group. The panel SHALL re-find the group by its ancestor name
chain, disambiguating same-named siblings by their order among them, and keep it
expanded or collapsed as before. A rename, which leaves the index path intact,
SHALL keep the state through the unchanged path. A refused operation SHALL change
neither the document nor the expansion.

#### Scenario: Adding a layer outside an expanded group keeps it open [lpr_add_keeps_expanded]

- **WHEN** a group is expanded and a layer is added elsewhere in the document so
  the group's index-based path shifts
- **THEN** the group stays expanded with its children visible

#### Scenario: A rename keeps the group expanded [lpr_rename_keeps_expanded]

- **WHEN** an expanded group is renamed in place
- **THEN** it stays expanded, because the rename leaves its index path intact

#### Scenario: Same-named sibling groups stay distinct [lpr_dup_group_names]

- **WHEN** two sibling groups share a name, one is expanded, and a structural
  change shifts their paths
- **THEN** the group that was expanded is the one that stays expanded

#### Scenario: A drop into an open group keeps it open [lpr_drop_into_expanded]

- **WHEN** an open group is a valid drop-into target and a layer is dropped onto it
- **THEN** the move succeeds as one undo step and the group stays expanded with
  the dropped layer visible as its child

