# ui/character-paragraph-panels Specification

## Purpose
Provides the Character and Paragraph panels as live attribute editors over the type model, and wires them into the Window and Type menus so they can be shown and hidden like the other panels.

## Requirements

### Requirement: Character panel edits the active type layer

The Character panel SHALL show the active type layer's character attributes —
font family, font style, size, language, leading, kerning, tracking, horizontal
and vertical scale, baseline shift, anti-aliasing method, fill colour, tsume,
the layer-wide Fractional Widths setting, the Faux Bold, Faux Italic, caps,
super/subscript, underline, and strikethrough toggles, and the OpenType
features — and SHALL apply an edit to that layer through the type bridge,
re-rendering it and recording exactly one history state.
When no type layer is active it SHALL present the model defaults and SHALL NOT
record a state. Tsume SHALL be shown only while the East Asian preference is on.

#### Scenario: Panel reflects the active type layer

- **WHEN** a type layer is selected with Show Asian Text Options on
- **THEN** the Character panel shows that layer's font style, language, size, leading, tracking, scale, anti-aliasing, colour, tsume, fractional widths, and toggles

#### Scenario: Editing an attribute updates the layer as one state

- **WHEN** the size field is committed on the active type layer and then undone once
- **THEN** the layer's rendered text reflects the new size and the single undo restores the previous size

#### Scenario: No type layer shows the defaults

- **WHEN** no type layer is active
- **THEN** the panel shows the default character attributes and committing a value records no history state

#### Scenario: Font style changes the rendered face

- **WHEN** a family with a Bold face is active and Bold is picked in the style field
- **THEN** the layer re-renders in that family's Bold face and one undo restores the previous style

#### Scenario: Language is stored with the layer

- **WHEN** `French` is picked in the Language field and the layer is read back
- **THEN** the layer reports `French` and one undo restores the previous language

#### Scenario: Tsume applies while the East Asian preference is on

- **WHEN** Show Asian Text Options is on and Set Tsume is committed at 50 for a CJK line
- **THEN** the line's character-side spacing is reduced and one undo restores the previous value

#### Scenario: Fractional widths off rounds advances

- **WHEN** `Fractional Widths` is unchecked in the Character panel menu
- **THEN** the layer's glyph advances snap to whole pixels and one undo restores fractional spacing

### Requirement: Paragraph panel edits the active type layer

The Paragraph panel SHALL show the active type layer's paragraph attributes —
alignment and justification, direction, left/right/first-line indents, space
before and after, hanging punctuation, hyphenation, and composer — and SHALL
apply an edit to that layer through the type bridge, re-rendering it and
recording exactly one history state. The direction control SHALL be shown only
while the Middle Eastern preference is on.

#### Scenario: Panel reflects the active type layer

- **WHEN** a type layer is selected with Middle Eastern on
- **THEN** the Paragraph panel shows its alignment, direction, indents, paragraph spacing, hanging punctuation, hyphenation, and composer

#### Scenario: Editing a paragraph attribute updates the layer

- **WHEN** the first-line indent is committed and then undone once
- **THEN** the layer's stored paragraph attributes reflect the change and the single undo restores the previous value

#### Scenario: Direction is stored with the layer

- **WHEN** RTL is picked in the direction control and the layer is read back
- **THEN** the layer reports RTL and one undo restores LTR

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

- the font family and the font style beside it;
- the language;
- icon-labelled size and leading, kerning and tracking, vertical and
  horizontal scale, and baseline shift with the Color swatch, in two columns;
- the Set Tsume field while the East Asian preference is on;
- a row of eight checkable style buttons: Faux Bold, Faux Italic, All Caps,
  Small Caps, Superscript, Subscript, Underline, Strikethrough;
- a row of eight checkable OpenType buttons: Standard Ligatures, Contextual
  Alternates, Discretionary Ligatures, Swash, Stylistic Alternates, Titling
  Alternates, Ordinals, Fractions;
- the anti-aliasing method;
- a panel menu with a checkable `Fractional Widths` item.

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

#### Scenario: The style field lists the family's faces

- **WHEN** a family with Regular, Italic, Bold, and Bold Italic faces is chosen
- **THEN** the style field offers those faces and the current one

#### Scenario: The Fractional Widths item follows the layer

- **WHEN** a layer with fractional widths off is selected
- **THEN** the panel menu's Fractional Widths item is unchecked

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

### Requirement: Character panel edits new type while it is set

While a Horizontal or Vertical Type session is setting new text (no layer
exists yet), the Character panel SHALL show and edit that text's character
attributes. Each change SHALL re-render the on-canvas preview at once, record
no history state, and be carried by the layer the commit creates. The layer
selected before the session SHALL NOT change.

#### Scenario: Toggles preview on new text and land on its layer [cpn_new_text]

- **WHEN** `tst_character_paragraph_panels::characterPanelEditsNewText` selects
  a type layer, opens new type elsewhere, clicks Faux Bold and Underline in the
  Character panel, and commits
- **THEN** each click changes the preview, the new layer has Faux Bold and
  Underline, and the previously selected layer has neither

### Requirement: The Paragraph panel opens the Justification dialog

The Paragraph panel menu SHALL open a Justification dialog with `Word Spacing`
(0–1000%), `Letter Spacing` (−100–500%), and `Glyph Scaling` (50–200%), each as
Minimum, Desired, and Maximum, plus Auto Leading (%). Accepting the dialog SHALL
apply every value to the active type layer and record exactly one history
state; cancelling SHALL write nothing and record no state.

#### Scenario: The dialog round-trips its values

- **WHEN** Word Spacing Desired is set to 110, Letter Spacing Minimum to −5, Glyph Scaling Maximum to 120, and Auto Leading to 130, and the dialog is accepted
- **THEN** the layer reports those values and one undo restores the previous values

#### Scenario: Cancelling writes nothing

- **WHEN** a value is changed in the dialog and Cancel is pressed
- **THEN** the layer keeps its previous values and no history state is recorded

### Requirement: The Paragraph panel sets paragraph direction

The Paragraph panel SHALL show LTR and RTL direction buttons while the Middle
Eastern preference is on. Choosing a direction SHALL apply it to the active type
layer through the type bridge, record exactly one history state, and re-render
the layer with its reading order changed and the line placed by its alignment.

#### Scenario: RTL puts the first character at the right edge

- **WHEN** an LTR paragraph layer is set to RTL
- **THEN** its first character renders at the line's right edge and one undo restores LTR

#### Scenario: The control is absent without the engine

- **WHEN** Middle Eastern is off and a type layer is selected
- **THEN** the Paragraph panel shows no direction control
