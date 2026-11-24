## MODIFIED Requirements

### Requirement: A CR2 reports its as-shot white balance

The CR2 reader SHALL set `AsShotNeutral` from the Canon maker note's colour
data (tag `0x4001`), read as 16-bit words: SHORT values, or BYTE and
UNDEFINED bytes taken in pairs in the file's byte order (an odd byte
count is unusable). The as-shot RGGB
multipliers start at entry 25 for a 582-entry array, 34 for a 653-entry
array, and otherwise by entry 0 as a signed version: 63 for versions 1 to 15,
71 for 16 to 19 and negative versions, 85 for 32 and 33, and 105 for 34, 48
and 64 to 67. The neutral SHALL be `(G/R, 1, G/B)` with `G` the mean of the
two green multipliers. Colour data of any other type, any other layout, an
array too short, or a multiplier that is not positive SHALL leave
`AsShotNeutral` empty without failing the read.

#### Scenario: Colour-data layouts [cr2_as_shot_layouts]

- **WHEN** synthetic CR2s carry colour data of 582 and 653 entries, and of versions 1, 6, 10, 15, 17, -3, -4, 33 and 48, each with multipliers (2000, 1000, 1024, 1500) at its layout's index and other values elsewhere
- **THEN** each reports `AsShotNeutral` (1012/2000, 1, 1012/1500) within 1e-9

#### Scenario: Colour data stored as bytes [cr2_as_shot_bytes]

- **WHEN** a synthetic CR2 carries version -4 colour data with multipliers (2000, 1000, 1024, 1500) at entry 71, written as an UNDEFINED entry of little-endian byte pairs
- **THEN** it reports `AsShotNeutral` (1012/2000, 1, 1012/1500) within 1e-9

#### Scenario: Unusable colour data [cr2_as_shot_refused]

- **WHEN** synthetic CR2s carry colour data of version 25, a version-10 array of 40 entries, version 10 with a zero red multiplier, version 10 written as a LONG entry, and version -4 written as BYTE values of an odd count
- **THEN** each reads with no `AsShotNeutral`

#### Scenario: A known camera's CR2 decodes [cr2_known_camera_decodes]

- **WHEN** a synthetic "Canon EOS 5DS" CR2 with version-12 colour data of multipliers (2208, 1024, 1024, 1967) is decoded with `decode_raw_source`
- **THEN** it decodes, and its as-shot white balance equals `as_shot_transform` of the image's colour tags and lies between 2000 K and 12000 K

#### Scenario: Real camera files take colour [cr2_colour_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory of user-supplied `.CR2` files
- **THEN** each file from a table camera decodes with `decode_raw_source` and reports an as-shot temperature between 2000 K and 12000 K, and each other file is refused as unsupported camera colour

### Requirement: Canon CR3 bodies take fitted matrices and the CTMD as-shot white balance

The generated table SHALL include, under make `CANON`, every CR3 body
rawtoaces-data publishes sensitivities for at the pinned commit: the EOS R,
RP, R5, R5m2, R6, R6m2, R10 and 200D II, with rawtoaces-data's aliases (R5
Mark II, R50, R8, 250D, Rebel SL3, Kiss X10). CR3 bodies without published
sensitivities SHALL take the matrices of "Canon CR3 bodies take camera-JPEG
matrices", and any other CR3 body SHALL keep no matrices. A CR3's as-shot
neutral SHALL come from the colour data (tag `0x4001`) of the first maker
note found in its `CTMD` track's first sample (records of types 7 to 9, block
`0x927C`), read with the CR2 layouts of "A CR2 reports its as-shot white
balance". Without one it SHALL be empty.

#### Scenario: CR3 models get both matrices [cr3_matrices_lookup]

- **WHEN** synthetic CR3s name make "Canon" and the models "Canon EOS R", "Canon EOS RP", "Canon EOS R5", "Canon EOS R5m2", "Canon EOS R6", "Canon EOS R6m2", "Canon EOS R10", "Canon EOS 250D", "Canon EOS R50", "Canon EOS R8" and "Canon EOS Ra"
- **THEN** each but the EOS Ra has both matrices with illuminants 17 and 21, the R50 equals the R10, the R8 equals the R6m2, the 250D equals the 200D II, and the EOS Ra has none

