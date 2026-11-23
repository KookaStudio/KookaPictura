## MODIFIED Requirements

### Requirement: An RW2's sensor plane is decoded from its plain packings

The RW2 reader SHALL read IFD0 at the offset stored at byte 4. The sensor
plane SHALL be `SensorWidth` (`0x0002`) by `SensorHeight` (`0x0003`), stored at
`RawDataOffset` (`0x0118`) or, without it, `StripOffsets`. The plain codings
SHALL occupy exactly `StripByteCounts` bytes. It SHALL decode three plain
codings:

- compression 34828 or 34830 without `RawFormat` (`0x002D`), at 12 or 14
  bits: little-endian 16-bit words;
- compression 34316 with RawFormat 2 at 12 bits: 16-byte groups of ten samples;
- compression 34316 with RawFormat 7 at 14 bits: 16-byte groups of nine samples.

A group's samples SHALL be taken from its least significant bit up, the group
read as a little-endian number, and its remaining bits ignored. A RawFormat
that neither this nor the compressed-codings requirement decodes, or a bit
depth they do not take, SHALL be refused as unsupported, naming the RawFormat.
Any other compression or bit depth without a RawFormat, a width that is no
whole number of the coding's packets, and a plain length other than the
coding's SHALL be refused as unsupported.

#### Scenario: Sixteen-bit words [rw2_words]

- **WHEN** 12-bit samples shifted left by four and 14-bit samples shifted left by two are stored as words under compression 34828 and 34830
- **THEN** the plane equals the stored words normalized by the levels times 16

#### Scenario: Sixteen-byte groups [rw2_packed]

- **WHEN** a plane is packed as RawFormat 2 and as RawFormat 7 with every padding bit set, and placed by `StripOffsets` and by `RawDataOffset`
- **THEN** each plane equals the normalized samples

#### Scenario: Other codings are refused by name [rw2_refused]

- **WHEN** files declare RawFormat 1 and 9 at 12 bits, RawFormat 5 at 10 bits, RawFormat 6 at 16 bits and RawFormat 8 at 17 bits, with and without a compression tag
- **THEN** each is named RW2 and refused as unsupported, the message naming the RawFormat

#### Scenario: Unobserved layouts are refused [rw2_unsupported]

- **WHEN** files declare RawFormat 2 at 14 bits, RawFormat 7 at 12 bits, words at 10 bits, compression 1, a packed width that is no whole number of groups, a CFA code of 5, or a length two bytes over
- **THEN** each is refused as unsupported

### Requirement: An RW2 is normalized to its sensor borders with Panasonic tags

