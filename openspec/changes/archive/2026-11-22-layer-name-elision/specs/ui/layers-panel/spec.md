# ui/layers-panel

## ADDED Requirements

### Requirement: Layer name label renders with one elision

The system SHALL paint a layer row's name with a single elision to the available
label band, for every row kind (layer, group, styled/effects, and synthetic
effect rows). A name that fits the band SHALL be shown in full, with no
ellipsis, and only the lock, fx, link, Blend If, and list-toggle badges actually
present SHALL reduce the band. A name wider than the band SHALL be elided
exactly once in the middle, keeping the start and the end with a single ellipsis
between them. The painted text SHALL be the same text whose width the row's
`nameRect` reports, so rendering and the inline-rename hit area cannot drift.

#### Scenario: A name that fits is shown in full [lpr_name_fit]

- **WHEN** a layer's name fits the available label band
- **THEN** the row paints the full name with no ellipsis

#### Scenario: An overlong name elides once [lpr_name_elide]

- **WHEN** a layer's name is wider than the available label band
- **THEN** the row paints the name middle-elided to the band, keeping its start
  and end with a single ellipsis between them
