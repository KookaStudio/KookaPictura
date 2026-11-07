## MODIFIED Requirements

### Requirement: Native quantization and colour math use the store's scale

Posterize SHALL quantize to `round(v * (levels - 1)) / (levels - 1)` at the
store's own resolution, not to a 1/255 grid, and `levels == 255` SHALL remain
the identity. Black & White and Channel Mixer SHALL compute in the unit domain
and SHALL NOT round their result through an 8-bit grid before encoding; on a
`u8` store they SHALL nevertheless match the existing 8-bit kernels
byte-for-byte, including the Black & White tint path. Both SHALL clamp their
output to `[0, 1]` before encoding, and Black & White SHALL clamp the computed
grey to `[0, 1]` before applying the tint.

A CMYK Channel Mixer on a 16/32-bit document SHALL mix in the f32 unit ink
domain using the profile-free GCR conversion and SHALL NOT route through the
8-bit profile conversion.

#### Scenario: 16-bit Posterize uses native levels [nda_posterize_native]

- **WHEN** Posterize with `levels == 5` is applied to a depth-16 store spanning
  the full range, and separately with `levels == 255`
- **THEN** the first output uses the native levels `0`, `16384`, `32768`,
  `49151`, and `65535`, and the second is unchanged

#### Scenario: 16-bit Black & White and Channel Mixer keep native precision [nda_bw_cm_native]

- **WHEN** Black & White with weights that drive the grey past its range, or a
  non-identity Channel Mixer, are applied to a depth-16 store
- **THEN** the Black & White grey clamps at the store's range before tinting, and
  in either case at least one output sample's low byte differs from its high
  byte, i.e. the result is not constrained to the 8-bit widening `high * 257`

#### Scenario: 16-bit CMYK mixing keeps native precision [nda_cm_cmyk_native]

- **WHEN** a non-identity CMYK Channel Mixer is applied to a depth-16 CMYK document
- **THEN** the result uses the f32 GCR path and at least one output sample's low
  byte differs from its high byte
