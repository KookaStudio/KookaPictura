## MODIFIED Requirements

### Requirement: Raw containers are recognized and read through one entry point

`sniff_raw` SHALL name a supported raw container from its leading bytes: a
TIFF or BigTIFF whose IFD0 carries `DNGVersion` is DNG, a little-endian
TIFF with `CR` at byte 8 and major version 2 at byte 10 is Canon CR2, and a
TIFF without `DNGVersion` whose IFD0 `Make` begins with `SONY` is Sony ARW.
Any other input SHALL be named as no raw container. `sniff_raw_prefix` SHALL
answer the same from a file prefix holding IFD0's entry table and its `Make`
value. `read_raw` SHALL read a DNG exactly as `read_dng` does, a CR2 with the
CR2 reader and an ARW with the ARW reader, all returning the same normalized
CFA image type, and SHALL refuse any other input with an unsupported-format
error.

#### Scenario: Dispatch by signature [raw_sniff]

- **WHEN** a synthetic DNG, a synthetic CR2, a plain TIFF and a PNG signature are passed to `sniff_raw` and `read_raw`
- **THEN** the DNG and CR2 are named and read, the DNG result equals `read_dng`'s, and the other two are named as no raw container and refused as unsupported

#### Scenario: An ARW is named by its make [arw_sniff]

- **WHEN** a synthetic ARW with make `SONY`, the same file with make `FUJIFILM`, and a DNG with make `SONY` are sniffed
- **THEN** the first is named ARW by `sniff_raw` and `sniff_raw_prefix` and reads through `read_raw`, the second is no raw container, and the DNG is named DNG

## ADDED Requirements

### Requirement: An ARW's sensor plane is decoded from its raw SubIFD

The ARW reader SHALL take the full-resolution image found among IFD0, its
SubIFDs and the IFD chain, and SHALL require photometric interpretation
32803 with one sample per pixel and a 2 × 2 CFA (`CFAPattern`, RGGB without
it). Compression 1 SHALL be read as one sample per little-endian 16-bit word
over the strips. Compression 32767 with strips of at least one byte per
photosite SHALL be read as ARW2: each row's 32-photosite groups are two
16-byte blocks, the even columns then the odd; a block read as a 128-bit
little-endian number holds an 11-bit maximum, an 11-bit minimum, the 4-bit
positions of the maximum and the minimum, then a 7-bit delta for each other
position, each value being the minimum plus its delta shifted by the
smallest shift in 0 to 4 for which 128 shifted by it exceeds the block's
range, capped at 2047; the codes SHALL be linearized by the tone curve of tag
`0x7010`, which over half-code steps rises with slopes 1, 2, 4, 8 and 16
between knees at 0, each tag value divided by 4, and 4095, a code reading the
curve at twice its value. Compression 7 SHALL be read as lossless JPEG tiles
whose frames are half the tile's width and height in four components, each
frame sample being one 2 × 2 quad of photosites with its components
row-major, any other frame shape being refused as unsupported, and
the plane SHALL be cut to the sensor size of tag `0x7038` when present. A
file whose raw directory cannot be found or has no photometric
interpretation (ARW 1.0), a non-CFA image (the YCbCr small and medium raws),
compression 32767 with a shorter strip, uncompressed strips holding fewer
than two bytes per photosite, and any other compression SHALL be refused as
unsupported. An ARW2 width that is not a multiple of 32 or a missing tone
curve SHALL be refused as invalid.

#### Scenario: Uncompressed words [arw_uncompressed]

- **WHEN** a 64 × 20 plane of 14-bit samples is written as 16-bit words with black 500, 510, 520, 530 by 2 × 2 position and white 16000
- **THEN** the decoded plane equals the samples normalized by each position's black and the white

#### Scenario: ARW2 blocks through the tone curve [arw_arw2]

- **WHEN** a 64 × 12 plane of 11-bit codes is ARW2-encoded with knees 8000, 10400, 12900, 14100, black 512 and white 16383
- **THEN** the decoded plane equals the codes a decoder reconstructs from the blocks, read through the tone curve and normalized

#### Scenario: Lossless tiles [arw_lossless]

- **WHEN** an 80 × 40 plane is written as 32 × 16 lossless JPEG tiles padded to 96 × 48, with tag `0x7038` naming 80 × 40
- **THEN** the decoded plane is 80 × 40 and equals the normalized samples

#### Scenario: Refused variants [arw_unsupported]

- **WHEN** an ARW holds a YCbCr image, an ARW2 strip shorter than one byte per photosite, an uncompressed strip of 1.5 bytes per photosite, or compression 8
- **THEN** reading fails with an unsupported-format error

### Requirement: An ARW is normalized with the levels the file records

Black, white and the as-shot multipliers SHALL be read from the raw IFD and,
for each one it lacks, from the SR2 block: black from tag `0x7310`, else
`0x7300`, one value per 2 × 2 position, else 0; white from tag `0xC61D`,
else the smallest value of tag `0x787F`, else the ceiling of the sample
precision (the tone curve's top for ARW2); the multipliers from tag `0x7313`
(red, green, green, blue), else `0x7303` (green, red, blue, green). The SR2
block SHALL be found through IFD0's `DNGPrivateData` (4 bytes in file order)
naming the SR2Private directory, whose tags `0x7200`, `0x7201` and `0x7221`
give the SR2SubIFD's offset, length and key; it SHALL be deciphered as
big-endian 32-bit words XORed with the key stream seeded from the key by
`k × 48828125 + 1`, and parsed as a directory whose offsets count from the
file start. A block ending beyond 4 MiB SHALL be refused as invalid. The
as-shot neutral SHALL be `(G/R, 1, G/B)` with `G` the green mean, and none
when a multiplier is not positive. The active area SHALL be the whole
(cut) plane, the default crop tags `0xC61F` and `0xC620` relative to it or
the whole area without them, make, model and orientation SHALL come from
IFD0, and the colour matrices those of `codec/camera-matrices`.

#### Scenario: Levels from the SR2 block [arw_levels_sr2]

- **WHEN** the same plane carries its levels in the raw IFD, in an enciphered SR2 block, and in an SR2 block with the older tags `0x7300` and `0x7303`; then in the raw IFD and, differently, the SR2 block; then nowhere
- **THEN** the first three decode alike with as-shot neutral (1012/2000, 1, 1012/1500), the raw IFD's levels win over the block's, and without levels the black is 0, the white 16383 and there is no neutral

#### Scenario: Camera tags and the demosaic [arw_tags]

- **WHEN** an ARW names model `ILCE-7M3`, orientation 6, CFA GBRG and default crop 4, 2, 56 × 16; and another names model `NEX-5` with no crop tags
- **THEN** make, model, orientation, CFA and crop are reported, the first has both colour matrices and a neutral and the demosaic accepts it, and the second has no matrices and the whole area as its crop

#### Scenario: Hostile files [arw_hostile]

- **WHEN** ARWs of each coding are cut at every length, an ARW2 declares width 48, a tone curve has two values, tag `0x7038` exceeds the plane, the width exceeds the image budget, or the SR2 offset points 64 MiB in
- **THEN** reading fails with an error and never panics

#### Scenario: Real camera files [arw_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory holding user-supplied `.ARW` files
- **THEN** each is named ARW and either is refused as unsupported or reads with samples not all equal and the two positions reported green within 5 % of each other
