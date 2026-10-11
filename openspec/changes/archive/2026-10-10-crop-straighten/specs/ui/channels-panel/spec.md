# Spec Delta

## ADDED Requirements

### Requirement: Channels panel affordances

A channel item SHALL show a pointing-hand cursor while hovered. Holding Ctrl
while over a channel item SHALL show a dashed-square symbol at the cursor,
indicating "select all pixels of this channel". The channel table rows SHALL use
the same surface and item styling as the Layers panel.

#### Scenario: Hover shows the hand cursor [uic_channel_hover_cursor]

- **WHEN** the pointer hovers a channel item
- **THEN** the cursor is the pointing hand

#### Scenario: Ctrl shows the select-all symbol [uic_channel_ctrl_symbol]

- **WHEN** Ctrl is held while the pointer is over a channel item
- **THEN** a dashed-square symbol is drawn at the pointer

#### Scenario: Rows match the Layers panel style [uic_channel_row_style]

- **WHEN** the Channels panel is shown
- **THEN** its rows use the same surface shading and item layout as the Layers panel
