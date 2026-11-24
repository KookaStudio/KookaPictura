## MODIFIED Requirements

### Requirement: A CR2 is normalized to its active area with camera tags

The active area SHALL come from the Canon maker note tag `0x00E0`, whose
signed 16-bit entries 5 to 8 are its inclusive left, top, right and bottom;
without it the whole plane SHALL be active. The black level SHALL be the
mean of the masked photosites left of the active area, or above it when
there is no left mask, taken separately for each position of the 2 × 2 phase
measured from the active area's corner, and 0 without a mask. The white level
SHALL be `2^precision − 1` of the JPEG frame; the white levels the maker
note's colour data records (`NormalWhiteLevel`, `SpecularWhiteLevel`) SHALL
NOT set it, because Canon files store clipped photosites at the ceiling. Samples SHALL be normalized as
DNG samples are, the CFA SHALL be RGGB at the sensor plane's corner and
reported from the active corner, shifted by the active area's offset,
the default crop SHALL be the whole active area, make, model and orientation
SHALL come from IFD0, and the colour tags SHALL be those of
`codec/camera-matrices`: the camera's fitted matrices when its make and model
are in the table, and the as-shot neutral from the maker note's colour data.

#### Scenario: Masked border black [cr2_levels]

- **WHEN** a 12-bit CR2 has an active area at left 8 and top 4 and a masked border with a different level at each 2 × 2 position
- **THEN** the plane has the active size and each sample equals its value normalized by its position's black and the 12-bit white

#### Scenario: Camera tags and the demosaic [cr2_tags]

- **WHEN** a CR2 carries make, model and orientation 6, with a model not in the matrix table and no colour data
- **THEN** they are reported, the CFA is RGGB for an even active corner and shifted for an odd one, the default crop is the active area, the colour tags are empty, and the demosaic accepts the image

#### Scenario: Hostile files [cr2_hostile]

- **WHEN** a CR2 is cut at every length, has slices that do not add up to the plane width, an active area outside the plane, or a frame beyond the image budget
- **THEN** reading fails with an error and never panics

#### Scenario: Real camera files [cr2_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory holding user-supplied `.CR2` files
- **THEN** each one reads, its samples are not all equal, and the two positions reported green have means within 5 % of each other

#### Scenario: Colour-data white levels do not set the white [cr2_white_ceiling]

- **WHEN** a 14-bit CR2 carries version-15 colour data recording a normal white level of 4000 and a specular white level of 4500 below most of its samples
- **THEN** its samples equal those of the same CR2 without colour data, normalized to the 14-bit white

### Requirement: A CR3 is normalized to its active area with Canon tags

The active area SHALL come from the `CMT3` maker note's sensor information
(tag `0x00E0`, entries 5 to 8 the inclusive left, top, right and bottom),
in the frame's coordinates; without it the whole frame SHALL be active. The
black level SHALL be the mean of the masked photosites left of the active
area, else above it, per position of the 2 × 2 phase measured from the
active corner, and 0 without a mask. The white SHALL be `2^precision − 1`;
the colour data's recorded white levels SHALL NOT set it, as for CR2.
The CFA SHALL be the header's layout at the frame's corner, reported from the
active corner. The default crop SHALL be the whole active area. Make, model
and orientation SHALL come from `CMT1`, else be absent (orientation 1). The
colour tags SHALL be those of `codec/camera-matrices`. A malformed file SHALL
fail without a panic and without an allocation beyond the image budget.

#### Scenario: Masked border black [cr3_levels]

- **WHEN** a 14-bit CR3 has an active area at left 8 and top 4 and a masked border with a different level at each 2 × 2 position
- **THEN** the plane has the active size and each sample equals its value normalized by its position's black and the 14-bit white

#### Scenario: Odd borders shift the colour pattern [cr3_borders]

- **WHEN** each colour layout is read under active areas with even and odd left and top
- **THEN** the reported pattern is that of the active area's first four photosites

#### Scenario: Camera tags and the demosaic [cr3_tags]

- **WHEN** a CR3 names make, model and orientation 6 in `CMT1` and has no `CMT3`, and another has no `CMT1`
- **THEN** the first reports them with the whole frame active and demosaics, and the second reports no make or model and orientation 1

#### Scenario: Hostile files [cr3_hostile]

- **WHEN** CR3s of each coding are cut at every length and have each byte of their headers and frame corrupted, a box declares a size past its parent, and a frame declares a size beyond the image budget
- **THEN** each is an error or a decoded image and none panics; every cut, the overrun and the oversize frame are errors

#### Scenario: Real camera files [cr3_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory holding user-supplied `.CR3` files
- **THEN** each one reads, its samples are not all equal, and the two positions reported green have means within 5 % of each other

#### Scenario: Colour-data white levels do not set the white [cr3_white_ceiling]

- **WHEN** a 14-bit CR3 carries version-48 colour data in its CTMD recording a normal white level of 4000 and a specular white level of 4500 below most of its samples
- **THEN** its samples equal those of the same CR3 without CTMD, normalized to the 14-bit white
