# Spec Delta — ui/channels-panel

## MODIFIED Requirements

### Requirement: Channels panel affordances

A channel item SHALL show a pointing-hand cursor while hovered. Holding Ctrl
while over a channel item SHALL show a dashed-square symbol at the cursor,
indicating "select all pixels of this channel". The channel table rows SHALL use
the same surface and item styling as the Layers panel, including the
family-aware row, gutter, and selected-row steps, and SHALL re-style when the
theme family changes so the selection stays distinct in either family.

#### Scenario: Hover shows the hand cursor [uic_channel_hover_cursor]

- **WHEN** the pointer hovers a channel item
- **THEN** the cursor is the pointing hand

#### Scenario: Ctrl shows the select-all symbol [uic_channel_ctrl_symbol]

- **WHEN** Ctrl is held while the pointer is over a channel item
- **THEN** a dashed-square symbol is drawn at the pointer

#### Scenario: Rows match the Layers panel style [uic_channel_row_style]

- **WHEN** the Channels panel is shown
- **THEN** its rows use the same surface shading and item layout as the Layers
  panel

#### Scenario: Channel rows follow the light family [cp_light_row_contrast]

- **WHEN** the light family is applied with a channel row selected
- **THEN** the selected row surface is darker than the unselected rows and the
  channel name stays legible
