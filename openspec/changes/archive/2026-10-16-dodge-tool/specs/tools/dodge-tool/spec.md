## ADDED Requirements

### Requirement: Dodge tool

A Dodge drag SHALL lighten the covered pixels of the active pixel layer, most
strongly for pixels whose luminance lies in the chosen Range, by an amount set
by Exposure and the tip's coverage, applying the effect once per pixel per
stroke however many dabs cover it, so a new stroke over the same area lightens
it further. With Protect Tones on the pixel's colour SHALL be kept and the
result SHALL NOT clip to white. Dodging SHALL never change a pixel's alpha. A
stroke SHALL record exactly one "Dodge" history state when pixels changed, and
a layer whose pixels are locked SHALL be refused.

#### Scenario: Lifting a dark grey

- **WHEN** the `tst_retouch_tools` Dodge test drags across a dark grey at the defaults
- **THEN** one "Dodge" state is recorded, the grey lifts gently, pixels outside the tip are unchanged, and a second stroke lifts it further
