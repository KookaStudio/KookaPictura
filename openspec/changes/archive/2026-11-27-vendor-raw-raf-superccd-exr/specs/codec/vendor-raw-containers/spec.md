## MODIFIED Requirements

### Requirement: A RAF's sensor plane is decoded from its raw directory

The RAF reader SHALL read the big-endian header's section offsets and
lengths (camera JPEG at `0x54`, directory at `0x5C`, sensor section at
`0x64`) and SHALL read the primary frame whether or not the header names a
second directory at `0x78`. A sensor section that is a TIFF SHALL be read
through its entry `0xF000`, which points at the raw directory. The plane SHALL be
`0xF001` by `0xF002` photosites of `0xF003` bits, its data the `0xF008` bytes
at `0xF007` from the section's start. For `n` photosites, `2n` bytes SHALL be
16-bit words in the section's byte order when the row length `0xF005` (in
32-bit words) is absent or 0; with a row length, rows of that length SHALL
hold 12-bit samples packed least significant bit first, or, when `0xF006`
is 1, most significant bit first over little-endian 32-bit words, or 14-bit
samples packed most significant bit first over little-endian 32-bit words. Data that is not `2n` bytes and starts with
Fujifilm's compressed-data header SHALL be decoded as "Fujifilm-compressed
RAF data is decoded as LibRaw decodes it" requires, the header's width and
height replacing `0xF001` and `0xF002`. Fewer bytes than the packed size
without that header SHALL be refused as unsupported, with a message naming
the missing compressed-data header. Any other size or bit depth SHALL be
refused as unsupported. A sensor section that is not a TIFF SHALL be a word
section: directory record `0x0100`'s height by width 16-bit words from the
section's start, in the byte order of the camera JPEG's Exif (little-endian
without one); big-endian words SHALL be 12-bit samples and little-endian
ones samples under LibRaw's white `0x3E00`. When the layout record's first
byte has its top bit set and the header names a second directory, each row
SHALL hold twice the width, the primary frame in its first half, and the
data SHALL be bounded by the file rather than by the section's recorded
length. A word section shorter than its plane SHALL fail as truncated.
Declared sizes SHALL be charged against the image budget before the plane
is allocated.

#### Scenario: Sixteen-bit words [raf_words]

- **WHEN** 14-bit planes are written as little-endian words in a little-endian section and as big-endian words in a big-endian section
- **THEN** each decodes to its samples normalized by the black and the 14-bit ceiling

#### Scenario: Packed samples [raf_packed]

- **WHEN** a 12-bit plane is packed least significant bit first, a 12-bit plane with `0xF006` = 1 and a 14-bit plane most significant bit first over 32-bit words, each with rows padded beyond the packed length, at an even and an odd width
- **THEN** each decodes to its samples normalized by the black and the ceiling

#### Scenario: Compressed data is refused by name [raf_compressed]

- **WHEN** a RAF's data holds one byte per photosite and does not start with the compressed-data header
- **THEN** it is named RAF and refused as unsupported, naming the compressed-data header

#### Scenario: Refused sensors and layouts [raf_unsupported]

- **WHEN** a RAF has a 2 × 2 layout record that lacks blue, an X-Trans pattern record beside a 2 × 2 layout, 10-bit packed samples, or packed rows that do not total its data length
- **THEN** reading fails with an unsupported-format error

#### Scenario: Word sections [raf_word_section]

- **WHEN** Bayer planes are written as word sections: little-endian words under a little-endian camera JPEG, big-endian 12-bit words under a big-endian one, and rows of twice the width beside a second directory with the layout's top bit set, whose second halves hold other samples
- **THEN** each decodes to its primary samples normalized by its black and a white of 15872 for little-endian words and 4095 for big-endian ones, and the same word section cut short of its plane fails

#### Scenario: A second directory reads the primary frame [raf_second_frame]

