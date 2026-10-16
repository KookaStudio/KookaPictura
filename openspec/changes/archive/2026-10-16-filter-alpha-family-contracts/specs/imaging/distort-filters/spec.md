# Spec Delta

## Purpose

A Distort warp moves the layer edge with the colour instead of leaving
channel 4 untouched (#406).

## ADDED Requirements

### Requirement: Distort alpha follows the kernel

For 4-channel buffers, every Distort filter SHALL resample channel 4 through the same warp as the colour planes, so the layer edge moves with the colour, and a channel 4 that is 255 everywhere SHALL stay 255.

#### Scenario: A warp moves a soft edge

- **WHEN** a Distort variant is applied to an RGBA buffer whose alpha is a soft-edged disc
- **THEN** the alpha plane differs from the input alpha plane

#### Scenario: An opaque buffer stays opaque

- **WHEN** a Distort variant is applied to an RGBA buffer whose alpha is 255 everywhere
- **THEN** every alpha sample is still 255

## REMOVED Requirements

### Requirement: Alpha channel preservation for Distort filters

**Reason**: It held channel 4 bit-identical for the whole family, which
contradicts the `imaging/filter-application` alpha contract and the fold
(#248): a kernel on `filters_alpha` runs over channel 4 so the layer edge
moves with the colour.

**Migration**: Replaced by "Distort alpha follows the kernel", which names the variants that move
channel 4 and those that keep it bit-identical.
