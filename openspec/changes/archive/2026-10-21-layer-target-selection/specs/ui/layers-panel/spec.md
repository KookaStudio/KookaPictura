# Spec Delta

## MODIFIED Requirements

### Requirement: Active layer row reflects the document

The system SHALL keep the document's layer target reflected in the panel. The
panel SHALL write the user's selection and current row into the document target
when the user changes them. On open, on a document switch, and on any refresh,
the panel SHALL select the rows named by the document target and SHALL NOT
capture or overwrite that target; it SHALL NOT fall back to the first displayed
row. An explicit deselect (Select > Deselect Layers, or a click on empty space)
SHALL clear the document target and SHALL NOT be undone by the next refresh. A
filter SHALL hide rows without changing the target; when the filter is cleared,
the target's visible rows SHALL be selected again. A selection-based command
SHALL use the target even while a selected row is hidden by the filter. With
several layers selected, every selected row SHALL remain visually selected. A
document whose only layer is the locked `Background` SHALL report that row as
active.

#### Scenario: The Background is active when it is the only layer [lpr_bg_active]

- **WHEN** a document whose only layer is the locked Background is opened or refreshed
- **THEN** the Background row is the selected and active row, and the document's
  active-layer path is its path

#### Scenario: The active layer is restored when it is not the first row [lpr_active_restore]

- **WHEN** a refresh rebuilds the model and the document's target is not the
  first displayed row
- **THEN** the panel selects the target's path rather than the first displayed
  row

#### Scenario: An explicit deselect is preserved [lpr_active_deselect]

- **WHEN** the user clears the layer selection, leaving the document with no
  active layer
- **THEN** the next refresh leaves the panel with no selected row

#### Scenario: A filter hiding the target does not clear it [lpr_filter_target]

- **WHEN** an active filter hides the row named by the document target
- **THEN** the document still reports that layer as selected and active, and
  clearing the filter selects its row again

#### Scenario: A command uses a filtered-out target [lpr_filter_command]

- **WHEN** the user runs a selection-based layer command while a selected row is
  hidden by the filter
- **THEN** the command acts on that hidden selected layer as part of the target

#### Scenario: A multi-selection is shown in full [lpr_multi_visual]

- **WHEN** the document target holds several selected layers
- **THEN** every visible selected row is shown as selected and the active path's
  row is the current row
