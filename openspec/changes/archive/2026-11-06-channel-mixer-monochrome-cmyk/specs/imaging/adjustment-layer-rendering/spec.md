## ADDED Requirements

### Requirement: CMYK Channel Mixer rows decode and encode

The `mixr` payload SHALL be read as four 10-byte rows, each holding four
big-endian `i16` sources and a constant. The existing RGB interpretation SHALL
be unchanged: rows 0..3 are Red, Green, Blue, and Gray, the fourth source slot
is the reserved pair, and the RGB rows and constants come from rows 0..3's
first three sources. The CMYK interpretation SHALL use all four sources of each
row: rows 0..3 are Cyan, Magenta, Yellow, and Black outputs over C/M/Y/K
sources, so the bytes ag-psd skips as reserved carry the K source.

`pictura-render` SHALL expose `encode_channel_mixer_ink(ink: [[f64; 4]; 4],
ink_constant: [f64; 4]) -> AdjustmentData` writing the CMYK form, and
`decode_adjustment` SHALL fill both the RGB rows and the `ink` rows from the
same bytes. The CMYK layout is inferred: no independent decoder models it, so
the tests SHALL use round-trip and hand-built bytes and the spec SHALL NOT claim
ag-psd verification for the CMYK form.

#### Scenario: Encoded CMYK rows decode back [alr_cm_cmyk_roundtrip]

- **WHEN** `encode_channel_mixer_ink` is given the rows `[20,0,0,0]`,
  `[0,30,0,0]`, `[0,0,40,0]`, `[0,0,0,50]` and constants `[5,-10,15,-20]`
- **THEN** `decode_adjustment` returns those `ink` rows and `ink_constant` values

#### Scenario: The fourth source slot is the K source [alr_cm_cmyk_k_source]

- **WHEN** a hand-built `mixr` block carries a non-zero fourth source in a row
- **THEN** the decoded row's Black source equals that value

#### Scenario: Out-of-range CMYK values are clamped [alr_cm_cmyk_clamp]

- **WHEN** `encode_channel_mixer_ink` is given a weight or constant above 200 or below -200
- **THEN** the encoded value is clamped into `-200..=200` and the block still decodes

### Requirement: CMYK Channel Mixer applies in ink space

For a non-monochrome Channel Mixer on a CMYK document, the compositor SHALL
convert the working RGB backdrop to unit CMYK, apply the ink matrix, and convert
back before gating the adjusted canvas through the layer's mask, opacity, and
blend. The conversion SHALL use the document's embedded four-channel CMYK
profile when the document is 8-bit and the profile parses, and the profile-free
GCR conversion otherwise. The destructive Image > Adjustments path SHALL apply
the same conversion to the active layer's region before the existing coverage
write-back. A monochrome mixer SHALL keep the RGB monochrome path in every
document mode. A 16/32-bit CMYK document SHALL use the f32 GCR path so the
result is not quantized to 8 bits.

#### Scenario: A CMYK mixer layer changes a CMYK backdrop [alr_cm_cmyk_composite]

- **WHEN** a Channel Mixer adjustment layer with a non-identity Cyan row is composited over a non-uniform CMYK document
- **THEN** the composite differs from the backdrop-only composite and stays in range

#### Scenario: The destructive path mixes CMYK [alr_cm_cmyk_destructive]

- **WHEN** a non-identity CMYK Channel Mixer is applied destructively to a CMYK document's layer
- **THEN** the layer's pixels change and the selection coverage still confines the edit

#### Scenario: 16-bit CMYK stays unquantized [alr_cm_cmyk_native]

- **WHEN** a non-identity CMYK mixer is applied to a depth-16 CMYK document
- **THEN** at least one output sample's low byte differs from its high byte

#### Scenario: Monochrome in a CMYK document uses the RGB path [alr_cm_cmyk_mono_rgb]

- **WHEN** a monochrome Channel Mixer is applied to a CMYK document
- **THEN** every output pixel has equal red, green, and blue values

## MODIFIED Requirements

### Requirement: The app can create a Channel Mixer adjustment layer

The app SHALL map the adjustment kind `channel-mixer` to a `mixr` adjustment
layer carrying the neutral identity default. In an RGB document that is
monochrome off, the rows `[100,0,0]`/`[0,100,0]`/`[0,0,100]`, and constants
`[0,0,0]`; in a CMYK document it is the 4×4 ink identity and zero ink
constants. The Adjustments panel menu SHALL offer a `Channel Mixer` entry that
dispatches `adjustment:channel-mixer`. Because the identity rows leave the
matrix unchanged, the default layer SHALL leave the backdrop unchanged. A `mixr`
layer with a non-identity row SHALL change the composite over a non-uniform
backdrop.

#### Scenario: The channel-mixer kind becomes an adjustment layer

- **WHEN** the app adds an adjustment layer of kind `channel-mixer`
- **THEN** the new layer carries a `mixr` block and is reported as an adjustment layer

#### Scenario: The default Channel Mixer layer is neutral

- **WHEN** a `channel-mixer` adjustment layer with the default parameters is composited over a backdrop
- **THEN** the result equals the backdrop-only composite

#### Scenario: The panel menu offers Channel Mixer

- **WHEN** the Adjustments panel menu is built
- **THEN** it contains a `Channel Mixer` row dispatching `adjustment:channel-mixer`

#### Scenario: A non-neutral Channel Mixer layer changes the composite

- **WHEN** a Channel Mixer adjustment layer with a non-identity row is composited over a non-uniform backdrop
- **THEN** the result differs from the backdrop-only composite

#### Scenario: A CMYK default is the ink identity [alr_cm_cmyk_default]

- **WHEN** the app adds a `channel-mixer` adjustment layer in a CMYK document
- **THEN** the block carries the 4×4 ink identity and the default layer leaves the backdrop unchanged
