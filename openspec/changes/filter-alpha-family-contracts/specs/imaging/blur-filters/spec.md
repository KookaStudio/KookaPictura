# Spec Delta

## Purpose

The blur family's alpha contract follows the fold: a blur spreads the layer
edge with the colour instead of leaving channel 4 untouched (#406).

## ADDED Requirements

### Requirement: Blur alpha follows the kernel

For 4-channel buffers, every blur filter SHALL run its kernel over channel 4 as it does over the colour planes, so the layer edge spreads with the colour, and a channel 4 that is 255 everywhere SHALL stay 255. Smart Blur SHALL do so in Normal mode only, averaging channel 4 over the same taps its colour threshold admits; its Edge Only and Overlay Edge modes SHALL leave channel 4 bit-identical. Lens Blur and the Blur Gallery's Blur Effects and Noise passes SHALL apply their highlight and noise to channels 1 through 3 only; channel 4 takes the blur alone.

#### Scenario: A blur moves a soft edge

- **WHEN** a blur variant is applied to an RGBA buffer whose alpha is a soft-edged disc
- **THEN** the alpha plane differs from the input alpha plane

#### Scenario: An opaque buffer stays opaque

- **WHEN** a blur variant is applied to an RGBA buffer whose alpha is 255 everywhere
- **THEN** every alpha sample is still 255

#### Scenario: Smart Blur edge modes keep alpha

- **WHEN** Smart Blur in Edge Only or Overlay Edge mode is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit

#### Scenario: Lens Blur alpha has no bloom or grain

- **WHEN** Lens Blur with specular highlights and noise is applied to an RGBA buffer
- **THEN** the alpha plane equals the same Lens Blur, highlights and noise off, applied to that plane as colour

## REMOVED Requirements

### Requirement: Alpha channel preservation for blur

**Reason**: It held channel 4 bit-identical for the whole family, which
contradicts the `imaging/filter-application` alpha contract and the fold
(#248): a kernel on `filters_alpha` runs over channel 4 so the layer edge
moves with the colour.

**Migration**: Replaced by "Blur alpha follows the kernel", which names the variants that move
channel 4 and those that keep it bit-identical.
