## ADDED Requirements

### Requirement: Swatches added from the picker

The Swatches panel SHALL accept a colour and name added from the color picker
and append them to the grid. The panel SHALL generate `Swatch N` names for
swatches added through its New action, where N is one more than the highest
existing `Swatch N` name.

#### Scenario: The picker adds a swatch [sw_add_from_picker]

- **WHEN** the color picker adds a colour to the panel
- **THEN** the grid contains that colour under the entered name

#### Scenario: New uses a generated name [sw_generated_name]

- **WHEN** New is invoked twice on a fresh panel
- **THEN** the added swatches are named `Swatch 1` and `Swatch 2`
