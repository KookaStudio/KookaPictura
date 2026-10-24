# native-depth-adjustments Specification

## Purpose
Tonal adjustments applied at native depth, with the 8-bit path unchanged and native edits surviving a round-trip.

## Requirements

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

### Requirement: Covered adjustments and the unsupported ceiling

The native entry SHALL support `Invert`, `Desaturate`, `Levels`, `Curves`,
`BrightnessContrast`, `Exposure`, `Posterize`, `Threshold`, `GradientMap`,
`HueSaturation`, `Vibrance`, `ColorBalance`, `BlackWhite`, `PhotoFilter`,
`ChannelMixer`, `SelectiveColor`, `Auto`, and `ColorLookup`. For any other
`Adjustment` — the generative fill kinds — it SHALL return
`AdjustError::Unsupported` without mutating the store.

#### Scenario: A covered color adjustment applies at native depth

- **WHEN** the native entry is called with `HueSaturation` on a depth-16 store
- **THEN** it succeeds and each sample's low byte is not constrained to the
  8-bit widen

#### Scenario: Auto applies at native depth

- **WHEN** the native entry is called with `Auto(AutoKind::Color)` on a depth-16
  store
- **THEN** it succeeds without an 8-bit round-trip

#### Scenario: An unsupported adjustment is refused without mutation

- **WHEN** the native entry is called with a `SolidFill`/`GradientFill`/
  `PatternFill` adjustment on a depth-16 store
- **THEN** it returns `Unsupported` and the samples are unchanged

### Requirement: A native edit survives a PSD round-trip

A depth-16 document read from PSD, adjusted at native depth, and written SHALL
re-emit the adjusted native samples, and reading the output SHALL yield the same
native samples (byte-identically at depth 16).

#### Scenario: Edit, save, re-read keeps the low bits

- **WHEN** a depth-16 RGB document is read, a covered tonal adjustment is applied
  to its retained native composite, the 8-bit composite is set to the narrowing,
  and the document is written and re-read
- **THEN** the re-read retained samples equal the adjusted native samples and are
  not the 8-bit widening of the edited composite

### Requirement: Sample conversion is total and clamped

The unit-domain conversion SHALL be total: `f32` inputs below `0.0` SHALL map to
`0.0`, above `1.0` to `1.0`, and `NaN` to `0.0`, so no adjustment can produce a
non-finite output from a finite store.

#### Scenario: Out-of-range f32 maps into the unit interval

- **WHEN** a depth-32 store holds `-0.5`, `1.5`, and `NaN`
- **THEN** their unit values are `0.0`, `1.0`, and `0.0` respectively

### Requirement: Auto and ColorLookup keep native precision

`Auto` and `ColorLookup` SHALL be applied at the store's own resolution: the
`Auto` histogram and stretch SHALL use the store's level count (`256` for `u8`,
`65536` for `u16`, a `65536`-bin `[0,1]` histogram for `f32`), and `ColorLookup`
SHALL sample its 3-D LUT on unit values without an intermediate 8-bit
quantization. A `ColorLookup` payload that is abstract-profile, device-link, or
otherwise unparseable SHALL remain a no-op as in the 8-bit path.

#### Scenario: A 16-bit ColorLookup keeps precision

- **WHEN** a non-identity parsed 3-D LUT is applied to a depth-16 store through
  the native entry
- **THEN** at least one output sample is not the 8-bit widening of the input

#### Scenario: Native Auto equals the 8-bit Auto on a u8 store

- **WHEN** `Auto(Tone)`, `Auto(Contrast)`, and `Auto(Color)` are applied to a
  fixed `u8` store through both `apply` and `apply_native`
- **THEN** the outputs are byte-identical

#### Scenario: An unparseable ColorLookup is a no-op

- **WHEN** the native entry is called with a `ColorLookup` whose payload cannot
  be parsed
- **THEN** it succeeds and the samples are unchanged

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
