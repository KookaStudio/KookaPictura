# Spec Delta

## ADDED Requirements

### Requirement: Pictura Raw settings actions

The Pictura Raw dialog's Presets tab SHALL present the Camera Raw settings
actions as a list: Image Settings, Camera Raw Defaults, Previous Conversion,
Custom Settings, Preset Settings, Apply Preset, Clear Imported Settings, Load
Settings…, and Save Settings…. Image Settings SHALL restore the settings the
dialog was opened with. Camera Raw Defaults SHALL clear every control to its
default. Previous Conversion SHALL restore the settings of the last Pictura Raw
dialog accepted in the session, and SHALL be disabled until one has been
accepted. Clear Imported Settings SHALL restore the image settings, and SHALL be
disabled when no settings file has been loaded. Custom Settings, Preset Settings,
and Apply Preset SHALL be present but disabled until the preset library exists.
Applying an action SHALL update every control and re-render the preview.

#### Scenario: Image Settings restores the opening settings [ra_settings_image]
- **WHEN** the dialog is opened with initial settings, a control is edited, and
  Image Settings is chosen
- **THEN** every control returns to the opening value and the settings text
  matches the opening settings

#### Scenario: Camera Raw Defaults clears every control [ra_settings_defaults]
- **WHEN** Camera Raw Defaults is chosen after editing
- **THEN** the settings text is empty and every control shows its default

#### Scenario: Previous Conversion restores the last accepted settings [ra_settings_previous]
- **WHEN** a Pictura Raw dialog is accepted with edited settings and a later
  dialog chooses Previous Conversion
- **THEN** the later dialog's controls and settings match the accepted settings

#### Scenario: Previous Conversion is unavailable before any conversion [ra_settings_previous_unavailable]
- **WHEN** no Pictura Raw dialog has been accepted in the session
- **THEN** the Previous Conversion action is disabled

#### Scenario: Clear Imported Settings reverts a loaded file [ra_settings_clear_imported]
- **WHEN** a settings file is loaded and Clear Imported Settings is chosen
- **THEN** the settings return to the image settings and the action is disabled
  again

#### Scenario: Preset-library actions are disabled [ra_settings_presets_disabled]
- **WHEN** the Presets tab is shown
- **THEN** Custom Settings, Preset Settings, and Apply Preset are present but
  disabled

### Requirement: Pictura Raw settings files

The dialog SHALL load and save Camera Raw settings as `crs:` XMP packets. Load
Settings… SHALL read a chosen file and apply its modeled properties to the
dialog. Save Settings… SHALL write the current settings as a `crs:` XMP packet. A
file with no `crs:` settings SHALL be refused without changing the dialog.

#### Scenario: Save then load round-trips the settings [ra_files_round_trip]
- **WHEN** settings are saved to a file and loaded into a fresh dialog
- **THEN** the fresh dialog reports the same modeled settings

#### Scenario: A non-XMP file is refused [ra_files_non_xmp]
- **WHEN** a file without `crs:` settings is loaded
- **THEN** loading fails and the dialog keeps its settings

### Requirement: Pictura Raw preview zoom

The preview pane SHALL change its zoom on a mouse wheel over the image, using the
same zoom ladder as the − and + buttons. A wheel up SHALL step toward a larger
zoom and a wheel down toward a smaller one.

#### Scenario: The wheel steps the preview zoom [ra_zoom_wheel]
- **WHEN** the mouse wheel is turned over the preview
- **THEN** the zoom readout moves one step along the ladder

#### Scenario: The wheel follows the button ladder [ra_zoom_ladder]
- **WHEN** the wheel is turned up from Fit in one dialog and the + button is
  pressed once in another
- **THEN** both dialogs show the same zoom readout

### Requirement: Pictura Raw dialog layout

The Pictura Raw dialog's tab strip SHALL be an even icon strip that draws no base
line, and the controls column SHALL be separated from the preview so the image
does not touch the controls.

#### Scenario: The tab strip has no base line and even icons [ra_layout_tabs]
- **WHEN** the dialog is opened
- **THEN** the tab strip draws no base line, expands its tabs evenly, and uses one
  icon size

#### Scenario: The controls clear the preview [ra_layout_spacing]
- **WHEN** the dialog is opened
- **THEN** the outer layout that separates the preview from the controls column
  carries spacing
