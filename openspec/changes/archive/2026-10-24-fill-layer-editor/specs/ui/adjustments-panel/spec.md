# Spec Delta — ui/adjustments-panel

## MODIFIED Requirements

### Requirement: Fill icons route to the fill-layer commands

Clicking the Solid Color, Gradient, or Pattern icon SHALL invoke the existing
New Fill Layer command for that type. The Pattern icon SHALL be enabled when the
open document contains at least one pattern resource and disabled with a "No
patterns are loaded" reason when it contains none, matching the Layer menu.

#### Scenario: Solid Color icon opens the solid-color flow

- **WHEN** the Solid Color icon is clicked
- **THEN** the Solid Color fill-layer command is dispatched

#### Scenario: Pattern icon is inert

- **WHEN** the open document contains no pattern resources and the Pattern icon is clicked
- **THEN** no layer is created, no command is dispatched, and the icon is disabled with the no-patterns reason

#### Scenario: Pattern follows the document's patterns [adj_pattern_icon]

- **WHEN** the Pattern icon is shown for a document with patterns and for one without
- **THEN** it is enabled and dispatches the Pattern fill command on the first, and disabled with the no-patterns reason on the second

### Requirement: Grid enablement follows the document

Without an open document, the grid SHALL be disabled. With a document open, grid
entries SHALL be enabled, except the Pattern entry, which SHALL be enabled only
when the document contains pattern resources.

#### Scenario: No document disables the grid

- **WHEN** no document is open
- **THEN** every grid entry is disabled

#### Scenario: A document enables the grid

- **WHEN** a document is open
- **THEN** every grid entry is enabled except Pattern when the document has no pattern resources
