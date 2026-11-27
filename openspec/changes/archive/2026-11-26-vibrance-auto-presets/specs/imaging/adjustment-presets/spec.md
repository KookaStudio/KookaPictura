## MODIFIED Requirements

### Requirement: Save Preset

The Properties panel menu SHALL offer `Save Preset…` for Levels, Curves,
Exposure, Hue/Saturation, Black & White, Channel Mixer, Selective Color, and
Vibrance. Vibrance is a Pictura extension to the CS6 preset list. Saving SHALL
prompt for a name, append the current adjustment block to the kind's saved
presets, and reject an empty name or the reserved names `Default` and
`Custom`. The item MUST be disabled for every other kind.

#### Scenario: Saving appends and persists [preset_save_persists]

- **WHEN** `Save Preset…` is used on a Levels layer and the name `My Look` is
  accepted
- **THEN** `My Look` is appended to the Levels Preset menu and is still listed
  after the panel is rebuilt

#### Scenario: Reserved names are rejected [preset_save_rejects_reserved]

- **WHEN** `Save Preset…` is used with the name `Default`, `Custom`, or an empty
  name
- **THEN** no preset is stored and the menu is unchanged

#### Scenario: Save is whitelisted [preset_save_whitelist]

- **WHEN** the active adjustment is Photo Filter
- **THEN** `Save Preset…` is disabled, while Levels, Curves, Exposure,
  Hue/Saturation, Black & White, Channel Mixer, Selective Color, and
  Vibrance enable it

### Requirement: Load Preset

The Properties panel menu SHALL offer `Load Preset…` for the same saveable
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

#### Scenario: Both Vibrance values are reusable [vibrance_preset_round_trip]

- **WHEN** a named Vibrance preset is saved from Properties
- **THEN** both values can be restored in one history state from a fresh
  Properties panel and are available in the direct dialog's Preset menu
