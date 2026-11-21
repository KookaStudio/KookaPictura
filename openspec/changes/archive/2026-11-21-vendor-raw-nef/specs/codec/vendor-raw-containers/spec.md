## MODIFIED Requirements

### Requirement: Raw containers are recognized and read through one entry point

`sniff_raw` SHALL name a supported raw container from its leading bytes: a
TIFF or BigTIFF whose IFD0 carries `DNGVersion` is DNG, a little-endian
TIFF with `CR` at byte 8 and major version 2 at byte 10 is Canon CR2, and a
classic TIFF without `DNGVersion` whose IFD0 lists `SubIFDs` and whose IFD0
`Make` begins with `NIKON` in any case is Nikon NEF. Any other input SHALL be
named as no raw container. `read_raw` SHALL read a DNG exactly as `read_dng`
does, a CR2 with the CR2 reader and a NEF with the NEF reader, all returning
the same normalized CFA image type, and SHALL refuse any other input with an
unsupported-format error. `sniff_raw_prefix` SHALL agree with `sniff_raw` on a
prefix holding IFD0's entry table and its `Make` value.

#### Scenario: Dispatch by signature [raw_sniff]

- **WHEN** a synthetic DNG, a synthetic CR2, a synthetic NEF, a plain TIFF, a TIFF with make "Nikon" and no SubIFDs, and a PNG signature are passed to `sniff_raw`, `sniff_raw_prefix` and `read_raw`
- **THEN** the DNG, CR2 and NEF are named and read, the DNG result equals `read_dng`'s, and the other three are named as no raw container and refused as unsupported

## ADDED Requirements

### Requirement: A NEF's sensor image is found and decoded

The NEF reader SHALL read the sensor image from the first IFD named by IFD0's
`SubIFDs` whose `NewSubfileType` is 0 and whose photometric interpretation is
CFA (32803), taking its CFA from `CFARepeatPatternDim` and `CFAPattern`; a
file without one SHALL be refused as invalid. The Nikon maker note SHALL be
the Exif maker note that starts with `Nikon\0` and holds a TIFF header at its
byte 10, offsets relative to that header. Compression 1 SHALL hold strips of
16-bit samples in the file's byte order when a strip holds two bytes per
sample, and otherwise samples packed most-significant bit first at
`BitsPerSample`. Compression 34713 SHALL be Nikon Huffman coding when the
maker note carries tag 0x0096, and otherwise one strip laid out as compression
1 except that packed samples run least-significant bit first. A maker note
`NEFCompression` (0x0093) of 8 (small) or 13 and 14 (high efficiency), any
other compression, and bit depths outside 12 to 16 (Huffman: 12 and 14) SHALL
be refused as unsupported.

#### Scenario: The CFA image among the SubIFDs [nef_raw_ifd]

- **WHEN** a big-endian NEF lists a preview SubIFD before the CFA SubIFD, whose `CFAPattern` is blue, green, green, red
- **THEN** the reader decodes the CFA SubIFD at its size and reports the BGGR pattern

#### Scenario: Uncompressed layouts [nef_uncompressed]

- **WHEN** NEFs carry a 12-bit plane packed most-significant bit first in five-row strips, a 14-bit plane in 16-bit containers in each byte order, and, under compression 34713 without a linearization table, a 12-bit plane packed least-significant bit first
- **THEN** each decodes to its samples scaled by its white

#### Scenario: Refused variants [nef_unsupported]

- **WHEN** NEFs carry `NEFCompression` 8, 13 or 14, compression 7, or a 10-bit plane
- **THEN** reading fails with an unsupported-format error

### Requirement: Nikon Huffman sensor data is decoded with its linearization curve

For compression 34713 with maker-note tag 0x0096, the table's bytes SHALL be
read in the maker note's byte order: bytes 0 and 1 the version, bytes 2 to 9
four initial vertical predictors indexed by row parity then column, and bytes
10 and 11 the curve size `n`. A version byte 0 of `0x46` SHALL be lossless:
an identity curve and the lossless code table. Otherwise the data SHALL be
lossy: for version `0x44 0x20` with `n > 1` the curve SHALL be listed at every
`step = 2^bits / (n − 1)` inputs, linearly interpolated between them and held
at the last beyond it, with the split row the 16-bit value at byte 562 when
the table reaches it; for other versions with `2 ≤ n ≤ 16385` the `n` entries
SHALL be the curve; otherwise the curve SHALL be the identity. The code
tables SHALL be Nikon's lossy, lossy-after-split and lossless tables for 12
or 14 bits, each a JPEG-style list of code counts and symbols, switching from
lossy to lossy-after-split at the split row when it is above 0. Each symbol
SHALL give a length `len` (low four bits) and dropped bits `shl` (high four
bits), followed by `len − shl` bits: with `shl = 0` the difference SHALL be
their ITU T.81 `EXTEND`; with `shl > 0` the bits SHALL be shifted up by `shl`
with the dropped bits at their midpoint, minus `2^len` when bit `len − 1` is
clear. Columns 0 and 1 SHALL add the difference to the vertical predictor of
the row's parity and predict from it; later columns SHALL predict from the
sample two columns left. Each output SHALL be the curve at the prediction
clamped to the curve's range. The bits SHALL run continuously across rows
without byte stuffing, and data that runs out SHALL be refused as truncated.

#### Scenario: Lossless data [nef_lossless]

- **WHEN** 12- and 14-bit planes are encoded with the lossless tables, in each byte order
- **THEN** each decodes to its samples exactly

#### Scenario: Lossy data through the curve [nef_lossy_curve]

- **WHEN** 12- and 14-bit planes of curve inputs are encoded with the lossy tables under a stepped `0x44 0x20` curve of 257 entries, and a 12-bit plane under a directly listed `0x44 0x10` curve of 683 entries
- **THEN** each sample decodes to the curve at its input

#### Scenario: The split row [nef_split]

- **WHEN** a 12-bit and a 14-bit plane are encoded with the lossy table above a split row of 6 and the lossy-after-split table from it
- **THEN** each decodes to the values the encoder reconstructed, and rows above the split decode exactly

### Requirement: A NEF is normalized with Nikon camera tags

The whole decoded plane SHALL be the active area and the default crop. The
black level SHALL be the maker note's `BlackLevel` (0x003D), four values for
the positions of the 2 × 2 phase on a 14-bit scale, multiplied by
`2^(bits − 14)`; fewer than four values or no tag SHALL give black 0. The
white level SHALL be `2^bits − 1`. Make, model and orientation SHALL come from
IFD0, and the colour tags SHALL be those of `codec/camera-matrices`.

#### Scenario: Black and camera tags [nef_levels]

- **WHEN** a 14-bit NEF carries black levels 600, 610, 620, 630, make "NIKON CORPORATION", a model not in the table and orientation 8, and a 12-bit NEF carries black level 400 for every position
- **THEN** each sample equals its value normalized by its position's black (the 12-bit file's black is 100) and the white, the tags are reported, the default crop is the whole plane, and the demosaic accepts the image

#### Scenario: Hostile files [nef_hostile]

- **WHEN** a NEF is cut at every length, has a linearization table shorter than its header, has Huffman data shorter than its plane, or a plane beyond the image budget
- **THEN** reading fails with an error and never panics

#### Scenario: Real camera files [nef_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory holding user-supplied `.NEF` files
- **THEN** each one reads or is refused as an unsupported variant, each read plane is not all equal, and the two positions reported green have means within 5 % of each other