#### Scenario: Colour data in the CTMD track [cr3_as_shot]

- **WHEN** synthetic CR3s carry CTMD colour data of versions 19, 32, 48 and 64 with multipliers (2000, 1000, 1024, 1500) at the layout's index, one has no CTMD track, and one has a CTMD record whose size runs past its sample
- **THEN** the first four report `AsShotNeutral` (1012/2000, 1, 1012/1500) within 1e-9 and the other two read with none

#### Scenario: A known camera's CR3 decodes [cr3_known_camera_decodes]

- **WHEN** a synthetic "Canon EOS R5" CR3 with version-34 colour data of multipliers (2208, 1024, 1024, 1967) is decoded with `decode_raw_source`
- **THEN** it decodes, and its as-shot white balance equals `as_shot_transform` of the image's colour tags and lies between 2000 K and 12000 K

#### Scenario: Real CR3 files take colour [cr3_colour_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory of user-supplied `.CR3` files
- **THEN** each file from a table camera decodes with `decode_raw_source` and reports an as-shot temperature between 2000 K and 12000 K, and each other file is refused as unsupported camera colour

### Requirement: Canon CR3 bodies take camera-JPEG matrices

CR3 bodies without published sensitivities SHALL take the matrices
`scripts/fit-camera-jpeg-matrices.py` fits, by the method and acceptance
limits of "Panasonic bodies take spectral or camera-JPEG matrices", from one
sample CR3 each, taken from raw.pixls.us or, where those samples are too
grey to fit (the EOS R3, PowerShot V1, EOS M6 Mark II, PowerShot G7 X Mark
III), fail the fit (the EOS R8 Mark II), are missing (the EOS R1) or fit a
pair that does not read back (the R6 Mark III), from owner-supplied files.
The camera rendering SHALL be the largest first sample of a track that is a
JPEG (the full-size JPEG track), make and model SHALL come from `CMT1`, and
the presets from the colour data (tag `0x4001`) of the `CTMD` maker note: by
colour-data version, Daylight at word 136 (versions 16 to 19), 150 (32, 33),
205 (34, 48) or 110 (64 to 67), followed by Shade, Cloudy and Tungsten, each
a record of four RGGB levels ending in the temperature Canon records, five
words long. The daylight matrix SHALL be placed under D55 (code 20) with the
neutral interpolated in mired between the Daylight and Cloudy presets, and
the tungsten matrix under ISO studio tungsten (code 24) when Tungsten is
recorded at 3200 K. When the colour stage's as-shot solve (the pair
interpolated in mired, iterated from 5003 K; the script estimates
temperatures with McCamy's approximation) does not read either matrix's own
light back within 2 %, the script SHALL drop the tungsten matrix, record why,
and keep the daylight matrix alone. Of a body's several accepted samples, the
generator SHALL take the first, in the recorded order, that has both
matrices, and else the first. Regional names SHALL share their body's row
(Kiss M the M50's, Kiss M2 the M50m2's, Rebel T8i and Kiss X10i the 850D's).
No body SHALL take another camera-JPEG body's row as a proxy. These matrices
reproduce Canon's rendering and are approximate.

#### Scenario: Canon camera-JPEG bodies get their matrices [cr3_jpeg_matrices]

- **WHEN** synthetic CR3s name make "Canon" and each camera-JPEG body (850D, 90D, M200, M50, M50m2, M6 Mark II, R1, R3, R100, R50 V, R6 Mark III, R7, R8 Mark II, 1D X Mark III, PowerShot G5 X Mark II, PowerShot G7 X Mark III, PowerShot SX70 HS, PowerShot V1), each regional name, and the EOS Ra
- **THEN** each body has matrices under 20 and 24, each regional name equals its body, the 90D differs from the M50 and the M6 Mark II, the G5 X Mark II differs from the G7 X Mark III, the R1 differs from the R3, and the EOS Ra has none

#### Scenario: Canon camera-JPEG matrices keep white neutral [cr3_jpeg_matrices_white]

- **WHEN** each camera-JPEG body's as-shot transform is built for an `AsShotNeutral` equal to each of its matrices' response to the Planckian white at that matrix's light
- **THEN** each neutral is positive and the reported temperature is that light's within 2 %

#### Scenario: A camera-JPEG CR3 decodes [cr3_jpeg_camera_decodes]

- **WHEN** a synthetic "Canon EOS M50" CR3 with version-16 colour data of multipliers (1621, 1024, 1024, 1357) is decoded with `decode_raw_source`
- **THEN** it decodes, and its as-shot white balance equals `as_shot_transform` of the image's colour tags and lies between 2000 K and 12000 K

#### Scenario: The Canon fits are reproducible [cr3_jpeg_fits_generated]

- **WHEN** `scripts/fit-camera-jpeg-matrices.py` is run on the CR3 samples `camera-jpeg-fits.json` names, then `scripts/gen-camera-matrices.py`
- **THEN** both outputs are rewritten byte for byte, the Panasonic fits kept as recorded

#### Scenario: Canon renders follow the camera's JPEG [cr3_render_matches_camera]

- **WHEN** each camera-JPEG body's sample is opened through File ▸ Open at its as-shot white balance and compared beside its embedded camera JPEG
- **THEN** its colours follow the camera's rendering, apart from tone and lens corrections the open does not apply

## ADDED Requirements

### Requirement: Canon CR2 bodies take camera-JPEG matrices

CR2 bodies without published sensitivities that the owner supplies a sample
of SHALL take the matrices `scripts/fit-camera-jpeg-matrices.py` fits from
one sample CR2 each, by the method, acceptance limits, preset placement and
sample choice of "Canon CR3 bodies take camera-JPEG matrices": the PowerShot
G1 X Mark III. The camera rendering SHALL be IFD0's strip (the full-size
camera JPEG), make and model SHALL come from IFD0, and the presets from the
colour data (tag `0x4001`) of the Exif maker note, whose offsets count from
the file's start, read with the CR3 layouts and, for colour-data version -4,
Daylight at word 103 followed by Shade, Cloudy and Tungsten records eight
words long, each ending in the temperature the camera records. The G1 X Mark
III records its Tungsten preset at 3037 K, so it SHALL take its D55 matrix
alone. These matrices reproduce Canon's rendering and are approximate.

