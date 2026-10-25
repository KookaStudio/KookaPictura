# color-management Specification

## Purpose
Runtime-built working-space profiles, ICC load and serialization, assign versus convert, and rendering intents.

## Requirements

### Requirement: Built-in working-space profiles are generated at runtime

The `pictura-color` crate SHALL provide built-in sRGB (IEC 61966-2.1), Adobe RGB
(1998), and ProPhoto RGB (ROMM RGB) profiles. These MUST be synthesized
programmatically from their white point, primaries, and tone curve at runtime;
the crate MUST NOT depend on bundled Adobe or third-party ICC profile files.

#### Scenario: sRGB built-in is valid

- **WHEN** `Profile::srgb()` is called
- **THEN** it returns a usable RGB profile with the sRGB color space signature

#### Scenario: Adobe RGB primaries

- **WHEN** `Profile::adobe_rgb()` is called
- **THEN** it returns a valid RGB profile built from the Adobe RGB (1998)
  primaries (D65 white, red 0.6400/0.3300, green 0.2100/0.7100, blue
  0.1500/0.0600) and gamma 563/256

#### Scenario: ProPhoto primaries

- **WHEN** `Profile::pro_photo()` is called
- **THEN** it returns a valid RGB profile built from the ROMM primaries (D50
  white, red 0.7347/0.2653, green 0.1596/0.8404, blue 0.0366/0.0001) with the
  ROMM tone curve

### Requirement: ICC profiles load from and serialize to bytes

The crate SHALL load an ICC profile from raw bytes via `Profile::from_icc` and
SHALL serialize a profile back to ICC bytes via `Profile::to_icc`. A profile
serialized by `to_icc` MUST reload through `from_icc` with the same color space
signature.

#### Scenario: Built-in profiles round-trip

- **WHEN** `to_icc()` is called on a built-in profile and the result is passed to
  `from_icc()`
- **THEN** the reloaded profile has the same color space signature as the original

#### Scenario: Empty serialization is avoided

- **WHEN** `to_icc()` is called on a valid profile
- **THEN** the returned byte vector is non-empty

### Requirement: Assign retags a profile without changing pixels

The `assign` operation SHALL return the input pixel buffer unchanged together
with the destination profile. It MUST NOT build a transform or alter any sample;
assignment only reinterprets the numeric values under a new tag.

#### Scenario: Pixels are byte-identical after assign

- **WHEN** an image buffer is assigned a new profile
- **THEN** the returned buffer equals the input buffer byte-for-byte and the
  returned profile is the assigned profile in the RGB color space

### Requirement: Convert transforms pixels between profiles

The `convert` function SHALL build a source-to-destination transform and apply it
per pixel, returning a buffer with the same channel count and bit depth as the
input. It SHALL error when the declared dimensions, channel count, and bit depth
do not match the input buffer length. For RGBA input the alpha channel MUST be
copied through unchanged.

#### Scenario: Output shape matches input

- **WHEN** `convert` is called on a well-formed buffer
- **THEN** the returned buffer has the same length and layout as the input buffer

#### Scenario: Length mismatch is an error

- **WHEN** the input buffer length does not equal
  `width × height × channels × (bits / 8)`
- **THEN** `convert` returns a `ColorError::Unsupported` error rather than
  reading out of bounds

#### Scenario: Alpha channel is preserved

- **WHEN** 4-channel RGBA input is converted between two profiles
- **THEN** every alpha byte in the output equals the corresponding input alpha
  byte

### Requirement: Rendering intents and black point compensation are accepted

`convert` SHALL accept all four ICC rendering intents — Perceptual, Relative
Colorimetric, Saturation, and Absolute Colorimetric — and a black point
compensation flag. Each intent MUST produce non-degenerate output, and enabling
black point compensation MUST set the Little CMS black-point-compensation flag.

#### Scenario: Every intent runs

- **WHEN** a conversion is run once per rendering intent
- **THEN** each run returns an output of the expected length with more than eight
  distinct pixel values

#### Scenario: Black point compensation is forwarded

- **WHEN** black point compensation is enabled
- **THEN** the transform is built with the black-point-compensation flag set

### Requirement: Bit depths and channel counts

`convert` SHALL support 8-bit, 16-bit, and 32-bit floating-point samples and
1-, 3-, and 4-channel interleaved input (gray, RGB, and RGBA respectively),
mapping gray/RGB/RGBA input into the destination color space. Any other
channel-count/bit-depth combination MUST return a `ColorError::Unsupported`.

