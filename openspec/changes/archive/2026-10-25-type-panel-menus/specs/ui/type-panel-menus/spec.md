# Spec Delta

## ADDED Requirements

### Requirement: Character panel menu mirrors CS6

The Character panel menu SHALL list, in this order, Change Text Orientation,
OpenType, Faux Bold, Faux Italic, All Caps, Small Caps, Superscript, Subscript,
Underline, Strikethrough, Fractional Widths, System Layout, No Break, Middle
Eastern Features, and Reset Character. The OpenType submenu SHALL list the eight
OpenType features in panel order.

#### Scenario: Character menu lists the CS6 rows

- **WHEN** the Character panel menu is opened on an active type layer
- **THEN** the rows appear in the CS6 order and the OpenType submenu carries all eight features

#### Scenario: Character menu check states follow the layer

- **WHEN** a layer with Faux Bold and Fractional Widths on and a selected OpenType feature is shown
- **THEN** those rows are checked, System Layout is unchecked, and the OpenType rows match the layer

### Requirement: Character menu commands apply through the panel

The menu's implemented commands SHALL apply exactly what the corresponding panel
control applies: each style and OpenType toggle, Fractional Widths, System
Layout (the inverse), and Reset Character SHALL re-render the layer and record
exactly one history state. Reset Character SHALL restore the model defaults for
the panel's character attributes while keeping the layer's font family, style,
size, colour, and text. Opening the menu SHALL NOT record a state.

#### Scenario: A style toggle applies from the menu

- **WHEN** Faux Bold is triggered in the Character panel menu and then undone once
- **THEN** the layer's Faux Bold flips, one history state is recorded, and the button follows

#### Scenario: Fractional Widths and System Layout are inverses

- **WHEN** Fractional Widths is triggered off and System Layout is triggered on and then undone
- **THEN** the layer's fractional widths follow, System Layout's check state is the inverse, and each trigger records one state

#### Scenario: Reset Character restores defaults

- **WHEN** Reset Character is triggered on a styled layer and then undone once
- **THEN** the resettable character attributes return to their defaults, the family, style, size, colour, and text are unchanged, and the undo restores the previous attributes

### Requirement: Paragraph panel menu mirrors CS6

The Paragraph panel menu SHALL list Roman Hanging Punctuation, Justification…,
Hyphenation…, Latin and East Asian Layout, World-Ready Layout, Default Features,
East Asian Features, Middle Eastern Features, Single-line Composer, Every-line
Composer, and Reset Paragraph.

#### Scenario: Paragraph menu lists the CS6 rows

- **WHEN** the Paragraph panel menu is opened on an active type layer
- **THEN** the rows appear in the CS6 order with the documented grouping separators

#### Scenario: Paragraph menu check states follow the layer

- **WHEN** a layer with hanging punctuation off and the single-line composer is shown
- **THEN** Roman Hanging Punctuation is unchecked and Single-line Composer is checked

### Requirement: Paragraph menu commands apply through the panel

Roman Hanging Punctuation, the two composer rows, and Reset Paragraph SHALL
apply through the panel's existing paragraph path and record exactly one history
state. Justification… SHALL open the Justification dialog. Reset Paragraph SHALL
restore the model paragraph defaults while keeping the layer's text and
character attributes. Opening the menu SHALL NOT record a state.

#### Scenario: A paragraph toggle applies from the menu

- **WHEN** Roman Hanging Punctuation is triggered and then undone once
- **THEN** the layer's hanging flag flips, one history state is recorded, and the checkbox follows

#### Scenario: Composer rows are exclusive

- **WHEN** Every-line Composer is triggered while Single-line Composer is on
- **THEN** the layer's composer changes to every-line, the menu shows Every-line checked, and one undo restores single-line

#### Scenario: Reset Paragraph restores defaults

- **WHEN** Reset Paragraph is triggered on a styled paragraph layer and then undone once
- **THEN** the paragraph attributes return to their defaults, the character attributes and text are unchanged, and the undo restores the previous paragraph

#### Scenario: The Justification item opens the dialog

- **WHEN** Justification… is triggered
- **THEN** the Justification dialog opens and accepting it records one history state

### Requirement: Unimplemented menu rows stay disabled and Close stays last

Rows whose behavior is not implemented SHALL remain present but disabled with the
existing "not implemented yet" tooltip. The final two entries in both menus SHALL
be Close and Close Tab Group.

#### Scenario: Unimplemented rows are disabled

- **WHEN** either panel menu is opened
- **THEN** the known unimplemented rows are disabled and carry the not-implemented tooltip

#### Scenario: Close entries are last

- **WHEN** either panel menu is opened
- **THEN** Close and Close Tab Group are its final two non-separator entries
