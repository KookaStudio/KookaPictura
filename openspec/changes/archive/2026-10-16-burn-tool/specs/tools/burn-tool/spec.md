## ADDED Requirements

### Requirement: Burn tool

A Burn drag SHALL darken the covered pixels of the active pixel layer, most
strongly for pixels whose luminance lies in the chosen Range, by an amount set
by Exposure and the tip's coverage, applying the effect once per pixel per
stroke however many dabs cover it, so a new stroke over the same area darkens
it further. With Protect Tones on the pixel's colour SHALL be kept and the
result SHALL NOT clip to black. Burning SHALL never change a pixel's alpha. A
stroke SHALL record exactly one "Burn" history state when pixels changed, and a
layer whose pixels are locked SHALL be refused.

#### Scenario: Darkening a light grey

- **WHEN** the `tst_retouch_tools` Burn test drags across a light grey at the defaults
- **THEN** one "Burn" state is recorded, the grey darkens, pixels outside the tip are unchanged, and a second stroke darkens it further