The active area SHALL run from `SensorTopBorder` and `SensorLeftBorder` up to,
not including, `SensorBottomBorder` and `SensorRightBorder`; borders outside
the plane or enclosing nothing SHALL be refused as invalid. `CFAPattern`
(`0x0009`: 1 RGGB, 2 GRBG, 3 GBRG, 4 BGGR) counts from the sensor's corner and
SHALL be reported from the active area's corner. The black SHALL be
`0x001C`-`0x001E` for red, green and blue, 0 when absent, plus 15 for
RawFormat 3 and 4; the white SHALL be the green linearity limit `0x000F` when
it lies above the blacks and within the coding's range, else that range's top
(4095 for words and RawFormat 3 and 4, the bit depth's top otherwise). Word
files SHALL scale both by 16. The as-shot neutral SHALL be the green level
over each of the red, green and blue white-balance levels `0x0024`-`0x0026`,
and absent when one is zero or missing. The crop `0x002F`-`0x0032` (top,
left, bottom, right in sensor coordinates) SHALL become the default crop when
it lies in the active area, else the whole active area is the crop. Make,
model and orientation SHALL come from IFD0. Colour matrices SHALL come only
from the fitted camera table. A malformed file SHALL fail without a panic and
without an allocation beyond the image budget.

#### Scenario: Odd borders shift the colour pattern [rw2_borders]

- **WHEN** each CFA code is read under borders with even and odd top and left, with a different black per colour
- **THEN** the plane is the area inside the borders, each colour normalized by its own black, and the pattern is that of the area's first four photosites

#### Scenario: The crop counts from the sensor [rw2_crop]

- **WHEN** a file with borders 3, 5 records a crop inside them, one reaching outside them, and none
- **THEN** the first gives that crop less the borders and the other two the whole active area

#### Scenario: Camera tags and white balance [rw2_tags]

- **WHEN** a file records make, model, orientation 6 and levels 512, 256, 384, and another a zero red level
- **THEN** they are reported, the neutral is 0.5, 1, 2/3 and absent for the zero, and the image demosaics

#### Scenario: A body without colour is refused at open [rw2_no_colour]

- **WHEN** a decodable DMC-FZ50 file goes through `decode_raw_source`
- **THEN** it is refused as unsupported, saying the camera colour for Panasonic DMC-FZ50 is not supported yet

#### Scenario: Hostile files fail cleanly [rw2_hostile]

- **WHEN** files of each plain coding are cut at every length, have header and directory bytes corrupted, declare reversed or outside borders, or declare a width over the budget
- **THEN** every cut, bad border and oversize file is an error and none panics

#### Scenario: Real camera files [rw2_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` holds RW2, RWL or Panasonic RAW files
- **THEN** each one of a known coding reads, is not flat and has green phases that agree, and each other one is refused naming its RawFormat

## ADDED Requirements

### Requirement: An RW2's compressed codings are decoded as rawspeed decodes them

The RW2 reader SHALL decode Panasonic's compressed codings as darktable's
rawspeed does, whatever the compression tag:

- RawFormat 3 and 4 (any bit depth): packets of 14 predicted photosites in 16
  bytes, read as one bit stream per 0x4000-byte block; with `RawDataOffset`
  each block's sections are swapped at 0x1FF8 and every block is whole,
  without it the blocks are unswapped and the last may be short. Each zero
  photosite SHALL then become the distance-weighted mean of the nearest
  non-zero photosite of its colour to its left, right, top and bottom, as
  rawspeed's bad-pixel fill weights them, and stay zero when there is none;
- RawFormat 5 at 12 or 14 bits: the plain 16-byte groups, inside whole
  0x4000-byte blocks swapped at 0x1FF8;
- RawFormat 6 at 12 or 14 bits: 16-byte packets of 14 or 11 photosites with
  a base per three and pair-wise prediction, less 15 and clipped as rawspeed
  clips them;
- RawFormat 8 at 1 to 16 bits: the strips listed by tags `0x0042`-`0x0048`,
  each a bit stream read from each byte's low bit, of Huffman-coded
  differences (table `0x0040`) predicted per colour of each 2 × 2 block, from
  the starting values `0x003C`-`0x003F` as rawspeed reads them.

The input SHALL be the coding's length from the data offset (RawFormat 8: each
strip at its own offset); a shorter file SHALL be refused as truncated. The
width SHALL be a whole number of the coding's packets (even for RawFormat 8).
RawFormat 8 strips that do not tile the plane in rows, a code table that is
malformed, a strip whose codes run past its length, an undefined code, and a
gamma curve, shift or clip other than the identity SHALL be refused.

#### Scenario: Predicted packets [rw2_predictive]

- **WHEN** a 252 × 64 plane of walks that need each shift is coded as RawFormat 4 with `RawDataOffset` and as RawFormat 3 without it, black 100, 110, 120 and odd borders
- **THEN** each plane equals the samples normalized with the blacks plus 15

#### Scenario: Zero photosites are filled [rw2_zeros]

- **WHEN** a RawFormat 4 plane of 1000 holds a packet starting at zero, and a packet whose even photosites are zero until a 5 at its thirteenth
- **THEN** the first zero reads as 1000, the 5 reads as black and each other zero is filled

#### Scenario: Groups in swapped blocks [rw2_blocked]

- **WHEN** 12- and 14-bit planes spanning two blocks are coded as RawFormat 5
- **THEN** each plane equals the normalized samples

#### Scenario: Paired packets [rw2_paired]

- **WHEN** 12- and 14-bit planes of walks that need each base are coded as RawFormat 6 under CFA code 2
- **THEN** each plane equals the normalized samples

#### Scenario: Huffman-coded strips [rw2_strips]

- **WHEN** a 16-bit plane is coded as RawFormat 8 in one strip, in 2 × 2 strips and in three strips across, with distinct starting values and black 2048
- **THEN** each plane equals the normalized samples

#### Scenario: Strips must tile the plane [rw2_strip_grid]

- **WHEN** a RawFormat 8 file's sensor width is two wider than its strips, or odd
- **THEN** each is an error

#### Scenario: Hostile compressed files fail cleanly [rw2_codings_hostile]

- **WHEN** small files of each compressed coding are cut at every length and have every byte corrupted
- **THEN** every cut is an error and none panics
