## ADDED Requirements

### Requirement: Channel Mixer dialog mode controls

The Channel Mixer dialog SHALL show a `Monochrome` checkbox in an RGB document
and SHALL show the Cyan, Magenta, Yellow, and Black output channels with four
ink source rows in a CMYK document. The checkbox MUST be hidden in a CMYK
document. The Output Channel menu SHALL follow the descriptor page, so it shows
`Gray` while Monochrome is on. Every numeric range SHALL come from the engine
layout.

#### Scenario: RGB dialog toggles Monochrome [ac_cm_monochrome]

- **WHEN** the Channel Mixer dialog opens on an RGB document and its Monochrome checkbox is clicked
- **THEN** the Output Channel menu changes to the single `Gray` entry and the
  source rows stay Red/Green/Blue

#### Scenario: CMYK dialog offers four ink outputs [ac_cm_cmyk]

- **WHEN** the Channel Mixer dialog opens on a CMYK document
- **THEN** the Output Channel menu lists Cyan, Magenta, Yellow, and Black, the
  four source rows are named for the inks, and no Monochrome checkbox is shown
