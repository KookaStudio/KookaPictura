## REMOVED Requirements

### Requirement: Layers panel action strip icons

**Reason**: The Link Layers button is no longer deferred; it is active with a
layer selection. The requirement is re-added as `Layers panel action strip
buttons` with the Link button's behaviour; the dropped "deferred button"
scenario no longer holds.

**Migration**: The mask button's behaviour stays specified in
`ui/layers-mask-commands` and the fx button's in `ui/layer-style-dialog` and the
`fx` badge requirement in `ui/layers-panel`. The other strip buttons and their
behaviour are unchanged.

## ADDED Requirements

### Requirement: Layers panel action strip buttons

The system SHALL give each Layers-panel action-strip button an icon from the
frozen layers asset set: `layers.link` (Link Layers), `layers.fx` (Layer Style),
`layers.mask` (Add Layer Mask), `layers.fillAdjustment` (New Fill / Adjustment
Layer), `layers.group` (New Group), `layers.newLayer` (New Layer), and
`layers.delete` (Delete). The buttons SHALL keep their text labels, and adding
an icon SHALL NOT change what each button does. The **Add Layer Mask** button
SHALL be active (see `ui/layers-mask-commands`). The **Layer Style** (`fx`)
button SHALL be active: a click SHALL open the Layer Style dialog on Blending
Options for the active layer when that layer can carry a style, and an
`Alt`-click SHALL toggle all effects through the layer-style bridge in one
undoable step. The **Link Layers** button SHALL be active: with a layer
selection a click SHALL link the selected layers into one link set, or unlink
them when they already share a set, through the link bridge, and its icon SHALL
be the link glyph when the selection is not all linked and the unlink glyph when
it is. When no document is open, every action-strip button SHALL be disabled;
the Link button SHALL also be disabled without a layer selection.

#### Scenario: The implemented strip buttons carry icons

- **WHEN** the Layers panel is shown
- **THEN** its link, mask, fx, fill/adjustment, group, new-layer, and delete buttons each carry their documented icon

#### Scenario: The link button toggles the selection's link state [lpl_strip_link]

- **WHEN** the user clicks the Link Layers strip button with two layers selected
- **THEN** the two layers share one link set and the button shows the unlink glyph
- **AND** clicking it again removes them from the set

#### Scenario: No document disables the strip [lpc_strip_no_document]

- **WHEN** no document is open
- **THEN** the Link, fx, Mask, New Fill/Adjustment, New Group, New Layer, and
  Delete buttons are disabled

### Requirement: Linked-layer row indicator

The Layers-panel row delegate SHALL draw a chain link glyph on the right side of
every row whose layer is a member of a link set, using the `layers.link2` asset,
in the row's right-edge badge run. The glyph's advance SHALL be reserved by the
same right-edge walk that lays out the fx, lock, and Blend If badges and the name
rect, so neither the name nor the mask/vector thumbnails overlap it. A row whose
layer is not linked SHALL draw no link glyph, and the glyph SHALL be omitted
when the asset is unavailable.

#### Scenario: A linked row shows the chain glyph [lpl_row_link]

- **WHEN** a row whose layer is a member of a link set is shown
- **THEN** a chain link glyph is drawn in its right-edge badge run

#### Scenario: An unlinked row shows no chain glyph [lpl_row_link_absent]

- **WHEN** a row whose layer is not linked is shown
- **THEN** no chain link glyph is drawn
