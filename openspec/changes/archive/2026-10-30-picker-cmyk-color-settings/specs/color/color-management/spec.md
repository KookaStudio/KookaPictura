## MODIFIED Requirements

### Requirement: Conversion options are shared by every profile operation

`pictura-color` SHALL expose `ConversionOptions` carrying a rendering intent, a
black-point-compensation flag, and a dither flag. The rendering intent and the
black-point-compensation flag SHALL drive the document Convert to Profile
operation, the incoming-policy conversion on open, and the picker's working-CMYK
evaluation. Dither SHALL be deterministic and SHALL be applied only to 8-bit
conversions; a 16- or 32-bit conversion SHALL ignore it, and dither SHALL NOT
apply to the picker, which converts one colour rather than a plane. The default
SHALL be relative colorimetric, black-point compensation on, and dither on,
matching the CS6 documented defaults; callers that need the historical no-BPC
behaviour SHALL pass it explicitly.

#### Scenario: Every intent produces a transform

- **WHEN** the same RGB buffer is converted with each of the four rendering intents
- **THEN** each run returns the expected byte length and more than eight distinct pixel values

#### Scenario: Black-point compensation reaches the transform

- **WHEN** a conversion is run with black-point compensation on and off
- **THEN** the two outputs differ for a shadow range and the on case matches an lcms2 transform built with the black-point flag

#### Scenario: Dither is deterministic and 8-bit only

- **WHEN** the same 8-bit conversion is run twice with dither on
- **THEN** the two outputs are byte-identical and differ from the undithered output; a 16-bit conversion with dither on equals the undithered 16-bit output

#### Scenario: The picker evaluates with the shared intent and BPC [picker_shared_options]

- **WHEN** the shared options carry a rendering intent and a black-point-compensation flag
- **THEN** the picker's CMYK conversion uses that intent and flag, and its dither flag is ignored
