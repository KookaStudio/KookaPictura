# Spec Delta

## Purpose

The Color Picker's colour services reuse their colour transforms, so a drag
through the field or ramp follows the cursor.

## ADDED Requirements

### Requirement: Picker colour services reuse their transforms

The picker's sRGB↔working-CMYK services (the gamut test, the closest printable
colour, and the CMYK readout and edit conversions) SHALL build their colour
transforms once for the installed working CMYK profile and SHALL reuse them on
every later call. They SHALL rebuild them only when Color Settings installs a
working CMYK profile. Every service MUST return the same value as a one-shot
conversion through the same profile, intent and black point compensation.

#### Scenario: Repeated calls build no transforms [cpd_reuse]

- **WHEN** the gamut test, closest colour, and CMYK conversions run for many
  colours with no Color Settings change
- **THEN** no transform is built after the first call

#### Scenario: Installing a working CMYK profile rebuilds once [cpd_rebuild]

- **WHEN** Color Settings installs a working CMYK profile
- **THEN** the transform pair is rebuilt exactly once, and later calls convert
  through the new pair

#### Scenario: Cached conversions match a one-shot conversion [cpd_exact]

- **WHEN** a colour is converted through the reusable transform and through a
  one-shot conversion with the same profiles, intent and black point
  compensation
- **THEN** the output bytes are identical

### Requirement: A picker drag follows the cursor

Each drag sample in the Color Picker dialog's colour field SHALL update the
field, every readout and both alerts. A sample MUST NOT parse a colour profile or
build a colour transform.

#### Scenario: A field drag updates the readouts on every sample [cpd_drag]

- **WHEN** the left button is held and the cursor moves across the colour field
  in many steps
- **THEN** after each step the hex readout shows the colour under the cursor,
  and the full drag finishes within the interactive budget
