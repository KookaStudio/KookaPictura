# Spec Delta

## Purpose

The noise family's rank filters move the layer edge with the colour; Add
Noise still leaves it untouched (#406).

## ADDED Requirements

### Requirement: Noise alpha follows the kernel

For 4-channel buffers, Median, Despeckle, and Dust & Scratches SHALL run their kernel over channel 4 as they do over the colour planes, and a channel 4 that is 255 everywhere SHALL stay 255. Add Noise SHALL leave channel 4 bit-identical; only channels 1 through 3 SHALL be modified.

#### Scenario: A rank filter moves a soft edge

- **WHEN** Median, Despeckle, or Dust & Scratches is applied to an RGBA buffer whose alpha is a soft-edged disc with a stray speck
- **THEN** the alpha plane differs from the input alpha plane

#### Scenario: Add Noise preserves alpha

- **WHEN** Add Noise is applied to an RGBA buffer
- **THEN** the alpha plane equals the input alpha plane bit for bit

## REMOVED Requirements

### Requirement: Alpha channel preservation for noise

**Reason**: It held channel 4 bit-identical for the whole family, which
contradicts the `imaging/filter-application` alpha contract and the fold
(#248): a kernel on `filters_alpha` runs over channel 4 so the layer edge
moves with the colour.

**Migration**: Replaced by "Noise alpha follows the kernel", which names the variants that move
channel 4 and those that keep it bit-identical.
