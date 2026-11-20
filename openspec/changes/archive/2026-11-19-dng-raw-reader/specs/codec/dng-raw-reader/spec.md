## ADDED Requirements

### Requirement: A DNG's CFA sensor plane is read from its container

`read_dng` SHALL parse a classic TIFF or BigTIFF container in either byte
order, SHALL require the `DNGVersion` tag, and SHALL read the full-resolution
image (`NewSubFileType` 0, photometric interpretation 32803) from IFD0 or one
of its SubIFDs. Strip and tile layouts SHALL both be read. Compression 1 SHALL
be read at 8 to 16 bits per sample, bit-packed most significant bit first
below 16 bits with each row starting on a byte boundary, and 16-bit samples
in the file's byte order. Compression 7 SHALL be read as ITU T.81 lossless
JPEG with predictors 1 to 7, any component count, restart intervals and point
transform, the decoded samples filling the strip or tile in row order.

#### Scenario: An uncompressed 16-bit strip [dng_uncompressed]

- **WHEN** a little-endian DNG holds a 16-bit uncompressed CFA strip with black 0 and white 65535
- **THEN** the decoded plane equals the written samples and the CFA pattern is reported

#### Scenario: Bit-packed samples [dng_packed]

- **WHEN** the same plane is written uncompressed at 10, 12 and 14 bits per sample with odd row widths
- **THEN** every depth decodes to the written values scaled by its white level

#### Scenario: Lossless JPEG tiles [dng_ljpeg]

- **WHEN** the plane is written as lossless JPEG tiles, once per predictor 1 to 7, with one component and with two components of half the tile width, and once with a restart interval
- **THEN** each decodes to the same plane as the uncompressed file

#### Scenario: Byte order and BigTIFF [dng_container]

- **WHEN** the plane is written little-endian, big-endian, and as little-endian BigTIFF, with the raw image in a SubIFD behind a thumbnail IFD0
- **THEN** all three decode to the same plane

### Requirement: Sensor samples are normalized to the active area

Each sample SHALL pass through the `LinearizationTable` when present, then be
mapped to `round((value − black) · 65535 / (white − black))` clamped to
0–65535, where black is the `BlackLevel` entry for the photosite's position
in the `BlackLevelRepeatDim` pattern plus `BlackLevelDeltaV` for its row and
`BlackLevelDeltaH` for its column. The plane SHALL be cropped to `ActiveArea`,
and positions, the black pattern and the CFA pattern SHALL be measured from
the active area's top-left corner. `DefaultCropOrigin` and `DefaultCropSize`
SHALL be reported relative to the active area.

#### Scenario: Black pattern, deltas and white [dng_levels]

- **WHEN** a DNG has a 2×2 `BlackLevelRepeatDim` with four rational black levels, per-row and per-column deltas, and white 4095
- **THEN** each decoded sample equals the formula above computed independently in the test

#### Scenario: Linearization table [dng_linearization]

- **WHEN** an 8-bit DNG carries a 256-entry `LinearizationTable` into a 14-bit white level
- **THEN** each sample is the table entry normalized by black and white

#### Scenario: Active area and default crop [dng_active_area]

- **WHEN** a DNG's `ActiveArea` starts at an odd row and column inside masked border pixels
- **THEN** the plane has the active area's size, holds the active samples only, and reports the CFA pattern and default crop relative to it

### Requirement: Colour and orientation tags are reported

The result SHALL carry `ColorMatrix1/2`, `ForwardMatrix1/2`,
`CameraCalibration1/2`, `CalibrationIlluminant1/2`, `AsShotNeutral`,
`AnalogBalance`, `BaselineExposure`, `Orientation`, `Make` and
`UniqueCameraModel` as stored, absent tags reported as absent (or the DNG
default where the specification gives one).

#### Scenario: Colour tags round-trip [dng_color_tags]

- **WHEN** a DNG carries both colour matrices, a forward matrix, as-shot neutral, illuminants 17 and 21, orientation 6 and a camera model
- **THEN** each value is reported as written, and the missing calibration matrices are absent

### Requirement: Unsupported or hostile DNGs fail with a typed error

A DNG whose image uses an unsupported compression (8, 32773, 34892, or any
other), photometric interpretation 34892, floating-point samples, a CFA with
other than the three colours red, green and blue, or a non-rectangular
`CFALayout` SHALL fail with `PsdError::Unsupported`. A TIFF without
`DNGVersion` SHALL fail with `PsdError::Invalid`. Declared dimensions SHALL be
checked against `ImageBudget` before any plane is allocated, and offsets,
counts and lossless JPEG frames SHALL be bounded by the file and the tile, so
a truncated or hostile file fails with an error rather than panicking or
allocating its declared size.

#### Scenario: Deferred features are refused [dng_unsupported]

- **WHEN** a DNG uses compression 8, 32773 or 34892, a linear-raw image, floating-point samples, or a CMYG pattern, or a TIFF has no `DNGVersion`
- **THEN** each read fails with the matching typed error

#### Scenario: Hostile headers [dng_hostile]

- **WHEN** a DNG declares 200000 × 200000 photosites, a strip offset past the end of the file, a truncated lossless JPEG tile, or a JPEG frame larger than its tile, and when every prefix of a valid DNG is read
- **THEN** each read returns an error and none panics

### Requirement: Camera-authored DNGs decode

A camera-authored DNG SHALL decode when the user supplies it; the check is
opt-in because such files are not committable.

#### Scenario: Leica M9 samples [dng_m9_corpus]

- **WHEN** `PICTURA_DNG_CORPUS` names a directory holding the Leica M9 `L1.dng` and `L2.dng`
- **THEN** each decodes to 5216 × 3472 with pattern RGGB, a 2,2 default crop of 5212 × 3468, illuminants 17 and 21, and spot samples equal to values from an independent Python parse of the file
