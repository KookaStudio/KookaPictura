# ui/color-picker Specification

## Purpose
The Color picker dialog: choose a colour by HSB, RGB, Lab, or hexadecimal.

## Requirements

### Requirement: Color picker dialog

The application SHALL provide a full Photoshop-style `ColorPickerDialog`: a
colour field whose plane and vertical ramp follow the selected axis (Hue,
Saturation, Brightness, Red, Green, or Blue), a new/current comparison swatch
whose lower half restores the colour the dialog opened with, and editable HSB,
RGB, Lab, CMYK, and hex fields. The CMYK fields SHALL be a projection of the
current colour through the application's working CMYK profile, and editing a CMYK
field SHALL convert through that profile back to the authoritative RGB colour. A
"Only Web Colors" option SHALL snap the colour to the 216-colour web-safe
palette. When the colour is not web-safe and Only Web Colors is off, a web-safe
alert SHALL be shown and clicking it SHALL select the nearest web-safe colour.
When the colour lies outside the working CMYK gamut, an out-of-gamut alert SHALL
be shown with a swatch of the closest printable CMYK equivalent, and clicking it
SHALL replace the colour with that equivalent. The dialog SHALL expose a modal
`getColor` helper returning an invalid colour on cancel.

#### Scenario: Picking a colour

- **WHEN** the Count options bar opens the picker on a group colour and a new colour is chosen and accepted
- **THEN** the dialog returns it and the group's marks redraw in it

#### Scenario: Web-safe snapping

- **WHEN** Only Web Colors is on
- **THEN** the chosen colour is snapped to the nearest multiple of 0x33 per channel

#### Scenario: The web-safe alert appears and snaps

- **WHEN** the current colour is not web-safe and Only Web Colors is off
- **THEN** the web-safe alert is visible and clicking it replaces the colour with the nearest multiple of 0x33 per channel

#### Scenario: The gamut alert applies the closest CMYK colour

- **WHEN** the current colour is outside the working CMYK gamut
- **THEN** the gamut alert is visible and clicking it replaces the colour with the closest printable CMYK equivalent

#### Scenario: CMYK fields are editable through the working profile

- **WHEN** the CMYK fields are set to a printable ink percentage
- **THEN** every readout updates from the profile conversion and the resulting colour is without a gamut alert

#### Scenario: An in-gamut colour hides the alerts

- **WHEN** the current colour is web-safe and inside the working CMYK gamut
- **THEN** both alerts are hidden

### Requirement: Picker entry points

The toolbox's foreground and background swatches SHALL open the color picker
dialog for that colour, and the Color panel SHALL embed the picker's colour
field and hue ramp inline so clicking them sets the active swatch.

#### Scenario: Toolbox swatch

- **WHEN** a toolbox foreground/background swatch is clicked and a colour chosen
- **THEN** that swatch adopts the chosen colour

#### Scenario: Color panel field

- **WHEN** the Color panel's field or ramp is clicked
- **THEN** the active foreground/background colour follows it

### Requirement: Hex shorthand and prefixes

The color picker's hexadecimal field SHALL accept a leading `#` or `0x`
(case-insensitive) and a three-digit shorthand, expanding each shorthand digit
to a pair (`#123` becomes `#112233`). A complete six-digit value SHALL apply
while the user types. A three-digit shorthand SHALL apply when the field is
committed by Enter or focus loss. A value that is neither three nor six hex
digits SHALL leave the current colour unchanged; committing an invalid value
SHALL restore the field to the current colour. After a commit the field SHALL
show the canonical bare uppercase six-digit value.

#### Scenario: Prefixes are accepted [cp_hex_prefix]

- **WHEN** `#aabbcc` or `0xAABBCC` is entered in the hex field
- **THEN** the picker's colour is `#aabbcc`

#### Scenario: Shorthand expands on commit [cp_hex_shorthand]

- **WHEN** `#123` is entered and the field is committed
- **THEN** the picker's colour is `#112233` and the field shows `112233`

#### Scenario: Invalid input leaves the colour [cp_hex_invalid]

- **WHEN** a value that is not three or six hex digits is committed
- **THEN** the picker's colour is unchanged and the field shows the current colour's hex

### Requirement: Add to Swatches

The color picker SHALL provide an Add to Swatches action that appends the
picker's current colour to the application's Swatches panel. The action SHALL
prompt for a name defaulting to the panel's next generated swatch name, and
SHALL append the colour only when the prompt is confirmed. The action SHALL NOT
close the picker. When the picker has no Swatches panel, the action SHALL be
disabled.

#### Scenario: Add the current colour [cp_add_swatch]

- **WHEN** Add to Swatches is clicked with a Swatches panel present and the name prompt is confirmed
- **THEN** the current colour is appended to the panel under the entered name and the picker stays open

#### Scenario: Cancel the name prompt [cp_add_swatch_cancel]

- **WHEN** the name prompt is cancelled
- **THEN** no swatch is appended

#### Scenario: No panel [cp_add_swatch_disabled]

- **WHEN** the picker has no Swatches panel
- **THEN** Add to Swatches is disabled

### Requirement: The color picker follows the picker preference

The application SHALL persist a Color Picker preference with `Adobe` and `System` choices, defaulting to `Adobe`. Every production picker entry point SHALL honour the preference through the shared `getColor` helper: `Adobe` SHALL open the application picker, and `System` SHALL open the platform/Qt colour dialog. A direct test construction of `ColorPickerDialog` MAY bypass the preference. The preference SHALL NOT affect the colour value returned or create a history state.

#### Scenario: The default is the Adobe picker

- **WHEN** the application starts with no stored picker preference
- **THEN** the reported preference is Adobe

#### Scenario: A stored preference survives a restart

- **WHEN** the preference is set to System and the session is saved and reloaded
- **THEN** the reported preference is System

#### Scenario: A chosen colour is returned by either picker

- **WHEN** a colour is accepted through the System picker
- **THEN** the caller receives the same RGB colour value it selected and no history state is added
