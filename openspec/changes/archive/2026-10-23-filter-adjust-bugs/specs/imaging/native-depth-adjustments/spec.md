## MODIFIED Requirements

### Requirement: Tonal adjustments apply at native depth

`pictura_adjust` SHALL expose an application entry that applies an `Adjustment`
to a native-depth `Samples` store (`u8`, `u16`, or `f32`) without first
quantizing to 8-bit, given the buffer's width, height, and channel count. For a
`u16` or `f32` store the per-sample math SHALL run in a unit-domain `f64`
representation and the result SHALL be encoded back to the store's depth, so
the output precision is the store's, not 8-bit. A depth-32 store holds linear
samples: Exposure SHALL apply `c * 2^exposure`, then `+ offset`, clamp at zero,
then `gamma` directly, without an sRGB decode or encode. The existing sample
conversion clamps an `f32` input to `[0, 1]`; `f32::from_unit` leaves an output
above `1.0` representable. A `u8` or `u16` store holds sRGB-encoded samples:
Exposure SHALL decode to linear and re-encode.

#### Scenario: A 16-bit Levels edit keeps 16-bit precision

- **WHEN** a tonal adjustment is applied to a depth-16 `Samples` store whose
  8-bit narrowing is unchanged by the edit
- **THEN** at least one sample's low byte differs from its high byte, i.e. the
  result is not the 8-bit widen `high * 257`

#### Scenario: A 32-bit store is adjusted in unit space

- **WHEN** an `Exposure` adjustment is applied to a depth-32 `Samples` store
- **THEN** the samples are adjusted in `[0, 1]` and values above `1.0` remain
  representable (not clipped by an 8-bit round-trip)

#### Scenario: 32-bit Exposure is linear [nda_exposure_linear]

- **WHEN** Exposure `+1` EV is applied to a depth-32 `Samples` store holding a
  linear `0.5`
- **THEN** the output is `1.0` (within float tolerance), not the value obtained
  by an extra sRGB decode/encode round trip

### Requirement: The 8-bit application is unchanged

The existing 8-bit `apply(adjustment, &mut PixelBuffer<u8>)` SHALL keep its
current kernels, rounding, and results, except for the documented neutral-
saturation Vibrance correction (`imaging/image-adjustments`) and the
selection-aware Auto statistics (`imaging/image-adjustments`). For each covered
tonal adjustment the native entry applied to a `u8` store SHALL produce the same
bytes as `apply` on the same input.

#### Scenario: Native u8 equals the existing apply

- **WHEN** each covered adjustment is applied to a fixed `u8` image through both
  `apply` and the native entry
- **THEN** the outputs are byte-identical

#### Scenario: The ImageMagick oracle is unmoved

- **WHEN** the `pictura-adjust` oracle tests run after this change
- **THEN** they pass with no golden or tolerance change

## ADDED Requirements

### Requirement: Native quantization and colour math use the store's scale

Posterize SHALL quantize to `round(v * (levels - 1)) / (levels - 1)` at the
store's own resolution, not to a 1/255 grid, and `levels == 255` SHALL remain
the identity. Black & White and Channel Mixer SHALL compute in the unit domain
and SHALL NOT round their result through an 8-bit grid before encoding; on a
`u8` store they SHALL nevertheless match the existing 8-bit kernels
byte-for-byte, including the Black & White tint path. Both SHALL clamp their
output to `[0, 1]` before encoding, and Black & White SHALL clamp the computed
grey to `[0, 1]` before applying the tint.

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
