## MODIFIED Requirements

### Requirement: Hue/Saturation adjustment

The system SHALL implement `Adjustment::HueSaturation(HueSaturationParams { hue, saturation, lightness })`. Hue SHALL default to 0 and span `-180..=180`; saturation SHALL default to 0 and span `-100..=100`; lightness SHALL default to 0 and span `-100..=100`. It SHALL convert each pixel to HSL, rotate hue by the hue value, scale saturation by `1 + saturation/100`, and convert back at the pixel's HSL lightness; it SHALL then blend each RGB channel toward white by `lightness/100` for positive lightness (`c + l·(1 − c)`) and toward black for negative lightness (`c·(1 + l)`), so lightness never changes a pixel's hue or relative chroma. All-zero parameters SHALL be an exact identity, and saturation -100 with lightness 0 SHALL produce a neutral grey at the pixel's HSL lightness. This change covers the composite (Master) path only; the CPU kernels and the GPU shader SHALL implement the same model. Oracle expectation: ImageMagick `-modulate` uses its own HSL space (observed max delta 45), so property tests cover the identity and increased channel spread under positive saturation. The lightness model approximates Photoshop's closed kernel and is not claimed as parity.

#### Scenario: Neutral parameters are identity

- **WHEN** Hue/Saturation is applied with hue 0, saturation 0, and lightness 0
- **THEN** the buffer is bit-exactly unchanged

#### Scenario: Saturation minus 100 is grey

- **WHEN** Hue/Saturation is applied with saturation -100
- **THEN** all three color channels of every pixel are equal to the rounded HSL lightness

#### Scenario: Lightness keeps a near-neutral pixel near neutral

- **WHEN** Hue/Saturation is applied with lightness 66 to the dark, nearly neutral pixel `(10, 5, 8)`
- **THEN** each channel moves the same fraction toward white, giving `(172, 170, 171)`, and lightness ±100 gives pure white or black

### Requirement: Replace Color adjustment

The system SHALL implement `Adjustment::ReplaceColor(ReplaceColorParams { samples, fuzziness, localized, hue, saturation, lightness })`. `samples` is a list of `(x, y, rgb)` picks; `fuzziness` spans `0.0..=200.0`; `hue` spans `-180.0..=180.0` degrees; `saturation` and `lightness` span `-100.0..=100.0`. For each pixel the system SHALL compute a weight as the maximum over samples of a Chebyshev colour-distance ramp (exact match when fuzziness is 0), optionally attenuated by a Gaussian of the pixel-to-sample distance when `localized` is set (a sample with a negative position has no canvas position and is not attenuated), then blend the pixel toward its shifted result by that weight. The shifted result SHALL rotate the hue in HSL, scale each channel's distance from the HSL lightness by `1 / (1 − s)` for `s = saturation/100 ≥ 0` (bounded) or `1 + s` for `s < 0`, and then blend each channel toward white or black by `lightness/100` as Hue/Saturation does; an achromatic pixel therefore takes no hue or chroma. An empty sample list SHALL be an exact identity, alpha SHALL be preserved, and out-of-range or non-finite parameters SHALL be rejected as `AdjustError::InvalidParams`. The shift approximates Photoshop's closed kernel and is not claimed as parity.

#### Scenario: Zero fuzziness selects exact samples only

- **WHEN** Replace Color with fuzziness 0 and one sample is applied to a buffer
- **THEN** only pixels exactly equal to the sample colour change, and no others

#### Scenario: Fuzziness shifts near colours

- **WHEN** Replace Color with a positive fuzziness and a non-zero hue is applied to pixels near the sample colour
- **THEN** those pixels shift toward the new hue

#### Scenario: Empty samples is identity and alpha is preserved

- **WHEN** Replace Color is applied with an empty sample list, or to an RGBA buffer
- **THEN** the buffer is unchanged, and in the RGBA case the alpha plane is bit-identical

#### Scenario: Raising saturation keeps dark noise near neutral

- **WHEN** Replace Color with saturation 81 is applied to near-black noise pixels of different hues
- **THEN** each stays within 20 levels of neutral, while a dark brown deepens

#### Scenario: A gray cannot be replaced with a colour

- **WHEN** the dialog's Result colour is picked as blue for a black sample
- **THEN** only Lightness moves (hue 0, saturation 0, lightness 50), and the Result swatch and the canvas both show the gray `(128, 128, 128)`
