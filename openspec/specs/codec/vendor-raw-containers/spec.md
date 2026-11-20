# codec/vendor-raw-containers Specification

## Purpose
Camera raw containers beyond DNG, read in-house into the same normalized CFA plane as the DNG reader through one signature-dispatched entry point (#399, part of raw open #86). Canon CR2 first; later containers add their requirements here.

## Requirements

### Requirement: Raw containers are recognized and read through one entry point

`sniff_raw` SHALL name a supported raw container from its leading bytes: a
TIFF or BigTIFF whose IFD0 carries `DNGVersion` is DNG, and a little-endian
TIFF with `CR` at byte 8 and major version 2 at byte 10 is Canon CR2. Any
other input SHALL be named as no raw container. `read_raw` SHALL read a DNG
exactly as `read_dng` does and a CR2 with the CR2 reader, both returning the
same normalized CFA image type, and SHALL refuse any other input with an
unsupported-format error.

#### Scenario: Dispatch by signature [raw_sniff]

- **WHEN** a synthetic DNG, a synthetic CR2, a plain TIFF and a PNG signature are passed to `sniff_raw` and `read_raw`
- **THEN** the DNG and CR2 are named and read, the DNG result equals `read_dng`'s, and the other two are named as no raw container and refused as unsupported

### Requirement: A CR2's sensor plane is decoded from its sliced lossless JPEG

The CR2 reader SHALL read the raw IFD at the offset stored at byte 12, SHALL
require compression 6, and SHALL decode its strip as ITU T.81 lossless JPEG
with one to four interleaved components. The sensor plane's size SHALL be
the raw IFD's `ImageWidth` and `ImageLength`, else the sensor width and height
of the maker note's sensor information, else the frame width times the
component count by the frame height, and its sample count SHALL equal the
frame's, since one frame row may carry several sensor rows. When tag `0xC640` holds `[n, w, last]`,
the decoded samples SHALL fill `n` vertical slices of width `w` and then one
of width `last`, each slice top to bottom, and `n·w + last` SHALL equal the
plane width; without the tag the samples SHALL fill the plane in row order.
A subsampled frame (small or medium raw) and any other compression SHALL be
refused as unsupported.

#### Scenario: Sliced frame [cr2_slices]

- **WHEN** a 14-bit plane is written as a two-component lossless JPEG in two slices of width 24 and a last slice of width 16
- **THEN** the decoded plane equals the written samples scaled by the 14-bit white

#### Scenario: Frame rows carrying two sensor rows [cr2_frame_rows]

- **WHEN** a 64 × 20 plane is written as a four-component frame 32 samples wide and 10 rows high, with `ImageWidth` 64, `ImageLength` 20 and slices `[1, 32, 32]`
- **THEN** the decoded plane is 64 × 20 and equals the written samples scaled by the white

#### Scenario: Unsliced frames and component counts [cr2_components]

- **WHEN** the same plane is written without the slice tag as one, two and four components
- **THEN** each decodes to the same plane

#### Scenario: Refused variants [cr2_unsupported]

- **WHEN** a CR2 holds a subsampled frame, or compression 1
- **THEN** reading fails with an unsupported-format error

### Requirement: A CR2 is normalized to its active area with camera tags

The active area SHALL come from the Canon maker note tag `0x00E0`, whose
signed 16-bit entries 5 to 8 are its inclusive left, top, right and bottom;
without it the whole plane SHALL be active. The black level SHALL be the
mean of the masked photosites left of the active area, or above it when
there is no left mask, taken separately for each position of the 2 × 2 phase
measured from the active area's corner, and 0 without a mask. The white level
SHALL be `2^precision − 1` of the JPEG frame. Samples SHALL be normalized as
DNG samples are, the CFA SHALL be RGGB at the sensor plane's corner and
reported from the active corner, shifted by the active area's offset,
the default crop SHALL be the whole active area, make, model and orientation
SHALL come from IFD0, and the colour tags SHALL be empty.

#### Scenario: Masked border black [cr2_levels]

- **WHEN** a 12-bit CR2 has an active area at left 8 and top 4 and a masked border with a different level at each 2 × 2 position
- **THEN** the plane has the active size and each sample equals its value normalized by its position's black and the 12-bit white

#### Scenario: Camera tags and the demosaic [cr2_tags]

- **WHEN** a CR2 carries make, model and orientation 6
- **THEN** they are reported, the CFA is RGGB for an even active corner and shifted for an odd one, the default crop is the active area, the colour tags are empty, and the demosaic accepts the image

#### Scenario: Hostile files [cr2_hostile]

- **WHEN** a CR2 is cut at every length, has slices that do not add up to the plane width, an active area outside the plane, or a frame beyond the image budget
- **THEN** reading fails with an error and never panics

#### Scenario: Real camera files [cr2_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory holding user-supplied `.CR2` files
- **THEN** each one reads, its samples are not all equal, and the two positions reported green have means within 5 % of each other
