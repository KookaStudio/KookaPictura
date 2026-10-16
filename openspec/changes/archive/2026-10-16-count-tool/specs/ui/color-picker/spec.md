## ADDED Requirements

### Requirement: Color picker dialog

The application SHALL provide a full Photoshop-style `ColorPickerDialog`: a
colour field whose plane and vertical ramp follow the selected axis (Hue,
Saturation, Brightness, Red, Green, or Blue), a new/current comparison swatch
whose lower half restores the colour the dialog opened with, and editable HSB,
RGB, Lab, and hex fields beside a CMYK readout. A "Only Web Colors" option
SHALL snap the colour to the 216-colour web-safe palette. The dialog SHALL
expose a modal `getColor` helper returning an invalid colour on cancel.

#### Scenario: Picking a colour

- **WHEN** the Count options bar opens the picker on a group colour and a new colour is chosen and accepted
- **THEN** the dialog returns it and the group's marks redraw in it

#### Scenario: Web-safe snapping

- **WHEN** Only Web Colors is on
- **THEN** the chosen colour is snapped to the nearest multiple of 0x33 per channel

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

