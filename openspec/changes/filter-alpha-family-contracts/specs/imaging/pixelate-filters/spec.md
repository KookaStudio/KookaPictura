# Spec Delta

## Purpose

Mosaic, Crystallize, and Fragment move the layer edge with the colour; the
value-clustering Pixelate filters keep it (#406).

## ADDED Requirements

### Requirement: Pixelate alpha follows the kernel

For 4-channel buffers, Mosaic, Crystallize, and Fragment SHALL run their kernel over channel 4 as they do over the colour planes, and a channel 4 that is 255 everywhere SHALL stay 255. Facet, Mezzotint, Pointillize, and Color Halftone SHALL leave channel 4 bit-identical; only channels 1 through 3 SHALL be modified.

#### Scenario: Mosaic, Crystallize, and Fragment move a soft edge

- **WHEN** Mosaic, Crystallize, or Fragment is applied to an RGBA buffer whose alpha is a soft-edged disc
- **THEN** the alpha plane differs from the input alpha plane

#### Scenario: The value-clustering variants preserve alpha

- **WHEN** Facet, Mezzotint, Pointillize, or Color Halftone is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit

## REMOVED Requirements

### Requirement: Alpha channel preservation for Pixelate filters

**Reason**: It held channel 4 bit-identical for the whole family, which
contradicts the `imaging/filter-application` alpha contract and the fold
(#248): a kernel on `filters_alpha` runs over channel 4 so the layer edge
moves with the colour.

**Migration**: Replaced by "Pixelate alpha follows the kernel", which names the variants that move
channel 4 and those that keep it bit-identical.
