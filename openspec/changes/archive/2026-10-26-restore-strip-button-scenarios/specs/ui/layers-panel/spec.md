## MODIFIED Requirements

### Requirement: Layers panel action strip buttons

The system SHALL give each Layers-panel action-strip button an icon from the
frozen layers asset set: `layers.link` (Link Layers), `layers.fx` (Layer Style),
`layers.mask` (Add Layer Mask), `layers.fillAdjustment` (New Fill / Adjustment
Layer), `layers.group` (New Group), `layers.newLayer` (New Layer), and
`layers.delete` (Delete). The buttons SHALL keep their text labels, and adding
an icon SHALL NOT change what each button does. The **Add Layer Mask** button
SHALL be active: a click SHALL add a `reveal-selection` mask when a selection
exists and a `reveal-all` mask otherwise, and an `Alt`-click SHALL add a
`hide-all` mask, each through the layer-mask bridge and each one undoable step.
The **Layer Style** (`fx`) button SHALL be active: a click SHALL open the Layer
Style dialog on Blending Options for the active layer when that layer can carry
a style, and an `Alt`-click SHALL toggle all effects through the layer-style
bridge in one undoable step. The **Link Layers** button SHALL be active: with a
layer selection a click SHALL link the selected layers into one link set, or
unlink them when they already share a set, through the link bridge, and its icon
SHALL be the link glyph when the selection is not all linked and the unlink
glyph when it is. When no document is open, every action-strip button SHALL be
disabled; the Link button SHALL also be disabled without a layer selection.

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

#### Scenario: The mask button adds a mask [lmk_strip]

- **WHEN** the user clicks the Add Layer Mask strip button with no selection
- **THEN** a reveal-all mask is added to the active layer in one undoable step

#### Scenario: The mask button respects a selection [lmk_strip_sel]

- **WHEN** the user clicks the Add Layer Mask strip button while a selection exists
- **THEN** a reveal-selection mask is added to the active layer in one undoable step

#### Scenario: Alt-clicking the mask button hides all [lmk_strip_alt]

- **WHEN** the user `Alt`-clicks the Add Layer Mask strip button
- **THEN** a hide-all mask is added to the active layer in one undoable step

#### Scenario: The fx button opens Blending Options

- **WHEN** the user clicks the Layer Style strip button with an active layer that can carry a style
- **THEN** the Layer Style dialog opens on Blending Options for that layer

#### Scenario: Alt-clicking the fx button toggles all effects

- **WHEN** the user `Alt`-clicks the Layer Style strip button
- **THEN** every layer's effects are shown when any were hidden and hidden when any were shown, as one undoable step