- **WHEN** a TIFF-section RAF names a second directory at `0x78`
- **THEN** it decodes to the same image as without it

### Requirement: A RAF is normalized with its directory's layout and levels

The CFA SHALL come from the directory's layout record `0x0130`: `0C 0C 0C 0C`
SHALL take the 36 values of `0x0131` in reverse order as the 6 × 6 pattern at
the plane's origin. A layout whose second byte has bit 3 set and that has no
`0x0131` SHALL be a 2 × 2 pattern at the plane's origin, its four bytes read
last first and each byte's low two bits naming red (0), green (1 or 3) or
blue (2); a pattern lacking a colour SHALL be refused as unsupported. A
layout whose second byte has bit 3 clear SHALL be a SuperCCD lattice, read
as "A SuperCCD plane is repacked onto LibRaw's 45° grid" requires; any other
layout SHALL be refused as unsupported. For other layouts the whole plane SHALL be the active area and the directory's
crop (`0x0110` top and left, `0x0111` height and width) its default crop,
refused when it exceeds the plane. In a TIFF section the black SHALL be
`0xF00A`'s one value, 2 × 2 or 6 × 6 values from the plane's origin, the
white the precision's ceiling, and the as-shot neutral `(G/R, 1, G/B)` of
`0xF00E`'s green, red and blue levels. In a word section the black SHALL be
record `0x4000`'s big-endian green, red, green, blue values (eight values
being two such sets, averaged), applied by each photosite's colour, a green
on a row without red taking the second green, 0 without the record, and the as-shot
neutral `(G/R, 1, G/B)` of record `0x2FF0`'s green, red, green and blue
levels. The baseline exposure SHALL be minus the directory's raw
exposure bias `0x9650` (a signed 16-bit ratio), 0 when it is absent or its
denominator is 0. Make, model and orientation SHALL come from IFD0 of the
camera JPEG's `Exif` segment; without it the make SHALL be `FUJIFILM`, the
model the header's camera name at `0x1C`, and the orientation 1. Colour
matrices SHALL come from the per-camera table.

#### Scenario: X-Trans pattern and levels [raf_xtrans]

- **WHEN** a RAF names the X-Trans layout with a pattern record, a 6 × 6 black whose values differ by position, a crop and green, red and blue levels 302, 577 and 454
- **THEN** the CFA is the record reversed, every photosite is normalized by its own position's black, the default crop is the directory's, and the neutral is (302/577, 1, 302/454)

#### Scenario: The raw exposure bias lifts the develop [raf_exposure_bias]

- **WHEN** RAFs record a raw exposure bias of -72/100, -172/100, 5/0 and none
- **THEN** their baseline exposures are 0.72, 1.72, 0 and 0

#### Scenario: Bayer layout [raf_bayer]

- **WHEN** a RAF names the Bayer layout with a 2 × 2 black
- **THEN** the CFA is red, green, green, blue and the demosaic accepts the image

#### Scenario: Upright 2 × 2 layouts read last first [raf_exr_layout]

- **WHEN** RAFs name the layouts `0A 0B 09 08`, `08 09 0B 0A` and `09 0A 08 0B`
- **THEN** their CFAs are red, green, green, blue; blue, green, green, red; and green, red, blue, green, and each demosaics

#### Scenario: Word-section levels come from the directory [raf_word_levels]

- **WHEN** Bayer word sections record `0x4000` as green 513, red 515, green 511, blue 517, and as two sets whose values differ by 2, and `0x2FF0` as green 80, red 167, green 80, blue 119
- **THEN** each photosite is normalized by its colour's black, greens on blue's rows by the second green and the sets averaged, and the neutral is (80/167, 1, 80/119)

#### Scenario: Camera tags [raf_tags]

- **WHEN** a RAF's camera JPEG names make `FUJIFILM`, model `X-T3` and orientation 6, and another RAF has no camera JPEG and header name `X100S`
- **THEN** the first reports them, and the second reports make `FUJIFILM`, model `X100S` and orientation 1

