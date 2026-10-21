# Spec Delta

## MODIFIED Requirements

### Requirement: Active layer resolution for tool edits

The application SHALL maintain one authoritative layer target per active document
and SHALL resolve its single tool-edit target through one shared resolver used by
every tool-edit entry point (paint, filter, Free Transform, and content move).
The Layers panel SHALL write its selection into the document target and SHALL
display that target; it SHALL NOT be the source the resolver reads. A tool edit
SHALL be refused, with the document unchanged and a user-visible refusal, when no
layer is selected or when more than one layer is selected. No tool-edit entry
point SHALL fall back to the topmost raster layer. A non-PSD raster import SHALL
make its imported layer active. Structural and property commands SHALL remain
available on a multi-selection and SHALL act on the whole selected set.

#### Scenario: Exactly one active layer receives the edit [ltf_active_layer]

- **WHEN** a tool edit runs with exactly one layer active
- **THEN** that layer is the edit target and the other layers are unchanged

#### Scenario: The panel pushes its selection [ltf_active_layer_push]

- **WHEN** the user changes the selected layer in the Layers panel
- **THEN** the document target follows the panel selection and the next tool
  edit targets it

#### Scenario: Zero or multiple selection refuses [ltf_active_layer_refuse]

- **WHEN** a tool edit runs with no layer active or with more than one layer
  selected
- **THEN** the edit is refused with a user-visible refusal, the document is
  unchanged, and no history state is added

#### Scenario: A multi-selection keeps structural commands available [ltf_multi_commands]

- **WHEN** two layers are selected
- **THEN** a paint or filter edit is refused, while Group Layers, Merge Layers,
  Delete Layers, and the property controls act on both selected layers