#### Scenario: The G1 X Mark III takes its matrix [cr2_jpeg_matrices]

- **WHEN** a synthetic CR2 names make "Canon" and model "Canon PowerShot G1 X Mark III"
- **THEN** it has one colour matrix, under illuminant 20, and none under the second illuminant

#### Scenario: The G1 X Mark III matrix keeps white neutral [cr2_jpeg_matrices_white]

- **WHEN** its as-shot transform is built for an `AsShotNeutral` equal to its matrix's response to the CIE daylight white at 5503 K
- **THEN** the neutral is positive and the reported temperature is 5503 K within 2 %

#### Scenario: A G1 X Mark III CR2 decodes [cr2_jpeg_camera_decodes]

- **WHEN** a synthetic "Canon PowerShot G1 X Mark III" CR2 with version -4 colour data of multipliers (1361, 1024, 1024, 1333) at entry 71, written as UNDEFINED bytes, is decoded with `decode_raw_source`
- **THEN** it decodes, and its as-shot white balance equals `as_shot_transform` of the image's colour tags and lies between 2000 K and 12000 K

#### Scenario: The CR2 fit is reproducible [cr2_jpeg_fits_generated]

- **WHEN** `scripts/fit-camera-jpeg-matrices.py` is run on the G1 X Mark III samples `camera-jpeg-fits.json` names, then `scripts/gen-camera-matrices.py`
- **THEN** both outputs are rewritten byte for byte

#### Scenario: G1 X Mark III renders follow the camera's JPEG [cr2_render_matches_camera]

- **WHEN** the G1 X Mark III sample is opened through File ▸ Open at its as-shot white balance and compared beside its embedded camera JPEG
- **THEN** its colours follow the camera's rendering, apart from tone, lens corrections and clipped highlights the open does not reproduce
