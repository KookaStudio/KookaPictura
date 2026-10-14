# Spec Delta

## ADDED Requirements

### Requirement: Inset slider-popup arrow

The numeric field's slider-popup arrow SHALL be drawn inside the value input, not
as a sibling outside its frame. The arrow SHALL be a child of the value text
editor, placed flush to its inner trailing edge and sized to the editor's inner
height, so the value box, its hairline separator, and the arrow read as one
control. Opening the popup SHALL mark the arrow active (a darker surface) and
closing it SHALL return the arrow to rest. The field's reserved text width SHALL
account for the arrow so the value never runs under it.

#### Scenario: The arrow is part of the input [ds_arrow_inset]

- **WHEN** a numeric field with a slider popup is laid out
- **THEN** the arrow's parent is the value text editor and the arrow's geometry
  lies inside the editor's rect, with the editor's trailing text margin at least
  the arrow's width

#### Scenario: The arrow is active while the popup is open [ds_arrow_active]

- **WHEN** the popup is opened from the arrow and then hidden
- **THEN** the arrow is active (styled darker) while the popup is shown and at
  rest once it is hidden

### Requirement: Input-family consistency

Every text input, editable input, and select SHALL share one surface definition:
the same background token, the same border token, the same focus treatment, the
same disabled treatment, and the same rendered height, declared once in the
global stylesheet. The numeric field, the editable combo box's inner editor, and
the options-bar workspace switcher SHALL resolve to that same definition. Input, editable-input, and select
text SHALL use a single shared size one pixel under the application default, so
no input family member disagrees. The down/up arrow SHALL be a single Lucide
chevron. The numeric field's arrow and spin-box arrows SHALL be tinted at render
time; the select's drop-down chevron uses the same glyph in the family's
foreground colour, because the stylesheet engine cannot tint an SVG and the
foreground is constant across the brightness levels. The select's drop-down cell
SHALL be flat and SHALL reserve the value's trailing space so the chevron never
overlaps the value. Spin-box arrows SHALL use the same chevron.

#### Scenario: One input surface [ds_input_family_surface]

- **WHEN** a line edit, spin box, combo box, or editable combo box is shown at
  rest, hovered, focused, or disabled
- **THEN** its state styling resolves to the shared input tokens, and the
  editable combo's inner editor adds no second frame

#### Scenario: One shared input text size [ds_input_family_font]

- **WHEN** the global stylesheet is built
- **THEN** every input, editable input, and select rule uses the same font size,
  one pixel under the application default

#### Scenario: One shared input height [ds_input_family_height]

- **WHEN** a line edit, combo box, and spin box are shown together
- **THEN** they render at the same height, the spin box's button floor carried to
  the other inputs

#### Scenario: The select and workspace switcher match [ds_input_family_select]

- **WHEN** a combo box and the workspace switcher are shown
- **THEN** both draw the shared input surface, a flat drop-down cell that reserves
  the value's trailing space, and the same Lucide chevron in the family
  foreground colour

#### Scenario: The chevron follows the family [ds_input_family_chevron]

- **WHEN** the theme family changes
- **THEN** every select and spin-box arrow is drawn in the new family's
  foreground colour, up and down pointing as the control requires

### Requirement: Checkbox tick

A checked checkbox SHALL show a tick mark inside the box, not a bare fill. The
tick SHALL be drawn in the checked box's legible foreground colour and SHALL
remain visible in the checked hover and disabled states.

#### Scenario: The checked box shows a tick [ds_checkbox_tick]

- **WHEN** a checkbox is checked
- **THEN** the indicator draws a tick mark over the highlight fill

