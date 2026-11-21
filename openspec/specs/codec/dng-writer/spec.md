# codec/dng-writer Specification

## Purpose
Writing a mosaic or linear DNG with its colour tags, thumbnail, optional JPEG preview, `crs:` settings and embedded original raw file, at an ACR compatibility level, readable by our reader and by LibRaw and ExifTool (#402).

## Requirements

### Requirement: A mosaic DNG round-trips through the reader

`write_dng` SHALL write a mosaic image as a little-endian DNG whose
full-resolution SubIFD holds the CFA plane as 16-bit samples with black level
0 and white level 65535, compressed as 256 × 256 lossless-JPEG tiles
(compression 7), with `CFARepeatPatternDim`, `CFAPattern`, `CFAPlaneColor`
and `CFALayout` 1, the default crop, and in IFD0 the orientation, `Make`,
`UniqueCameraModel` and every colour tag the image carries (`ColorMatrix1/2`,
`ForwardMatrix1/2`, `CameraCalibration1/2`, `CalibrationIlluminant1/2`,
`AnalogBalance`, `AsShotNeutral`, `BaselineExposure`). Reading the file with
`read_dng` SHALL return the written samples, pattern, crop, orientation, make,
model and colour exactly.

#### Scenario: Mosaic round trip [dng_write_mosaic_round_trip]

- **WHEN** a 40 × 30 noise plane with a GRBG pattern, a default crop of (2, 3, 36, 24), orientation 6, make "Kooka", model "Kooka Test", two colour matrices, two forward matrices, a camera calibration, analog balance, as-shot neutral and baseline exposure 0.5 is written as a mosaic DNG and read with `read_dng`
- **THEN** every field of the read image equals the written one

#### Scenario: Several tiles and ragged edges [dng_write_mosaic_tiles]

- **WHEN** a 600 × 300 noise plane is written as a mosaic DNG
- **THEN** the full-resolution IFD has compression 7, tile width and length 256 and six tiles, and `read_dng` returns the samples exactly

### Requirement: A linear DNG round-trips through the reader

`write_dng` SHALL write a linear image as `PhotometricInterpretation` 34892
with three 16-bit samples per pixel in three-component lossless-JPEG tiles,
with the same colour, crop, orientation, make and model tags as a mosaic
image. Reading the file with `read_linear_dng` SHALL return the written camera
RGB and tags exactly.

#### Scenario: Linear round trip [dng_write_linear_round_trip]

- **WHEN** a 37 × 21 noise camera RGB with a colour matrix, as-shot neutral, crop and orientation 3 is written as a linear DNG and read with `read_linear_dng`
- **THEN** the camera RGB, colour, crop, orientation, make and model equal the written ones

### Requirement: IFD0 holds a thumbnail and a JPEG preview may follow

IFD0 SHALL be a thumbnail (`NewSubFileType` 1) of uncompressed 8-bit RGB whose
long side is at most 256, taken from the caller's sRGB preview pixels. When a
preview is requested, a second SubIFD SHALL hold it as one baseline JPEG strip
(`NewSubFileType` 1, compression 7, photometric 6) at the caller's size, with
`PreviewColorSpace` 2 (sRGB) at compatibility 1.2 and later; without one, no
preview IFD SHALL be written.

#### Scenario: Thumbnail [dng_write_thumbnail]

- **WHEN** a DNG is written with 400 × 300 preview pixels and no JPEG preview
- **THEN** IFD0 has `NewSubFileType` 1, photometric 2, three 8-bit samples, a 256 × 192 strip, and IFD0 lists exactly one SubIFD

#### Scenario: JPEG preview [dng_write_jpeg_preview]

- **WHEN** a DNG is written with a 120 × 80 JPEG preview
- **THEN** the second SubIFD has `NewSubFileType` 1, compression 7, photometric 6 and `PreviewColorSpace` 2, and its strip starts with `FF D8` and decodes to 120 × 80 pixels

### Requirement: Settings and the original raw file can be embedded

A given XMP packet SHALL be written as IFD0's tag 700 and read back by
`raw_xmp` byte for byte. When an original raw file is given, IFD0 SHALL carry
`OriginalRawFileName` (its file name) and `OriginalRawFileData`: a big-endian
fork length, `n + 1` big-endian offsets relative to the fork start, then `n`
zlib streams each inflating to 64 KiB of the file (the last to the rest),
then zero words for the empty resource fork, file type and creator.

#### Scenario: Embedded settings [dng_write_xmp]

- **WHEN** a DNG is written with a `crs:` packet setting exposure +0.50
- **THEN** `raw_xmp` returns the packet unchanged

#### Scenario: Embedded original raw [dng_write_original_raw]

- **WHEN** a DNG is written with a 200 000-byte original named `shot.cr2`
- **THEN** `OriginalRawFileName` is `shot.cr2`, the data holds four blocks, and inflating them returns the original bytes

### Requirement: The compatibility level sets the DNG version

The compatibility levels Camera Raw 2.4, 4.1, 4.6, 5.4 and 6.6 and later SHALL
write `DNGVersion` and `DNGBackwardVersion` 1.1, 1.2, 1.2, 1.3 and 1.4
respectively (an approximate mapping). Below 1.2, `ForwardMatrix1/2` and
`PreviewColorSpace` SHALL be omitted.

#### Scenario: Oldest and newest levels [dng_write_compatibility]

- **WHEN** an image with a forward matrix and a JPEG preview is written at Camera Raw 2.4 and at 6.6
- **THEN** the first has versions 1.1.0.0, no `ForwardMatrix1` and no `PreviewColorSpace`, and the second has versions 1.4.0.0, `ForwardMatrix1` and `PreviewColorSpace` 2

### Requirement: Inconsistent input and cancellation stop the write

`write_dng` SHALL fail with `PsdError::Invalid` when the image is empty, its
sample count does not match its size and kind, a preview's pixel count does
not match its size, or the file would pass 4 GiB. It SHALL return `None`
without a file when its cancellation check turns true.

#### Scenario: Refusals [dng_write_refuses]

- **WHEN** a 0 × 0 image, a mosaic with one sample missing, and a preview with one byte missing are written
- **THEN** each write fails with `Invalid`

#### Scenario: Cancellation [dng_write_cancel]

- **WHEN** a write starts with its cancellation check already true
- **THEN** it returns `None`

### Requirement: Reference tools read the DNGs we write

A mosaic and a linear DNG written by `write_dng` SHALL open in LibRaw (through
`rawpy`) with the written size, and ExifTool SHALL read their `DNGVersion` and
extract the embedded original raw file unchanged. The check SHALL skip when
the tool is absent.

#### Scenario: LibRaw and ExifTool [dng_write_reference_tools]

- **WHEN** a 64 × 48 mosaic DNG with an embedded original and a 64 × 48 linear DNG are written and given to `rawpy` and `exiftool`
- **THEN** LibRaw reports 64 × 48 for both, ExifTool reports `DNGVersion` 1.4.0.0, and `exiftool -b -OriginalRawImage` returns the original bytes
