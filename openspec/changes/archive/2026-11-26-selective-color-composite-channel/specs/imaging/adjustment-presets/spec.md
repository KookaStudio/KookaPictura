## MODIFIED Requirements

### Requirement: Load Preset

The Properties panel menu SHALL offer `Load Preset…` for the same whitelist
kinds, listing the saved presets for the active kind and applying the chosen
one as a preset application. Each supported Image > Adjustments dialog SHALL
list the same saved presets in its Preset menu and apply them to its scratch
block.

#### Scenario: Panel Load applies a saved preset [preset_load_panel]

- **WHEN** `Load Preset…` is used on a Levels layer and `My Look` is chosen
- **THEN** the layer's parameters match the saved block

#### Scenario: Dialog Load applies a saved preset [preset_load_dialog]

- **WHEN** a saved Levels preset is chosen in the Levels dialog's Preset menu
- **THEN** the dialog's fields and preview show the saved values

#### Scenario: Selective Color round-trips a saved preset [selective_color_preset_round_trip]

- **WHEN** a user saves a Selective Color adjustment with a non-default method and per-family values through the Properties panel
- **THEN** choosing that preset through the Properties menu restores those settings
- **AND** choosing the same preset in the Selective Color dialog applies those settings to its scratch block
