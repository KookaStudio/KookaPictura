# ui/character-paragraph-panels Specification

## Purpose
Provides the Character and Paragraph panels as live attribute editors over the type model, and wires them into the Window and Type menus so they can be shown and hidden like the other panels.

## Requirements

### Requirement: Character panel edits the active type layer

The Character panel SHALL show the active type layer's character attributes —
font family, size, leading, kerning, tracking, horizontal and vertical scale,
baseline shift, anti-aliasing method, fill colour, the Faux Bold, Faux Italic,
caps, super/subscript, underline, and strikethrough toggles, and the OpenType
features — and SHALL apply an edit to that layer through the type bridge,
re-rendering it and recording exactly one history state.
When no type layer is active it SHALL present the model defaults and SHALL NOT
record a state.

#### Scenario: Panel reflects the active type layer

- **WHEN** a type layer is selected
- **THEN** the Character panel shows that layer's size, leading, tracking, scale, anti-aliasing, colour, and toggles

#### Scenario: Editing an attribute updates the layer as one state

- **WHEN** the size field is committed on the active type layer and then undone once
- **THEN** the layer's rendered text reflects the new size and the single undo restores the previous size

#### Scenario: No type layer shows the defaults

- **WHEN** no type layer is active
- **THEN** the panel shows the default character attributes and committing a value records no history state

### Requirement: Paragraph panel edits the active type layer

The Paragraph panel SHALL show the active type layer's paragraph attributes —
alignment and justification, left/right/first-line indents, space before and
after, hanging punctuation, hyphenation, and composer — and SHALL apply an edit
to that layer through the type bridge, re-rendering it and recording exactly one
history state.

#### Scenario: Panel reflects the active type layer

- **WHEN** a type layer is selected
- **THEN** the Paragraph panel shows its alignment, indents, paragraph spacing, hanging punctuation, hyphenation, and composer

#### Scenario: Editing a paragraph attribute updates the layer

- **WHEN** the first-line indent is committed and then undone once
- **THEN** the layer's stored paragraph attributes reflect the change and the single undo restores the previous value

### Requirement: Window and Type menus toggle the panels

`Window > Panels > Character`/`Paragraph` and `Type > Panels > Character`/
`Paragraph` SHALL be enabled checkable commands. Checking one SHALL show the
panel in the panel column, unchecking SHALL hide it, and the checked state SHALL
reflect the panel's actual visibility.

#### Scenario: Toggling shows and hides the panel

- **WHEN** `Window > Panels > Character` is checked and then unchecked
- **THEN** the Character panel becomes visible and then hidden, and the menu item's checked state follows

#### Scenario: Menu state reflects visibility

- **WHEN** a panel is shown or hidden by any means
- **THEN** its Window and Type menu entries report the matching checked state

### Requirement: The type options bar opens the panels

The type options bar's Panel button SHALL toggle the Character and Paragraph
panels, matching CS6.

#### Scenario: Options-bar button toggles the type panels

- **WHEN** the type options bar's Panel button is pressed
- **THEN** the Character and Paragraph panels toggle visible or hidden

### Requirement: Character panel follows the CS6 layout

The Character panel SHALL lay out its controls as CS6 does:

- the font family;
- icon-labelled size and leading, kerning and tracking, vertical and
  horizontal scale, and baseline shift with the Color swatch, in two columns;
- a row of eight checkable style buttons: Faux Bold, Faux Italic, All Caps,
  Small Caps, Superscript, Subscript, Underline, Strikethrough;
- a row of eight checkable OpenType buttons: Standard Ligatures, Contextual
  Alternates, Discretionary Ligatures, Swash, Stylistic Alternates, Titling
  Alternates, Ordinals, Fractions;
- the anti-aliasing method.

The icons SHALL be drawn by the application, not copied from Adobe artwork.
All Caps and Small Caps SHALL be mutually exclusive buttons, as SHALL
Superscript and Subscript. Each button press SHALL be one history state.

#### Scenario: A style button applies its toggle

- **WHEN** the Faux Bold button is pressed on the active type layer
- **THEN** the layer's Faux Bold is on, one history state is recorded, and the layer re-renders

#### Scenario: Exclusive caps buttons

- **WHEN** All Caps is on and Small Caps is pressed
- **THEN** Small Caps is on and All Caps is off on the layer

#### Scenario: Exclusive position buttons

- **WHEN** Superscript is on and Subscript is pressed
- **THEN** Subscript is on and Superscript is off on the layer

#### Scenario: An OpenType button applies its feature

- **WHEN** an enabled OpenType button is pressed on the active type layer
- **THEN** the layer's matching OpenType attribute flips and one history state is recorded

### Requirement: OpenType buttons follow the font

An OpenType button SHALL be enabled only while a type layer is active and the
layer's font carries the button's feature (per the renderer's support query).
A disabled button SHALL keep showing the layer's stored value.

#### Scenario: Buttons grey out for features the font lacks

- **WHEN** a type layer whose font lacks some OpenType features is active
- **THEN** exactly the buttons for the features the font carries are enabled

#### Scenario: No type layer disables every OpenType button

- **WHEN** no type layer is active
- **THEN** every OpenType button is disabled

### Requirement: CS6 kerning and leading fields

The kerning field SHALL be an editable list offering `Metrics`, `Optical`, and
the CS6 manual presets −100, −75, −50, −25, −10, −5, 0, 5, 10, 25, 50, 75, 100,
200. It SHALL always be editable while a type layer is active. Picking
`Metrics` or `Optical` SHALL set that kerning mode, and picking or typing a
number SHALL set Manual kerning with that value. The leading field SHALL offer
`(Auto)` and the size presets; a number sets Fixed leading. The size and
tracking fields SHALL be editable lists of the CS6 presets.

#### Scenario: Picking Optical sets optical kerning

- **WHEN** `Optical` is picked in the kerning field
- **THEN** the layer's kerning mode is Optical

#### Scenario: Typing a number sets manual kerning

- **WHEN** `50` is typed into the kerning field and committed
- **THEN** the layer's kerning is Manual with value 50

#### Scenario: Picking Auto leading

- **WHEN** a fixed leading is shown and `(Auto)` is picked
- **THEN** the layer's leading is Auto

### Requirement: Numeric fields scrub

Every numeric Character panel field SHALL change its value when the pointer is
dragged horizontally on its icon or value: size, leading, kerning, tracking,
vertical scale, horizontal scale, and baseline shift. Dragging right SHALL
increase the value by one step per pixel, Shift SHALL scale the step by 10, and
Ctrl SHALL scale it by 1/10. The result SHALL be clamped to the field's range.
A drag SHALL record exactly one history state on release. Scrubbing kerning or
leading from `Metrics`, `Optical`, or `(Auto)` SHALL start from 0 or the
current auto value respectively and switch to the manual or fixed mode.

#### Scenario: Dragging an icon right raises the value

- **WHEN** the horizontal-scale icon is dragged 20 px to the right from 100%
- **THEN** the field and the layer read 120% and one history state is recorded

#### Scenario: Scrubbing kerning leaves Metrics

- **WHEN** the kerning icon is dragged 10 px to the right while the field shows `Metrics`
- **THEN** the layer's kerning is Manual with value 10
