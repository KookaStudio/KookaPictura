# Spec Delta

## ADDED Requirements

### Requirement: Clear zeroes RGB at full coverage

A `Clear` stroke at full opacity and full coverage SHALL write the pixel's RGB
to 0 as well as reducing its alpha to 0. At partial coverage the existing
colour SHALL remain, so a partly transparent pixel keeps a defined colour.

#### Scenario: Clear zeroes RGB at full coverage [pe_clear_zeroes_rgb]

- **WHEN** a Clear stroke paints at full opacity and full coverage on an opaque
  coloured pixel
- **THEN** the pixel's alpha becomes 0 and its RGB becomes 0

#### Scenario: Partial Clear keeps the colour [pe_clear_keeps_rgb]

- **WHEN** a Clear stroke paints at 50 % opacity on an opaque coloured pixel
- **THEN** the pixel's alpha falls but its RGB keeps the pre-stroke colour

### Requirement: Stroke dirtiness follows changed pixels

A stroke SHALL be dirty only for pixels whose stored bytes differ from their
pre-stroke values. Coverage that leaves every stored byte unchanged — Behind
over an opaque pixel, Opacity 0, Clear on a layer without alpha, or a clone
source outside the image — SHALL produce no dirty rectangle and therefore no
history state.

#### Scenario: A stroke that changes no pixel is a no-op [pe_noop_no_state]

- **WHEN** a Behind stroke paints over an opaque pixel, or a Normal stroke runs
  at Opacity 0
- **THEN** the stroke returns no dirty rectangle, adds no history state, and
  leaves the document not dirty
