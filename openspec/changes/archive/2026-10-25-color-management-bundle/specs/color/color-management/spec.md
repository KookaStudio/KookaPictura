# Spec Delta

## ADDED Requirements

### Requirement: A bundled CMYK working profile is available

`pictura-color` SHALL expose the CC0 synthetic coated-offset CMYK profile bundled at `assets/icc/photocraft-coated-cmyk.icc` as `Profile::coated_cmyk()`, and SHALL expose its exact bytes as `Profile::coated_cmyk_icc()`. The loaded profile SHALL report four colour channels, SHALL round-trip through `Profile::to_icc` and `Profile::from_icc`, and SHALL support RGB-to-CMYK and CMYK-to-RGB transforms through Little CMS 2. The profile is a modelled press, not a measured vendor characterisation; parity is with lcms2, not Adobe.

#### Scenario: The CMYK profile loads

- **WHEN** `Profile::coated_cmyk()` is called
- **THEN** it returns a profile whose `channels()` is `Some(4)` and whose description is non-empty

#### Scenario: The CMYK profile serializes and reloads

- **WHEN** the bundled profile is serialized with `to_icc` and parsed with `from_icc`
- **THEN** the reloaded profile has four colour channels and the same description

### Requirement: RGB and CMYK convert through a profile pair

`pictura-color` SHALL expose an 8-bit colour-space conversion that accepts a Gray, RGB, or CMYK source profile and a Gray, RGB, or CMYK destination profile, converting interleaved samples through the profile connection space with a caller-supplied intent and black-point-compensation flag. The source and destination channel counts MAY differ. A buffer that is not a whole number of source pixels SHALL return a typed error without panicking; a profile whose colour space is not Gray, RGB, or CMYK SHALL return a typed error; alpha is not a colour channel and is not accepted by this conversion.

#### Scenario: sRGB converts to the CMYK working profile

- **WHEN** white, black, and a saturated blue are converted from sRGB to `Profile::coated_cmyk()` with relative-colorimetric intent and black-point compensation on
- **THEN** white carries little or no ink, black carries heavy black ink, and the blue is more cyan than yellow

#### Scenario: CMYK converts back to sRGB

- **WHEN** a CMYK pixel produced by the working profile is converted back to sRGB
- **THEN** the result is within the lcms2 round-trip tolerance of the original sRGB pixel, and a highly saturated out-of-gamut source may clip

#### Scenario: A malformed buffer is refused

- **WHEN** the conversion is given a byte length that is not a whole number of source pixels
- **THEN** it returns a typed error and does not panic

### Requirement: Conversion options are shared by every profile operation

`pictura-color` SHALL expose `ConversionOptions` carrying a rendering intent, a black-point-compensation flag, and a dither flag. The same options SHALL drive the document Convert to Profile operation, the incoming-policy conversion on open, and the picker's working-CMYK evaluation. Dither SHALL be deterministic and SHALL be applied only to 8-bit conversions; a 16- or 32-bit conversion SHALL ignore it. The default SHALL be relative colorimetric, black-point compensation on, and dither on, matching the CS6 documented defaults; callers that need the historical no-BPC behaviour SHALL pass it explicitly.

#### Scenario: Every intent produces a transform

- **WHEN** the same RGB buffer is converted with each of the four rendering intents
- **THEN** each run returns the expected byte length and more than eight distinct pixel values

#### Scenario: Black-point compensation reaches the transform

- **WHEN** a conversion is run with black-point compensation on and off
- **THEN** the two outputs differ for a shadow range and the on case matches an lcms2 transform built with the black-point flag

#### Scenario: Dither is deterministic and 8-bit only

- **WHEN** the same 8-bit conversion is run twice with dither on
- **THEN** the two outputs are byte-identical and differ from the undithered output; a 16-bit conversion with dither on equals the undithered 16-bit output