#### Scenario: Hostile files [raf_hostile]

- **WHEN** RAFs of each coding, word sections and SuperCCD files included, are cut at every length, their header offsets and record sizes are corrupted, the crop exceeds the plane, or the width exceeds the image budget
- **THEN** reading returns without panicking, and a cut, an oversized crop and an oversized width fail with an error

#### Scenario: The RAF matches its converted DNG [raf_dng_pair]

- **WHEN** `PICTURA_RAW_CORPUS` holds the raw.pixls.us X100S sample and the DNG converted from it
- **THEN** over the DNG's default crop both have the same CFA colours and the same normalized samples, and the same as-shot neutral within 1e-5

#### Scenario: Real camera files [raf_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory of user-supplied `.RAF` files
- **THEN** each is named RAF and reads non-flat; each upright one demosaics with each photosite's own channel equal to its sample, and each SuperCCD one demosaics to its rotated size

#### Scenario: Upright files match LibRaw [raf_upright_libraw]

- **WHEN** `PICTURA_RAW_CORPUS` holds uncompressed RAFs with a 2 × 2 layout and python3 with rawpy is available
- **THEN** each has LibRaw's raw plane size, CFA colours equal to LibRaw's raw pattern at the plane's origin, and samples equal to LibRaw's raw plane normalized by the RAF's own levels

## ADDED Requirements

### Requirement: A SuperCCD plane is repacked onto LibRaw's 45° grid

For a SuperCCD layout the RAF reader SHALL follow LibRaw 0.22.1. With
`L` the top bit of the layout record's first byte, the active area SHALL
be the directory's inset (`0x0110` top and left, `0x0111` height `h` and
width `w`) when it fits the plane; otherwise, for a plane 2944 wide in a
file without a `0xC000` record, 2144 × 2880 centred in the plane, with
orientation 6; otherwise record `0x0121`'s height shifted left by `L` and
width shifted right by `L`, centred in the plane. With `fw = w >> (1 − L)`,
the image SHALL be a grid `W = (h >> L) + fw` wide and `W − 1` high. For
`row` below the plane's height less twice the top margin, and `col` below
`fw << (1 − L)`, photosite `(row, col)` of the active area SHALL go to grid
row `fw − 1 − col + ⌊row / 2⌋` and column `col + ⌊(row + 1) / 2⌋` when `L`
is set, and to row `fw − 1 + row − ⌊col / 2⌋` and column
`row + ⌊(col + 1) / 2⌋` otherwise, when that cell is in the grid and the
column in the plane; every other cell SHALL be 0. The grid's CFA SHALL be
red, green, green, blue when `fw` is odd and green, blue, red, green when it
is even. The image SHALL carry `fw` as its `fuji_width`, and its default crop
SHALL be the whole upright image the demosaic produces.

#### Scenario: Photosites land on LibRaw's grid [raf_superccd_repack]

- **WHEN** SuperCCD RAFs of each layout bit, with odd and even `fw`, hold distinct sample values
- **THEN** each sample lies at LibRaw's grid cell, normalized; every other cell is 0; the CFA follows `fw`'s parity; and the default crop is the rotated size

#### Scenario: The active area follows LibRaw [raf_superccd_area]

- **WHEN** SuperCCD RAFs have an inset that fits the plane, an inset wider than the plane with a `0x0121` size, and a plane 2944 wide without `0xC000`
- **THEN** the first repacks the inset, the second the centred `0x0121` area, and the third a centred 2880 × 2144 area with orientation 6

#### Scenario: Real SuperCCD files match LibRaw [raf_superccd_libraw]

- **WHEN** `PICTURA_RAW_CORPUS` holds SuperCCD RAFs and python3 with rawpy is available
- **THEN** each grid has LibRaw's size and equals LibRaw's raw plane, repacked with LibRaw's margins and normalized by LibRaw's black and white, within 1
