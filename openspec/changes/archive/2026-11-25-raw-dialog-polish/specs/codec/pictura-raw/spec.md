## ADDED Requirements

### Requirement: Pictura Raw panel inset

Every control panel SHALL inset its content from the panel's left and right
edges so a slider's name does not touch the border.

#### Scenario: Slider names clear the panel border [rp_panel_inset]

- **WHEN** a control panel is shown
- **THEN** its layout carries a left and right content margin of at least eight
  pixels

### Requirement: Pictura Raw HSL range colours

The HSL/Grayscale Hue, Saturation, and Luminance sliders SHALL each paint the
colour ramp of their own range: reds, oranges, yellows, greens, aquas, blues,
purples, and magentas, in the same order as the range labels. The Grayscale Mix
sliders SHALL stay grey.

#### Scenario: Each range slider carries its own colours [rp_hsl_ramps]

- **WHEN** the Hue, Saturation, or Luminance tab is shown
- **THEN** the Reds slider's middle stop is red, Oranges orange, Yellows
  yellow, Greens green, Aquas cyan, Blues blue, Purples violet, and Magentas
  magenta, and the Grayscale Mix sliders are grey

### Requirement: Pictura Raw effects icon and settings button

The tab strip SHALL expose eight tabs, with the Effects tab carrying the `fx`
icon. The settings actions SHALL move from a tab to a separate `fx` button at
the right of the tab strip that opens the settings list as a popup.

#### Scenario: The Effects tab is the fx tab [rp_effects_icon]

- **WHEN** the dialog is shown
- **THEN** the tab strip has eight tabs, the Effects tab is the sixth, and a
  separate `acrSettingsButton` sits at its right

#### Scenario: The fx button opens the settings list [rp_settings_button]

- **WHEN** the fx button is clicked
- **THEN** the settings list appears and its actions apply as before