#### Scenario: 16-bit input is preserved at 16-bit

- **WHEN** a 16-bit RGB buffer is converted
- **THEN** the output is the same number of 16-bit samples as the input

#### Scenario: 32-bit float input is preserved at 32-bit [cm_convert_32bit]

- **WHEN** a 32-bit float RGB buffer is converted
- **THEN** the output is the same number of 32-bit float samples as the input and the samples are finite

#### Scenario: Unsupported format is rejected

- **WHEN** `convert` is called with e.g. 2 channels or 12 bits
- **THEN** it returns a `ColorError::Unsupported`

### Requirement: Identity conversion is a no-op

Converting a profile to itself SHALL leave every sample within ±1 LSB of the
input, at both 8-bit and 16-bit depths.

#### Scenario: 8-bit identity

- **WHEN** an 8-bit image is converted from sRGB to sRGB
- **THEN** every output sample differs from its input by at most 1

#### Scenario: 16-bit identity

- **WHEN** a 16-bit image is converted from sRGB to sRGB
- **THEN** every output sample differs from its input by at most 1

### Requirement: Malformed ICC input returns an error, never panics

`Profile::from_icc` SHALL return `ColorError::InvalidProfile` for empty,
truncated, or non-ICC byte input. It MUST NOT panic on any byte sequence.

#### Scenario: Empty and garbage input

- **WHEN** `from_icc` is called with an empty slice, 128 zero bytes, or arbitrary
  non-ICC text
- **THEN** it returns an `Err` and does not panic

### Requirement: ImageMagick differential oracle within tolerance

The change SHALL include an independent ImageMagick oracle
(`scripts/color_oracle.py`) that runs
`magick … -profile SRC.icc -profile DST.icc` and a differential test comparing
`pictura_color::convert` against it. The differential and known-value tests MUST
accept a per-sample tolerance of 3 (8-bit LSB); differences beyond that MUST fail
the test. The oracle tests MAY skip when ImageMagick or the system ICC profiles
are unavailable.

#### Scenario: sRGB to Adobe RGB matches ImageMagick

- **WHEN** the differential test converts the fixture sRGB → Adobe RGB
  (relative colorimetric, black point compensation on) and ImageMagick is present
- **THEN** no 8-bit sample differs by more than 3 from the ImageMagick output

#### Scenario: Known sRGB primaries

- **WHEN** the sRGB primaries are converted to Adobe RGB with relative
  colorimetric and black point compensation
- **THEN** pure red, green, and blue land within 3 LSB of (219, 2, 0),
  (144, 255, 60), and (0, 2, 250) respectively, and black/white/gray are preserved

#### Scenario: Oracle skips cleanly without ImageMagick

- **WHEN** the `magick` binary is not on `PATH`
- **THEN** the oracle-dependent tests report a skip and do not fail

### Requirement: ICC serialization is byte-deterministic

`Profile::to_icc` SHALL produce byte-identical output each time it serializes a
built-in working-space profile, regardless of when it is called. To make this
hold, `to_icc` SHALL zero the profile's creation-date field — the 12-byte
`dateTimeNumber` at ICC header bytes 24..35 (ICC.1:2022 §7.2.8) — rather than
leaving the wall-clock time Little CMS stamps at profile creation, for which it
offers no setter. The zeroing MUST NOT break `to_icc`'s panic-free contract on a
short or empty buffer. A serialized built-in profile MUST still reload through
`Profile::from_icc` with the same color-space signature and MUST remain a valid
ICC profile that an external ICC reader accepts and can name by its description.

#### Scenario: Repeated serialization is byte-identical [icc_serial_repeat]

- **WHEN** `to_icc` is called twice on a built-in profile
- **THEN** the two byte vectors are equal

#### Scenario: The serialized creation date is zeroed [icc_serial_zero]

- **WHEN** a built-in profile is serialized with `to_icc`
- **THEN** bytes 24..35 of the result are all zero

#### Scenario: A serialized built-in profile reloads [icc_serial_reload]

- **WHEN** the bytes from `to_icc` are passed to `Profile::from_icc`
- **THEN** the reloaded profile succeeds and has the same color-space signature as
  the original

#### Scenario: An external ICC reader accepts the zeroed-date profile [icc_serial_external]

- **WHEN** a document carrying a built-in profile is saved and an external ICC
  reader opens the embedded profile
- **THEN** the reader accepts it and reports the profile's description

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
