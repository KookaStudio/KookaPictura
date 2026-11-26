## MODIFIED Requirements

### Requirement: A RAF's sensor plane is decoded from its raw directory

The RAF reader SHALL read the big-endian header's section offsets and
lengths (camera JPEG at `0x54`, directory at `0x5C`, sensor section at
`0x64`), SHALL refuse a file whose header names a second directory at `0x78`
(an EXR sensor) as unsupported, and SHALL read the sensor section as a TIFF
whose entry `0xF000` points at the raw directory, refusing a sensor section
that is not a TIFF (a SuperCCD file) as unsupported. The plane SHALL be
`0xF001` by `0xF002` photosites of `0xF003` bits, its data the `0xF008` bytes
at `0xF007` from the section's start. For `n` photosites, `2n` bytes SHALL be
16-bit words in the section's byte order when the row length `0xF005` (in
32-bit words) is absent or 0; with a row length, rows of that length SHALL
hold 12-bit samples packed least
significant bit first, or 14-bit samples packed most significant bit first
over little-endian 32-bit words. Data that is not `2n` bytes and starts with
Fujifilm's compressed-data header SHALL be decoded as "Fujifilm-compressed
RAF data is decoded as LibRaw decodes it" requires, the header's width and
height replacing `0xF001` and `0xF002`. Fewer bytes than the packed size
without that header SHALL be refused as unsupported, with a message naming
the missing compressed-data header. Any other size or bit depth SHALL be
refused as unsupported, and declared sizes SHALL be charged against the
image budget before the plane is allocated.

#### Scenario: Sixteen-bit words [raf_words]

- **WHEN** 14-bit planes are written as little-endian words in a little-endian section and as big-endian words in a big-endian section
- **THEN** each decodes to its samples normalized by the black and the 14-bit ceiling

#### Scenario: Packed samples [raf_packed]

- **WHEN** a 12-bit plane is packed least significant bit first and a 14-bit plane most significant bit first over 32-bit words, each with rows padded beyond the packed length, at an even and an odd width
- **THEN** each decodes to its samples normalized by the black and the ceiling

#### Scenario: Compressed data is refused by name [raf_compressed]

- **WHEN** a RAF's data holds one byte per photosite and does not start with the compressed-data header
- **THEN** it is named RAF and refused as unsupported, naming the compressed-data header

#### Scenario: Refused sensors and layouts [raf_unsupported]

- **WHEN** a RAF names a second frame, has a sensor section that is not a TIFF, has the EXR layout `08 09 0B 0A`, stores 10-bit packed samples, or stores packed rows that do not total its data length
- **THEN** reading fails with an unsupported-format error

### Requirement: A RAF is normalized with its directory's layout and levels

The CFA SHALL come from the directory's layout record `0x0130`: `0C 0C 0C 0C`
SHALL take the 36 values of `0x0131` in reverse order as the 6 × 6 pattern at
the plane's origin, and `0A 0B 09 08` without `0x0131` SHALL be red, green,
green, blue at the plane's origin; any other layout SHALL be refused as
unsupported. The whole plane SHALL be the active area and the directory's
crop (`0x0110` top and left, `0x0111` height and width) its default crop,
refused when it exceeds the plane. The black SHALL be `0xF00A`'s one value, 2
× 2 or 6 × 6 values from the plane's origin, and the white the precision's
ceiling. The as-shot neutral SHALL be `(G/R, 1, G/B)` of `0xF00E`'s green, red
and blue levels, and the baseline exposure minus the directory's raw
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

#### Scenario: Camera tags [raf_tags]

- **WHEN** a RAF's camera JPEG names make `FUJIFILM`, model `X-T3` and orientation 6, and another RAF has no camera JPEG and header name `X100S`
- **THEN** the first reports them, and the second reports make `FUJIFILM`, model `X100S` and orientation 1

#### Scenario: Hostile files [raf_hostile]

- **WHEN** RAFs of each coding are cut at every length, their header offsets and record sizes are corrupted, the crop exceeds the plane, or the width exceeds the image budget
- **THEN** reading returns without panicking, and a cut, an oversized crop and an oversized width fail with an error

#### Scenario: The RAF matches its converted DNG [raf_dng_pair]

- **WHEN** `PICTURA_RAW_CORPUS` holds the raw.pixls.us X100S sample and the DNG converted from it
- **THEN** over the DNG's default crop both have the same CFA colours and the same normalized samples, and the same as-shot neutral within 1e-5

#### Scenario: Real camera files [raf_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory of user-supplied `.RAF` files
- **THEN** each is named RAF, each SuperCCD or EXR file is refused as unsupported, and each other, compressed or not, reads non-flat and demosaics with each photosite's own channel equal to its sample

## ADDED Requirements

### Requirement: Fujifilm-compressed RAF data is decoded as LibRaw decodes it

The RAF reader SHALL decode Fujifilm-compressed sensor data as LibRaw
0.22.1's `fuji_compressed.cpp` does. The data SHALL start with a 16-byte
big-endian header: signature `0x4953`, a lossless flag (1 lossless, 0
lossy), the sensor type (16 X-Trans, 0 Bayer), the precision (12, 14 or 16
bits), the height, the width rounded up to whole strips, the width, the
strip width `0x300`, the strip count and the count of six-row lines. A
header outside LibRaw's limits (height a multiple of 6 up to `0x4002`,
width a multiple of 24 from `0x300` to `0x4200`, the rounded width a whole
number of strips less than one strip beyond the width, lines equal to the
height over 6), or whose sensor type disagrees with the layout record, SHALL
be refused as invalid. The header SHALL be followed by the strips' 32-bit
byte counts padded to 16 bytes, for lossy data by one quantizer base per
strip and line (each strip's run padded to 16), then the strips' bit
streams; a strip that runs past the sensor section SHALL fail as
truncated. Each strip SHALL decode six rows at a time through LibRaw's 18
line buffers, six passes and adaptive gradient codes, lossy lines with the
main quantizer of their base and LibRaw's three fine quantizers, and its rows
SHALL be laid out through the file's CFA. Corrupt codes SHALL fail or decode
without a panic.

#### Scenario: Lossless data [raf_compressed_lossless]

- **WHEN** noise planes two strips wide, the second narrower, are coded losslessly on the compressed X-Trans pattern at 12 and 14 bits and on RGGB at 14 and 16 bits
- **THEN** each decodes to the written plane, normalized by its black and its precision's ceiling

#### Scenario: Lossy data [raf_compressed_lossy]

- **WHEN** such planes are coded lossy with quantizer bases varying by strip and line between 0 and 9, on X-Trans at 14 bits and RGGB at 14 and 16 bits
- **THEN** each decodes to the plane the coder reconstructed, which differs from the written plane by between 1 and 9

#### Scenario: Header checks [raf_compressed_header]

- **WHEN** a compressed RAF's lossless flag, sensor type, precision, height, width, strip width, strip count or line count is outside LibRaw's limits, or its X-Trans header sits in a Bayer-layout file
- **THEN** reading fails as invalid data naming compressed data

#### Scenario: Hostile compressed files [raf_compressed_hostile]

- **WHEN** lossless and lossy compressed RAFs are cut short, have bytes of their compressed data flipped, or record a strip size past the section
- **THEN** reading returns without panicking, and the cut files and the oversized strip fail with an error

#### Scenario: Real files match LibRaw [raf_compressed_libraw]

- **WHEN** `PICTURA_RAW_CORPUS` holds compressed RAFs and python3 with rawpy is available
- **THEN** each decodes to LibRaw's raw plane of the same size, sample for sample after the same normalization
