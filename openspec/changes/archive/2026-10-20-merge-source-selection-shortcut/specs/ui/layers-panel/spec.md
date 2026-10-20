# Spec Delta

## ADDED Requirements

### Requirement: Merge result selection

After a successful Merge Down, Merge Layers, or Merge Visible, the Layers panel
SHALL select exactly the merged pixel layer. Any hidden or unselected layer that
survives the merge MUST NOT remain selected.

#### Scenario: Merge Visible selects the result over a hidden survivor

- **WHEN** Merge Visible merges the visible layers while a hidden layer survives
- **THEN** the merged pixel layer is the sole selected layer and the hidden layer
  is not selected

#### Scenario: Merge Down selects the result

- **WHEN** Merge Down replaces a layer and the layer below it
- **THEN** the merged pixel layer is the sole selected layer

#### Scenario: Merge Layers selects the result

- **WHEN** Merge Layers replaces a non-contiguous selection
- **THEN** the merged pixel layer is the sole selected layer
