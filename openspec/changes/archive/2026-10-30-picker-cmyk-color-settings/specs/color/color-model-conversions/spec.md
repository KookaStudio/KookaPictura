## MODIFIED Requirements

### Requirement: CMYK values come from the CMYK working profile

The engine SHALL convert sRGB to CMYK and back through the CMYK working profile
and the Color Settings conversion options — the rendering intent and the
black-point-compensation flag — which default to relative colorimetric with
black-point compensation. Dither SHALL NOT apply: this conversion produces one
colour, not a plane, and the ordered dither pattern is spatial. The engine SHALL
NOT use a fixed `1 − max(R, G, B)` formula. Components SHALL be percentages in
0–100. Until Color Settings exposes a CMYK working-space choice, the working
profile SHALL be the bundled coated CMYK profile. Every CMYK readout, CMYK field
edit, gamut alert, and closest printable colour SHALL convert through the one
working CMYK profile and the one set of conversion options that Color Settings
installs.

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

#### Scenario: Every conversion follows the installed profile [cmyk_installed_profile]

- **WHEN** a working CMYK profile other than the bundled coated one is installed
- **THEN** the CMYK readout, the CMYK edit, the gamut test, and the closest
  printable colour each equal that profile's conversion

#### Scenario: Picker fields use the installed pair [cmyk_fields_installed_pair]

- **WHEN** the picker shows a colour
- **THEN** its C, M, Y and K fields and the shared readout conversion equal the
  picker's working-profile projection of that colour

#### Scenario: Every conversion follows the installed options [cmyk_installed_options]

- **WHEN** the Color Settings rendering intent or black-point compensation changes
- **THEN** the CMYK readout, the CMYK edit, the gamut test, and the closest
  printable colour each equal that option's conversion
