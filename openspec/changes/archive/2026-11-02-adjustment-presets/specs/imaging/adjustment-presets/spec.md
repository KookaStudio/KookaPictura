## ADDED Requirements

### Requirement: Properties preset menu

The Properties panel SHALL show a Preset menu for a Levels, Curves, Photo
Filter, Selective Color, or Vibrance adjustment layer. The menu MUST list
`Default`, the kind's built-in presets, the saved presets for that kind, and
`Custom`. The menu MUST be hidden for every other adjustment kind.

#### Scenario: Built-ins and saved presets are listed [preset_menu_lists]

- **WHEN** a Levels adjustment layer is active and a preset named `My Look` is
  saved for Levels
- **THEN** the Preset menu lists `Default`, the Levels built-ins, `My Look`, and
  `Custom`

#### Scenario: Unsupported kind has no preset menu [preset_menu_hidden]

- **WHEN** a Brightness/Contrast adjustment layer is active
- **THEN** the Properties panel shows no Preset menu

### Requirement: Preset application

Choosing a preset SHALL replace the adjustment's parameters with the preset's
block and SHALL record exactly one history state named `Modify <title> Layer`.
The menu MUST show `Custom` after a parameter edit that no longer matches a
preset, and MUST show the matching preset's name when the parameters match one.

#### Scenario: Applying a preset is one history state [preset_apply_history]

- **WHEN** a built-in preset is chosen for a Levels layer
- **THEN** the layer parameters change and exactly one `Modify Levels Layer`
  state is added

#### Scenario: A hand edit shows Custom [preset_custom]

- **WHEN** a parameter is edited after a preset was applied
- **THEN** the Preset menu shows `Custom`

### Requirement: Save Preset

The Properties panel menu SHALL offer `Save Preset…` for the CS6 save
whitelist: Levels, Curves, Exposure, Hue/Saturation, Black & White, Channel
Mixer, and Selective Color. Saving SHALL prompt for a name, append the current
adjustment block to the kind's saved presets, and reject an empty name or the
reserved names `Default` and `Custom`. The item MUST be disabled for every other
kind.

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

- **WHEN** the active adjustment is Vibrance or Photo Filter
- **THEN** `Save Preset…` is disabled, while Levels, Curves, Exposure,
  Hue/Saturation, Black & White, Channel Mixer, and Selective Color enable it

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

### Requirement: Built-in presets

The built-in preset lists SHALL use only established sources: the Levels,
Exposure, Hue/Saturation, and Black & White tables already in the dialogs, the
community Curves list, and the Photo Filter Wratten/colour filters. Kinds with
no established list (Channel Mixer, Selective Color, Vibrance) MUST NOT invent
built-in looks; they list `Default` plus saved presets. A Photo Filter built-in
preset MUST change only the filter colour and preserve Density and Preserve
Luminosity.

#### Scenario: Established built-ins are listed [preset_builtins_established]

- **WHEN** the Levels, Curves, or Photo Filter Preset menu is opened
- **THEN** it lists the established names for that kind

#### Scenario: Photo Filter preset keeps density [preset_photo_filter_density]

- **WHEN** Density is 50 % and a Photo Filter built-in preset is chosen
- **THEN** the colour changes and Density remains 50 %

### Requirement: Preset persistence

Saved presets SHALL persist outside the document in the user data directory
(`$XDG_DATA_HOME/kooka-pictura/Presets/Adjustments/`, falling back to
`~/.local/share/...`), one file per adjustment kind, storing the preset name and
the raw adjustment block. Writes MUST be atomic: a temporary file in the same
directory is renamed over the target. A missing, unreadable, or malformed file
MUST read as an empty list and MUST NOT panic or block the panel.

#### Scenario: Presets survive a restart [preset_survives_restart]

- **WHEN** a preset is saved and a fresh panel opens the same kind
- **THEN** the preset is listed and applies the same block

#### Scenario: A malformed store reads empty [preset_malformed_store]

- **WHEN** the kind's store file contains malformed lines
- **THEN** reading it returns an empty list without panicking

### Requirement: Preset kind mapping

The engine SHALL map each adjustment block to its catalogue kind and SHALL
compare two blocks by their decoded adjustment, so a preset saved from one
layer matches an identical layer regardless of encoding details.

#### Scenario: Blocks map to their kind [preset_kind_mapping]

- **WHEN** a default block for each preset kind is mapped
- **THEN** it returns that kind's catalogue key

#### Scenario: Decoded equality matches presets [preset_decoded_match]

- **WHEN** two blocks decode to the same adjustment
- **THEN** the preset matcher reports a match
