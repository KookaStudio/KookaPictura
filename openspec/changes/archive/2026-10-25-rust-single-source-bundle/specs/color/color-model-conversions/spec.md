# Spec Delta

## Purpose

Defines the engine-owned scalar colour-model conversions: sRGB to and from Lab and
sRGB to and from CMYK. Every colour readout and colour entry field in the shell
uses them, so all readouts agree with each other and with the colour-management
engine.

## ADDED Requirements

### Requirement: sRGB and Lab convert through the D50 connection space

The engine SHALL convert an 8-bit sRGB triple to CIELAB relative to the D50 ICC
profile connection space, adapting from sRGB's D65 white with the Bradford
transform. It SHALL convert Lab back to sRGB by the inverse path, clamped to
0–255. The result SHALL agree with the colour-management engine's own sRGB→Lab
(D50) transform within one unit in L, a and b across a sweep of the 8-bit
sRGB cube. Lab SHALL be reported as L in 0–100 and a, b in −128…+127.

#### Scenario: White is L 100 neutral [lab_white]

- **WHEN** sRGB `(255, 255, 255)` is converted to Lab
- **THEN** the result rounds to `L 100, a 0, b 0`

#### Scenario: Known primaries match the connection space [lab_known_values]

- **WHEN** sRGB red `(255, 0, 0)` is converted to Lab
- **THEN** the result is within one unit of the colour-management engine's D50 Lab
  for the same colour (about `L 54, a 81, b 70`)

#### Scenario: Lab round-trips to the same sRGB [lab_round_trip]

- **WHEN** a sweep of 8-bit sRGB triples is converted to Lab and back without
  rounding Lab
- **THEN** each channel equals the original within one level

#### Scenario: Out-of-gamut Lab clamps [lab_clamps]

- **WHEN** a Lab value outside the sRGB gamut, such as `L 50, a 127, b −128`, is
  converted to sRGB
- **THEN** every channel is clamped to 0–255 and none wraps

### Requirement: CMYK values come from the CMYK working profile

The engine SHALL convert sRGB to CMYK and back through the CMYK working profile,
using the relative colorimetric intent with black-point compensation. It SHALL
NOT use a fixed `1 − max(R, G, B)` formula. Components SHALL be percentages in
0–100. Until Color Settings exposes a CMYK working-space choice, the working
profile SHALL be the bundled coated CMYK profile.

#### Scenario: White has no ink [cmyk_white]

- **WHEN** sRGB `(255, 255, 255)` is converted to CMYK
- **THEN** every component rounds to 0

#### Scenario: Black uses the profile's rich black [cmyk_black_profile]

- **WHEN** sRGB `(0, 0, 0)` is converted to CMYK
- **THEN** the result equals the working profile's transform of black and is not
  the naive `0, 0, 0, 100`

#### Scenario: CMYK round-trips in gamut [cmyk_round_trip]

- **WHEN** an in-gamut sRGB colour is converted to CMYK and back
- **THEN** each channel is within the profile's round-trip tolerance of the
  original

### Requirement: Colour readouts and fields use the shared conversions

The colour picker's Lab and CMYK fields and the Info panel's Lab, CMYK and Total
Ink readouts SHALL take their values from the shared conversions. No shell
component SHALL keep its own Lab or CMYK formula. HSB stays the standard
cylindrical transform of the RGB value.

The Channels panel's ink thumbnails are not readouts. They show the document's
plate split, the same split that channel-to-selection uses, and SHALL take it
from the engine's single plate derivation rather than a shell formula.

#### Scenario: Picker and Info panel show the same Lab [lab_picker_info_agree]

- **WHEN** the picker holds a colour and the Info panel samples a pixel of the
  same sRGB value in Lab mode
- **THEN** both show the same rounded L, a and b

#### Scenario: Editing Lab in the picker updates RGB through the engine [picker_lab_edit]

- **WHEN** the user types Lab values into the picker
- **THEN** the RGB and hex fields show the shared conversion's sRGB for that Lab

#### Scenario: Picker CMYK matches the Info panel [cmyk_picker_info_agree]

- **WHEN** the picker holds a colour and the Info panel samples the same sRGB
  value in CMYK mode
- **THEN** both show the same rounded C, M, Y and K

#### Scenario: Channels panel thumbnails match channel-to-selection [channels_plate_source]

- **WHEN** a CMYK-mode or Multichannel document's Channels panel renders its ink
  thumbnails
- **THEN** each thumbnail's grey values equal the engine's plate values for that
  row, the same values a Ctrl-click on the row loads as a selection, and are
  unchanged from before the move
