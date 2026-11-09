# Spec Delta

## MODIFIED Requirements

### Requirement: Fill icons route to the fill-layer editors

Clicking the Solid Color, Gradient, or Pattern icon SHALL create a fill layer
of that type with the current defaults and then open its type editor, leaving
the default layer in place if the editor is cancelled. The Pattern icon SHALL be
enabled whenever a document is open, because the built-in pattern library is
always available, and SHALL NOT carry a no-patterns reason, matching the Layer
menu.

#### Scenario: Solid Color icon opens the solid-color flow

- **WHEN** the Solid Color icon is clicked
- **THEN** a Solid Color fill layer is created with the default content and its editor opens

#### Scenario: Pattern icon is inert

- **WHEN** no document is open or no pattern is available from the document or the built-in library and the Pattern icon is shown
- **THEN** the icon is disabled with the no-patterns reason

#### Scenario: Pattern follows the document's patterns [adj_pattern_icon]

- **WHEN** the open document contains no embedded pattern resources and the Pattern icon is clicked
- **THEN** a Pattern fill layer is created with a built-in pattern and its editor opens, and cancelling leaves the default layer in place

### Requirement: Grid enablement follows the document

Without an open document, the grid SHALL be disabled. With a document open, every
grid entry SHALL be enabled, including Pattern, because the built-in pattern
library is always available.

#### Scenario: No document disables the grid

- **WHEN** no document is open
- **THEN** every grid entry is disabled

#### Scenario: A document enables the grid

- **WHEN** a document is open
- **THEN** every grid entry is enabled, including Pattern on a document with no embedded pattern resources
