## MODIFIED Requirements

### Requirement: Channel Mixer adjustment

The system SHALL implement `Adjustment::ChannelMixer(ChannelMixerParams)` with
the monochrome flag, the RGB rows `red`/`green`/`blue` over three sources, the
RGB constants `constant`, the CMYK rows `ink` over four sources, and the CMYK
constants `ink_constant`. Every weight and constant SHALL be a percentage no
greater than 200 in absolute value.

In RGB the system SHALL compute, per output channel,
`out = (w[0]*R + w[1]*G + w[2]*B) / 100 + constant/100 * 255`, clamped, with
the identity matrix `[100, 0, 0]`, `[0, 100, 0]`, `[0, 0, 100]` and zero
constants. Monochrome SHALL use the red row as one source mix for all three
outputs.

In CMYK the system SHALL compute, per output channel `c`,
`out_c = clamp(sum_s(w_cs/100 * in_s) - constant_c/100, 0, 1)` on unit ink,
where `0` is no ink and `1` is full ink, with the 4×4 identity and zero
constants. A negative constant therefore adds ink (black) and a positive
constant removes it (white). The system SHALL reject a weight or constant
outside `-200..=200`.

Oracle expectation: ImageMagick `-color-matrix` takes fractions while Photoshop
uses percent weights (observed max delta 252), so property tests cover the exact
identities, a hand-computed RGB blend, and a hand-computed CMYK blend.

#### Scenario: Identity matrix is a no-op

- **WHEN** the identity matrix is applied
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Known blend

- **WHEN** the red row is set to sources `[50, 100, 0]` and the pixel is `(50, 100, 200)`
- **THEN** the red output is 125 and green and blue are unchanged

#### Scenario: CMYK identity is a no-op [ia_cm_cmyk_identity]

- **WHEN** the 4×4 CMYK identity is applied to unit ink samples
- **THEN** every sample is bit-exactly unchanged

#### Scenario: CMYK known blend [ia_cm_cmyk_blend]

- **WHEN** the Cyan row is `[0, 100, 0, 0]` and the input ink is C 0.5, M 0.25, Y 0, K 0
- **THEN** the Cyan output is 0.25 and Magenta, Yellow, and Black are unchanged

#### Scenario: CMYK constant adds or removes ink [ia_cm_cmyk_constant]

- **WHEN** the Cyan constant is -50 and then +50 on an input Cyan of 0.25
- **THEN** the first output is 0.75 and the second clamps to 0

#### Scenario: Out-of-range CMYK weights are rejected [ia_cm_cmyk_invalid]

- **WHEN** a CMYK weight or constant is 201 or -201
- **THEN** the CMYK kernel returns `AdjustError::InvalidParams` and leaves the samples unchanged
