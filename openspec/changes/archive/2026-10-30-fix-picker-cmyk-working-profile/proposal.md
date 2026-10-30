# Proposal: CMYK readouts follow the working CMYK profile

## Why

The Color Picker's C/M/Y/K fields and the Info panel's CMYK and Total Ink
readouts come from `CmykConverter::working()`, which is always the bundled coated
profile. The gamut alert, the closest printable colour, and CMYK field edits use
the picker pair that Color Settings installs. Once another working CMYK profile
can be selected, the fields can disagree with the alert, and typing the shown
values back can produce a different colour (#1047).

## What changes

- `rgbToCmyk` reads `picker_cmyk`, so every CMYK readout, edit, and alert goes
  through the installed pair.
- The `srgb_to_cmyk` bridge, `CmykValue`, and `CmykConverter` are removed, so no
  second CMYK path remains.
- Readouts now come from the 8-bit transform as whole percentages. A shown value
  can move by a point against the old floating-point path.

Selecting another profile in `set_color_settings`, and the picker's use of the
Color Settings intent and black-point compensation, are out of scope.

## Capabilities

- Modified: `color/color-model-conversions`
