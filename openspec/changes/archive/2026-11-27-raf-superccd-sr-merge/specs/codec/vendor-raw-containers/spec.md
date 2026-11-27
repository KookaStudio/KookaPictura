## MODIFIED Requirements

### Requirement: A RAF's sensor plane is decoded from its raw directory

The RAF reader SHALL read the big-endian header's section offsets and
lengths (camera JPEG at `0x54`, directory at `0x5C`, sensor section at
`0x64`) and SHALL read the primary frame whether or not the header names a
second directory at `0x78`. The header SHALL name a second directory only
when `0x78` is not 0 and the camera JPEG's offset is above `0x78`, as LibRaw
checks; a shorter header's bytes at `0x78` SHALL be ignored. A sensor section that is a TIFF SHALL be read
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

#### Scenario: A short header names no second directory [raf_short_header]

- **WHEN** a SuperCCD word-section RAF with the layout's top bit set has non-zero bytes at `0x78` but its camera JPEG starts at `0x6C`
- **THEN** its rows hold the width once, as without a second directory

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

- **WHEN** `PICTURA_RAW_CORPUS` holds SuperCCD RAFs and python3 with rawpy is available, the SR files read with header offset `0x80` cleared so only their primary frame is read, as LibRaw reads them
- **THEN** each grid has LibRaw's size and equals LibRaw's raw plane, repacked with LibRaw's margins and normalized by LibRaw's black and white, within 1

## ADDED Requirements

### Requirement: A SuperCCD SR file merges its R frame into extended highlight range

For a SuperCCD layout in a word section whose header names a second
directory and whose header offset at `0x80` is not 0, the RAF reader SHALL
read a second frame of the same size and row stride from that offset,
bounded by the file; rows or
samples the file does not hold SHALL be missing. The second frame's black
SHALL come from the second directory's `0x4000`. Both frames SHALL be
repacked onto LibRaw's grid with the same active area. With `s` and `r`
each frame's black-subtracted sample over the primary frame's range, for
each grid CFA colour the ratio `k` SHALL be the median of `s / r` over
samples with `s` from 0.25 to 0.75 and `r` at least 16 counts, falling back,
with fewer than 256 such samples, to record `0x9200`'s first value in the
primary directory over that in the second, or 16 when either is absent or
0. The merged value SHALL be `s` below 0.80, `k·r` from 0.95, and linear
between the two from 0.80 to 0.95, and no less than `s` where `s` reaches
the range, a clipped sample being a lower bound; a missing second sample
SHALL keep `s`.
With `h` the larger of 1 and the largest merged value, each sample SHALL be
`65535 · m / h` rounded and `BaselineExposure` SHALL grow by `log2 h`. Any
other layout SHALL keep the primary frame alone.

#### Scenario: Unclipped samples keep the primary frame [raf_sr_unclipped]

- **WHEN** an SR RAF of each layout bit has S samples below 0.80 of its range and R samples a sixteenth of them
- **THEN** it decodes to the same samples and `BaselineExposure` as without the R frame

#### Scenario: Clipped samples come from the R frame [raf_sr_clipped]

- **WHEN** an SR RAF's R samples are a per-colour fraction (1/12, 1/16, 1/20) of a scene that clips its S samples, the R frame's black differs from the S frame's, and some clipped photosites have R at black
- **THEN** the fitted ratio recovers each fraction, clipped samples decode to the scene over the headroom within 1 count, those with R at black to the S range over it, and `BaselineExposure` grows by `log2` of the largest merged value over the S range

#### Scenario: The blend is continuous [raf_sr_blend]

- **WHEN** S samples ramp from 0.75 to 0.99 of the range, unclipped, with R samples 4 % above S over the fitted ratio
- **THEN** the merged samples equal `s + t·(k·r − s)`, `t` rising linearly from 0 at 0.80 to 1 at 0.95, over the headroom within 1 count, and rise monotonically with `s`

#### Scenario: Too few samples fall back to the nominal ratio [raf_sr_nominal]

- **WHEN** an SR RAF's S samples are all clipped, with `0x9200` 256 in the primary directory and 32 in the second, then without `0x9200`
- **THEN** they merge with ratios of 8 and 16

#### Scenario: Missing R rows keep the S frame [raf_sr_missing_rows]

- **WHEN** an S3 Pro-style RAF, R in a section of its own, is cut short of its R frame's last row, and that row is clipped
- **THEN** it decodes, and that row's samples equal the primary frame's over the headroom within 1 count

#### Scenario: Upright layouts keep the primary frame [raf_sr_upright]

- **WHEN** a Bayer word section has a second directory and a second frame at `0x80`, and an SR RAF has a second directory but `0x80` is 0
- **THEN** each decodes to its primary samples alone

#### Scenario: Real SR files gain highlight range [raf_sr_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` holds the S3 Pro, S5 Pro or F700 samples
- **THEN** each reads as with `0x80` cleared except that `BaselineExposure` grows by `log2 h` > 0, samples where that primary read is below 0.80 of its range equal it over `h` within 1 count, and samples where it clips decode to at least the range over `h`
