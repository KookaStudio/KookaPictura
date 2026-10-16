## ADDED Requirements

### Requirement: Filter Gallery dialog

`Filter ▸ Filter Gallery…` SHALL open a dialog with a zoomable preview, the gallery filters as thumbnails grouped under collapsible category headers (Artistic, Brush Strokes, Distort, Sketch, Stylize, Texture, in CS6 order, each listing only filters with an engine kernel), a toggle that hides the thumbnails, OK and Cancel, a menu of every gallery filter, the selected effect's options, and an effect-layer list. The entry SHALL be enabled exactly when the active layer can take a filter.

#### Scenario: Categories follow CS6

- **WHEN** the gallery lists its categories
- **THEN** Artistic has 15 filters, Brush Strokes 8, Sketch 14, Stylize only Glowing Edges, and Texture 6

#### Scenario: Thumbnails show the picture filtered

- **WHEN** the gallery opens
- **THEN** each thumbnail is rendered from the picture through its filter

### Requirement: Filter Gallery effect stack

The gallery SHALL hold a stack of effects applied in order, shown top-down as last-applied first. Clicking a thumbnail or choosing from the menu SHALL make the selected effect that filter on its defaults. New effect layer SHALL insert a copy of the selected effect above it; Delete SHALL remove the selected effect but never the last one; the eye SHALL hide an effect from the preview and the commit, and SHALL be the row's check state, so Space toggles it from the keyboard and assistive technology reads it; dragging SHALL reorder. The stack SHALL preview in the dialog as it changes (see `imaging/filter-gallery-preview`), OK SHALL commit the visible effects as one "Filter Gallery" history state, and Cancel SHALL leave the layer bit-identical. The gallery SHALL reopen on the session's last stack.

#### Scenario: OK commits the stack as one state

- **WHEN** a two-effect stack is committed
- **THEN** the layer changes and the history gains exactly one state

#### Scenario: Cancel restores the layer

- **WHEN** the gallery previews a stack and is cancelled
- **THEN** the canvas is pixel-identical to before it opened

#### Scenario: Hidden effects are not applied

- **WHEN** every effect is hidden and OK is pressed
- **THEN** nothing is committed, the layer is unchanged, and no refusal is reported

#### Scenario: Space toggles the eye

- **WHEN** an effect row has focus and Space is pressed
- **THEN** the effect is hidden and its row unchecked, and a second Space shows it again
