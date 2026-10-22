# Spec Delta

## MODIFIED Requirements

### Requirement: Colour bit depth

Each colour block SHALL keep an independent bit depth of 8, 16, or 32,
selectable from the depth entries of its colour menu, and its footer SHALL read
`8-bit`, `16-bit`, or `32-bit`. Depth SHALL scale the channel values of Actual
Color, Proof Color, RGB, and Grayscale: 8-bit raw 0–255, 16-bit `v × 257`
(0–65535), and 32-bit SHALL read the current 32-bit source for that pixel and
show its unclamped value to three decimals. The current source is the retained
`f32` store while it is current and the current composite after an edit; a
document without a current `f32` source SHALL fall back to `v / 255.0` from the
8-bit sample. The colour-sampler rows SHALL use the same 32-bit source and SHALL
average the same sample-size box as their 8-bit path. HSB, CMYK, Lab, Total Ink,
and Opacity SHALL be depth-independent. `ponytail:` a document without a
retained `f32` store, or a converted mode, still scales the 8-bit sample for the
32-bit readout; HSB/CMYK/Lab remain derived from the 8-bit sample.

#### Scenario: 16-bit scales an RGB channel [info_bit_depth_16]

- **WHEN** the top-left block is at 16-bit over a white pixel
- **THEN** the `R` value is `65535` and the footer reads `16-bit`

#### Scenario: Depth does not change percentages [info_bit_depth_independent]

- **WHEN** a block is set to CMYK at 16-bit
- **THEN** the C/M/Y/K values stay 0–100 percentages and only the footer changes

#### Scenario: 32-bit reads the current float sample [info_bit_depth_32_float]

- **WHEN** a 32-bit document has a current colour sample above `1.0` and the top-left block is set to 32-bit over that pixel
- **THEN** the readout shows the unclamped float value with three decimals rather than a value scaled from the 8-bit composite

#### Scenario: A sampler uses the float source [info_bit_depth_32_sampler]

- **WHEN** a colour sampler sits over an unclamped 32-bit pixel and the block is at 32-bit
- **THEN** the sampler row reports the same float source, using its configured sample-size average

#### Scenario: 32-bit falls back without a float store [info_bit_depth_32_fallback]

- **WHEN** a document reports 32-bit depth but has no current `f32` colour source
- **THEN** the 32-bit readout uses `v / 255.0` from the 8-bit sample and the panel does not crash
