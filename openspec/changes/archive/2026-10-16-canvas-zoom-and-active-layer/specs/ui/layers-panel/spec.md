## ADDED Requirements

### Requirement: Active layer row reflects the document

The system SHALL keep the document's active layer selected in the panel. On
open, on a document switch, and on any refresh that finds no selected row, the
panel SHALL select the document's active-layer path, falling back to the first
displayed row only when the document has no active layer. Because the model
reset a refresh performs clears the document's active layer through the
selection sync, the panel SHALL capture the active-layer path before rebuilding
the model. An explicit deselect (Select > Deselect Layers, or a click on empty
space) SHALL clear the document's active layer and SHALL NOT be undone by the
next refresh. A document whose only layer is the locked `Background` SHALL
report that row as active.

#### Scenario: The Background is active when it is the only layer [lpr_bg_active]

- **WHEN** a document whose only layer is the locked Background is opened or refreshed
- **THEN** the Background row is the selected and active row, and the document's
  active-layer path is its path

#### Scenario: The active layer is restored when it is not the first row [lpr_active_restore]

- **WHEN** a refresh finds no selected row and the document's active layer is not
  the first displayed row
- **THEN** the panel selects the document's active-layer path rather than the
  first displayed row

#### Scenario: An explicit deselect is preserved [lpr_active_deselect]

- **WHEN** the user clears the layer selection, leaving the document with no
  active layer
- **THEN** the next refresh leaves the panel with no selected row
