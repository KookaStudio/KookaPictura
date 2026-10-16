# ui/properties-panel Specification

## Purpose
The CS6 Properties panel: it identifies the active layer and, for an adjustment
layer, hosts the descriptor-driven controls that edit it
(`ui/properties-panel-editing`); any other layer shows a read-only summary.

## Requirements

### Requirement: Empty Properties state

The `PropertiesPanel` SHALL show the text "No Properties" when there is no open
document, when there is no active layer, or when the active layer is not an
adjustment layer. `refresh` SHALL recompute this state from the current view.

#### Scenario: A plain document shows no properties [pp_empty_plain]

- **WHEN** a document with only a pixel layer is open and no adjustment layer is
  active
- **THEN** `refresh` sets the panel's message to "No Properties"

#### Scenario: No document shows no properties [pp_empty_none]

- **WHEN** the panel has no view or the view has no document
- **THEN** the panel's message is "No Properties"

### Requirement: Active adjustment layer is named

When the active layer is an adjustment layer, `PropertiesPanel::refresh` SHALL
name that layer read-only. The active row SHALL be found by scanning the layer
rows and matching `layer_row_path` against `active_layer_path`.

#### Scenario: An active adjustment is named [pp_adjustment_named]

- **WHEN** an adjustment layer is added, made active, and the panel is refreshed
- **THEN** the panel's message contains that adjustment layer's name

### Requirement: Properties panel edits the active adjustment layer

The `PropertiesPanel` SHALL build the active adjustment layer's controls from
its descriptor, apply edits to the canvas live, and record one "Modify `Title`
Layer" state per gesture; the parameter model and edit session are specified in
`ui/properties-panel-editing`.

#### Scenario: The panel edits the active adjustment [pp_edits]

- **WHEN** the panel shows an active adjustment layer
- **THEN** it exposes live-editable controls and records one history state per gesture

#### Scenario: The suite is registered [pp_contract_suite]

- **WHEN** the app tests are built and run
- **THEN** `tst_properties_panel` is one of the registered Qt Test suites
