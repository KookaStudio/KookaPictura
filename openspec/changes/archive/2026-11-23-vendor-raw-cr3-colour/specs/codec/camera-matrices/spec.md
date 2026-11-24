## MODIFIED Requirements

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

- **WHEN** synthetic CR3s name make "Canon" and the models "Canon EOS R", "Canon EOS RP", "Canon EOS R5", "Canon EOS R5m2", "Canon EOS R6", "Canon EOS R6m2", "Canon EOS R10", "Canon EOS 250D", "Canon EOS R50", "Canon EOS R8" and "Canon EOS R3"
- **THEN** each but the R3 has both matrices with illuminants 17 and 21, the R50 equals the R10, the R8 equals the R6m2, the 250D equals the 200D II, and the R3 has none

#### Scenario: Colour data in the CTMD track [cr3_as_shot]

- **WHEN** synthetic CR3s carry CTMD colour data of versions 19, 32, 48 and 64 with multipliers (2000, 1000, 1024, 1500) at the layout's index, one has no CTMD track, and one has a CTMD record whose size runs past its sample
- **THEN** the first four report `AsShotNeutral` (1012/2000, 1, 1012/1500) within 1e-9 and the other two read with none

#### Scenario: A known camera's CR3 decodes [cr3_known_camera_decodes]

- **WHEN** a synthetic "Canon EOS R5" CR3 with version-34 colour data of multipliers (2208, 1024, 1024, 1967) is decoded with `decode_raw_source`
- **THEN** it decodes, and its as-shot white balance equals `as_shot_transform` of the image's colour tags and lies between 2000 K and 12000 K

#### Scenario: Real CR3 files take colour [cr3_colour_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory of user-supplied `.CR3` files
- **THEN** each file from a table camera decodes with `decode_raw_source` and reports an as-shot temperature between 2000 K and 12000 K, and each other file is refused as unsupported camera colour

## ADDED Requirements

### Requirement: Canon CR3 bodies take camera-JPEG matrices

CR3 bodies without published sensitivities SHALL take the matrices
`scripts/fit-camera-jpeg-matrices.py` fits, by the method and acceptance
limits of "Panasonic bodies take spectral or camera-JPEG matrices", from one
sample CR3 each. The camera rendering SHALL be the largest first sample of a
track that is a JPEG (the full-size JPEG track), make and model SHALL come
from `CMT1`, and the presets from the colour data (tag `0x4001`) of the
`CTMD` maker note: by colour-data version, Daylight at word 136 (versions 16
to 19), 150 (32, 33), 205 (34, 48) or 110 (64 to 66), followed by Shade,
Cloudy and Tungsten, each four RGGB levels and the temperature Canon records.
The daylight matrix SHALL be placed under D55 (code 20) with the neutral
interpolated in mired between the Daylight and Cloudy presets, and the
tungsten matrix under ISO studio tungsten (code 24) when Tungsten is recorded
at 3200 K. When the colour stage's as-shot solve (the pair interpolated in
mired, iterated from 5003 K; the script estimates temperatures with McCamy's
approximation) does not read either matrix's own light back within 2 %, the
script SHALL drop the tungsten matrix, record why, and keep the daylight
matrix alone (the R6 Mark III). Regional names SHALL share their body's row (Kiss M the M50's,
Kiss M2 the M50m2's, Rebel T8i and Kiss X10i the 850D's). A body whose sample
is rejected SHALL take, by owner decision, the matrices of a fitted body
with the same sensor: the M6 Mark II the 90D's and the PowerShot G7 X Mark
III the PowerShot G5 X Mark II's. These matrices reproduce Canon's rendering
and are approximate.

#### Scenario: Canon camera-JPEG bodies get their matrices [cr3_jpeg_matrices]

- **WHEN** synthetic CR3s name make "Canon" and each camera-JPEG body (850D, 90D, M200, M50, M50m2, R100, R50 V, R6 Mark III, R7, 1D X Mark III, PowerShot G5 X Mark II, PowerShot SX70 HS), each regional name and proxy, and the R3 and PowerShot V1
- **THEN** each body has matrices under 20 and 24 (the R6 Mark III one under 20 alone), each regional name and proxy equals its body, the 90D differs from the M50, and the R3 and V1 have none

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
