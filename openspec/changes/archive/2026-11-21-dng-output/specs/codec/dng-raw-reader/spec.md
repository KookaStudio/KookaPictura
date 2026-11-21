## ADDED Requirements

### Requirement: A linear DNG is read as camera RGB

`read_linear_dng` SHALL read a DNG whose full-resolution image is
`LinearRaw` (photometric 34892) with three samples per pixel, uncompressed
(8–16 bit) or lossless JPEG (2–16 bit), strips or tiles. Each sample SHALL
pass through the `LinearizationTable` when present and be normalized by its
own black and white level (`BlackLevel` and `WhiteLevel` with one value or one
per sample) to 0–65535, cropped to the `ActiveArea`. It SHALL report the same
colour, crop, orientation, make and model tags as `read_dng`. A linear image
with other than three samples, black-level deltas, or a `BlackLevelRepeatDim`
other than 1 × 1 SHALL fail with `PsdError::Unsupported`; a CFA image SHALL
fail with `PsdError::Unsupported`. `read_dng` SHALL keep refusing a linear
image.

#### Scenario: Uncompressed linear DNG with levels [dng_linear_read]

- **WHEN** a 12 × 8 uncompressed 16-bit linear DNG with black levels (100, 200, 300), white level 4095 and an active area of rows 1–7, columns 2–12 is read
- **THEN** the camera RGB is 10 × 6 and each sample equals `(v − black) × 65535 / (4095 − black)` rounded and clamped

#### Scenario: Lossless JPEG linear tiles [dng_linear_read_ljpeg]

- **WHEN** a 20 × 10 linear DNG of 16 × 16 three-component lossless-JPEG tiles is read
- **THEN** the camera RGB equals the encoded samples

#### Scenario: Unsupported linear forms [dng_linear_refuses]

- **WHEN** a linear DNG with four samples per pixel, one with `BlackLevelDeltaV`, and a CFA DNG are read with `read_linear_dng`, and the 12 × 8 linear DNG with `read_dng`
- **THEN** each read fails with `Unsupported`
