# Spec Delta

## Purpose

The sharpen family's alpha contract follows the fold: a sharpen moves the
layer edge with the colour instead of leaving channel 4 untouched (#406).

## ADDED Requirements

### Requirement: Sharpen alpha follows the kernel

For 4-channel buffers, every sharpen filter SHALL run its kernel over channel 4 as it does over the colour planes, so the layer edge sharpens with the colour, and a channel 4 that is 255 everywhere SHALL stay 255. Sharpen Edges SHALL sharpen channel 4 only where its colour edge mask does.

#### Scenario: A sharpen moves a soft edge

- **WHEN** a sharpen variant is applied to an RGBA buffer whose alpha is a soft-edged disc
- **THEN** the alpha plane differs from the input alpha plane

#### Scenario: An opaque buffer stays opaque

- **WHEN** a sharpen variant is applied to an RGBA buffer whose alpha is 255 everywhere
- **THEN** every alpha sample is still 255

## REMOVED Requirements

### Requirement: Alpha channel preservation for sharpen

**Reason**: It held channel 4 bit-identical for the whole family, which
contradicts the `imaging/filter-application` alpha contract and the fold
(#248): a kernel on `filters_alpha` runs over channel 4 so the layer edge
moves with the colour.

**Migration**: Replaced by "Sharpen alpha follows the kernel", which names the variants that move
channel 4 and those that keep it bit-identical.
