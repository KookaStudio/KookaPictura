# Spec Delta

## MODIFIED Requirements

### Requirement: Destructive adjustment application contract

The system SHALL provide `pictura_adjust::apply(adjustment: &Adjustment, buf: &mut PixelBuffer) -> Result<(), AdjustError>`. `Adjustment` SHALL contain only destructive operations; generative Solid,
Gradient, and Pattern content SHALL belong to `FillContent` and SHALL NOT be
accepted by this interface. For valid parameters and a supported buffer, every
`Adjustment` variant SHALL return `Ok(())`; unsupported input channel counts
remain errors. It SHALL operate in place on a planar 8-bit buffer whose `channels` is 3 (RGB) or 4 (RGBA), treating channels 1, 2, and 3 as the composite color planes and channel 4, when present, as alpha. It SHALL transform every color sample or return an error; it MUST NOT partially apply an adjustment and then fail.

#### Scenario: Apply to a 3-channel planar buffer

- **WHEN** `apply` receives any `Adjustment` with valid parameters and a 3-channel planar buffer
- **THEN** the R, G, and B planes are rewritten in place and `Ok(())` is returned

#### Scenario: Reject an unsupported channel count

- **WHEN** the buffer has a channel count other than 3 or 4
- **THEN** `apply` returns `AdjustError::Unsupported` and leaves the buffer unchanged

### Requirement: Alpha channel preservation

For 4-channel buffers, `apply` SHALL leave channel 4 bit-identical whenever it
succeeds. Only channels 1 through 3 SHALL be modified. Generative fills SHALL
NOT be representable as an `Adjustment` argument.

#### Scenario: All adjustment variants preserve alpha

- **WHEN** each destructive adjustment variant is passed to `apply` with valid parameters and an RGBA buffer
- **THEN** the call succeeds and the alpha plane equals the input alpha plane bit for bit

### Requirement: Parameter validation and error signaling

`apply` SHALL return `AdjustError` instead of panicking when parameters are out of range or buffers are malformed. It SHALL reject a zero-pixel buffer and a buffer whose `data.len()` does not equal `width * height * channels`. Each parameterized adjustment SHALL validate its documented ranges: Levels gamma greater than zero and input black below input white; Curves at least 2 and at most 14 points with strictly increasing inputs; Brightness/Contrast brightness in `-150..=150` and contrast in `-50..=100`; Exposure gamma greater than zero and exposure/offset finite; Posterize levels in `2..=255`; Threshold level in `1..=255`; Hue/Saturation hue in `-180..=180` and saturation/lightness in `-100..=100`; Black & White weights in `-200..=300` percent; Photo Filter density in `0..=100`; Channel Mixer weights and constant no greater than 200 percent in absolute value; Vibrance and saturation in `-100..=100`; Color Balance values finite and within `-100..=100`; Selective Color corrections in `-100..=100`; Gradient Map at least two stops with strictly increasing locations each at most 4096.

#### Scenario: Out-of-range parameters return an error

- **WHEN** an adjustment is given a value outside its documented range, such as Levels gamma `0.0`, Curves with one point, Brightness/Contrast brightness `200`, Exposure gamma `0.0`, Posterize `1`, Threshold `0`, Hue/Saturation hue `200`, Black & White weight `999`, Photo Filter density `150`, Channel Mixer weight `999`, Vibrance `500`, Color Balance `200`, Selective Color correction `200`, Gradient Map with one stop or a location above 4096
- **THEN** `apply` returns `AdjustError::InvalidParams` or `AdjustError::Unsupported` and does not panic

#### Scenario: Malformed buffers error instead of panicking

- **WHEN** `apply` receives a zero-pixel buffer, a 2-channel buffer, or a buffer whose data length does not match `width * height * channels`
- **THEN** `apply` returns `AdjustError` and does not panic

## REMOVED Requirements

### Requirement: Gradient Fill adjustment model

**Reason**: Generative gradient content is not a destructive adjustment.
**Migration**: Use `FillContent::Gradient(GradientFillParams)` under `document/layer-content`; the geometry, stops, defaults, and rendering contract are retained there and in `imaging/adjustment-layer-rendering`.

### Requirement: Pattern Fill adjustment model

**Reason**: Generative pattern content is not a destructive adjustment.
**Migration**: Use `FillContent::Pattern(PatternFillParams)` under `document/layer-content`; the pattern reference, scale, anchoring, origin, defaults, and rendering contract are retained there and in `imaging/adjustment-layer-rendering`.
