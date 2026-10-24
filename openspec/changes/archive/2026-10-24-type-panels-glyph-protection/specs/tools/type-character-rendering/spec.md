# Spec Delta

## MODIFIED Requirements

### Requirement: Per-font OpenType support

`pictura-render` SHALL report, for a font family and style, which of the
Character panel's eight OpenType features the resolved face carries, as a bit
mask in panel order: Standard Ligatures, Contextual Alternates, Discretionary
Ligatures, Swash, Stylistic Alternates, Titling Alternates, Ordinals,
Fractions. A bit SHALL be set exactly when the face's GSUB or GPOS feature list
names the feature's tag (`liga` or `clig` for Standard Ligatures).

#### Scenario: Support follows the face's feature list

- **WHEN** the support mask of a face is requested
- **THEN** each bit is set exactly when the face's layout tables list that feature's tag

#### Scenario: Style support follows the selected face

- **WHEN** a family registers a Bold face whose OpenType features differ from its Regular face
- **THEN** the support mask for style Bold equals the Bold face's features, not the Regular face's

## ADDED Requirements

### Requirement: Font style selects the face

The renderer SHALL resolve a type layer's face from its font family and font
style together. When the requested style's face is registered for the family, the
renderer SHALL use it; when it is not, the renderer SHALL use the family's
regular face. The style name SHALL match the face's own style name, ignoring case
and spacing. Selecting a style SHALL NOT change the layer's other character
attributes.

#### Scenario: A bold style renders a different face

- **WHEN** a layer set in a family with a Bold face is rendered with style Bold
- **THEN** its ink differs from the same text rendered in the family's Regular style

#### Scenario: An unregistered style falls back to Regular

- **WHEN** a layer requests a style the family does not register
- **THEN** the family's Regular face renders and no other attribute changes

#### Scenario: Style selection is case- and spacing-insensitive

- **WHEN** the style is written `bold italic` and the registered face is `BoldItalic`
- **THEN** the Bold Italic face is used
