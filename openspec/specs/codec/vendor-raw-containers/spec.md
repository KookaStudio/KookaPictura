# codec/vendor-raw-containers Specification

## Purpose
Camera raw containers beyond DNG, read in-house into the same normalized CFA plane as the DNG reader through one signature-dispatched entry point (#399, part of raw open #86). Canon CR2 first; later containers add their requirements here.

## Requirements

### Requirement: Raw containers are recognized and read through one entry point

`sniff_raw` SHALL name a supported raw container from its leading bytes: a
TIFF or BigTIFF whose IFD0 carries `DNGVersion` is DNG, a little-endian
TIFF with `CR` at byte 8 and major version 2 at byte 10 is Canon CR2, an ISO
base media file with `ftyp` at byte 4 and major brand `crx ` at byte 8 is
Canon CR3, and a
classic TIFF without `DNGVersion` whose IFD0 lists `SubIFDs` and whose IFD0
`Make` begins with `NIKON` in any case is Nikon NEF, and any other TIFF without
`DNGVersion` whose IFD0 `Make` begins with `SONY` is Sony ARW. A file whose
first four bytes are `IIRO`, `IIRS` or `MMOR`, with at least eight bytes, is
Olympus ORF whatever its make, and one whose first four bytes are
`IIU\0`, with at least eight bytes, is Panasonic RW2 whatever its make, and
one whose first sixteen bytes are `FUJIFILMCCD-RAW ` is Fujifilm RAF. Any
other input SHALL be named as no raw
container. `read_raw` SHALL read a DNG exactly as
`read_dng` does, a CR2 with the CR2 reader, a CR3 with the CR3 reader, a NEF
with the NEF reader, an
ARW with the ARW reader, an ORF with the ORF reader, an RW2 with the RW2
reader and a RAF with the RAF reader, all returning the same normalized CFA image type, and
SHALL refuse any other input with an unsupported-format error.
`sniff_raw_prefix` SHALL agree with `sniff_raw` on a prefix holding IFD0's
entry table and its `Make` value, a CR3's first twelve bytes, or a RAF's first
sixteen bytes.

#### Scenario: Dispatch by signature [raw_sniff]

- **WHEN** a synthetic DNG, a synthetic CR2, a synthetic NEF, a plain TIFF, a TIFF with make "Nikon" and no SubIFDs, and a PNG signature are passed to `sniff_raw`, `sniff_raw_prefix` and `read_raw`
- **THEN** the DNG, CR2 and NEF are named and read, the DNG result equals `read_dng`'s, and the other three are named as no raw container and refused as unsupported

#### Scenario: An ARW is named by its make [arw_sniff]

- **WHEN** a synthetic ARW with make `SONY`, the same file with make `FUJIFILM`, and a DNG with make `SONY` are sniffed
- **THEN** the first is named ARW by `sniff_raw` and `sniff_raw_prefix` and reads through `read_raw`, the second is no raw container, and the DNG is named DNG

#### Scenario: An ORF is named by its header [orf_sniff]

- **WHEN** synthetic ORFs with each of the three headers, their first eight bytes alone, and a plain TIFF with make `OLYMPUS CORPORATION` are sniffed
- **THEN** each ORF and each prefix is named ORF, each ORF reads through `read_raw`, and the TIFF is no raw container

#### Scenario: An RW2 is named by its header [rw2_sniff]

- **WHEN** a synthetic RW2 with make `Panasonic`, the same with make `LEICA`, their first eight bytes alone, and a plain TIFF with make `Panasonic` are sniffed
- **THEN** each RW2 and each prefix is named RW2, each RW2 reads through `read_raw`, and the TIFF is no raw container

#### Scenario: A CR3 is named by its brand [cr3_sniff]

- **WHEN** a synthetic CR3, its first twelve bytes alone, and an ISO media file of brand `isom` are sniffed
- **THEN** the CR3 and its prefix are named CR3 by `sniff_raw` and `sniff_raw_prefix`, the CR3 reads through `read_raw`, and the `isom` file is no raw container

#### Scenario: A RAF is named by its header [raf_sniff]

- **WHEN** a synthetic RAF, its first sixteen bytes alone, and a file starting `FUJIFILMCCD-RAX ` are sniffed
- **THEN** the RAF and its prefix are named RAF by `sniff_raw` and `sniff_raw_prefix`, the RAF reads through `read_raw`, and the other file is no raw container

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

### Requirement: A NEF's sensor image is found and decoded

The NEF reader SHALL read the sensor image from the first IFD named by IFD0's
`SubIFDs` whose `NewSubfileType` is 0 and whose photometric interpretation is
CFA (32803), taking its CFA from `CFARepeatPatternDim` and `CFAPattern`; a
file without one SHALL be refused as invalid. The Nikon maker note SHALL be
the Exif maker note that starts with `Nikon\0` and holds a TIFF header at its
byte 10, offsets relative to that header. Compression 1 SHALL hold strips of
16-bit samples in the file's byte order when a strip holds two bytes per
sample, and otherwise samples packed most-significant bit first at
`BitsPerSample` through 32-bit words in the file's byte order; in a
little-endian file they SHALL be one strip whose rows end on whole words, and
any other little-endian packing SHALL be refused as unsupported. Compression 34713 SHALL be Nikon Huffman coding when the
maker note carries tag 0x0096, and otherwise one strip whose exact size names
its layout: 16-bit containers as for compression 1, rows packed
least-significant bit first, or, at 12 bits, rows of 16-byte groups each
holding ten samples packed most-significant bit first and a pad byte; a strip
matching none SHALL be refused as unsupported. A maker note
`NEFCompression` (0x0093) of 8 (small) or 13 and 14 (high efficiency), any
other compression, and bit depths outside 12 to 16 (Huffman: 12 and 14) SHALL
be refused as unsupported.

#### Scenario: The CFA image among the SubIFDs [nef_raw_ifd]

- **WHEN** a big-endian NEF lists a preview SubIFD before the CFA SubIFD, whose `CFAPattern` is blue, green, green, red
- **THEN** the reader decodes the CFA SubIFD at its size and reports the BGGR pattern

#### Scenario: Uncompressed layouts [nef_uncompressed]

- **WHEN** NEFs carry a 12-bit plane packed most-significant bit first in five-row strips of a big-endian file and in one strip of little-endian 32-bit words, a 14-bit plane in 16-bit containers in each byte order, and, under compression 34713 without a linearization table, a 12-bit plane packed least-significant bit first and a 12-bit plane in padded 16-byte groups
- **THEN** each decodes to its samples scaled by its white

#### Scenario: Refused variants [nef_unsupported]

- **WHEN** NEFs carry `NEFCompression` 8, 13 or 14, compression 7, a 10-bit plane, or a compression 34713 strip without a linearization table whose size matches no layout
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

### Requirement: An ORF's sensor plane is decoded from IFD0's strips

The ORF reader SHALL take the sensor image from IFD0 and its coding from the
strips' total size for `n` photosites: `2n` bytes SHALL be 16-bit words in
file order; `1.6n` bytes SHALL be 16-byte groups of ten 12-bit samples packed
least significant bit first followed by one pad byte; `1.5n` bytes SHALL be
12-bit samples packed most significant bit first, over little-endian 32-bit
words under an `IIRO` header and over bytes under an `IIRS` header, whose
strips SHALL be taken as two fields, the even rows and then the odd, each
strip holding whole rows whatever `RowsPerStrip` says. One strip of fewer
than `1.5n` bytes is the Olympus compressed stream and SHALL be refused as
unsupported, with a message saying that compressed sensor data is not
supported yet. Any other size, and packed data of other than 12 valid bits,
SHALL be refused as unsupported, and declared sizes SHALL be charged against
the image budget before the plane is allocated.

#### Scenario: Sixteen-bit words [orf_words]

- **WHEN** 12-bit and 10-bit planes are written left-aligned in 16-bit words, little-endian and big-endian, over several strips
- **THEN** each decodes to its samples normalized by the scaled black and ceiling

#### Scenario: Packed samples [orf_packed]

- **WHEN** a 12-bit plane is written in each of the three packings
- **THEN** each decodes to the samples normalized by the black and white

#### Scenario: Interlaced compact fields [orf_fields]

- **WHEN** `IIRS` planes of odd and even height are written as two fields, each ending in its own short strip
- **THEN** each decodes to its samples in sensor row order

#### Scenario: Compressed data is refused by name [orf_compressed]

- **WHEN** 12-bit and 14-bit ORFs whose one strip holds one byte per photosite are opened
- **THEN** each is named ORF and refused as unsupported, naming compressed sensor data

#### Scenario: Refused variants [orf_unsupported]

- **WHEN** an ORF's strips total 1.75 bytes per photosite, several strips total fewer than 1.5, a packed ORF names 14 valid bits, or the CFA is not Bayer
- **THEN** reading fails with an unsupported-format error

### Requirement: An ORF takes levels, crop and white balance from its maker note

The reader SHALL read the maker note of each of the headers `OLYMP` (offsets
from the file start), `OLYMPUS` and `OM SYSTEM` (offsets from the note), and
from it, preferring ImageProcessing, then RawInfo, then the main note: the
black per 2 x 2 position, the valid bits, the as-shot red and blue
multipliers over 256, and the crop, which SHALL become the default crop of
the whole plane. The white SHALL be the first SensorCalibration value when it
lies above the black and within the precision, else the precision's ceiling;
16-bit words SHALL scale both levels by their left alignment. The CFA SHALL
be Exif `CFAPattern`, 2 x 2 red, green, green, blue in some phase, any other
being refused as unsupported. Make, model and orientation SHALL come from
IFD0.

#### Scenario: Every maker-note layout [orf_maker_notes]

- **WHEN** the same levels, crop and multipliers are written in an `OLYMPUS` note, an `OM SYSTEM` note, an `OLYMP` note with ImageProcessing as a value, one with RawInfo, and one with only the main note's tags
- **THEN** all decode to the same plane and neutral, and all but the last report the crop

#### Scenario: Camera tags and the demosaic [orf_tags]

- **WHEN** an ORF names make `OM Digital Solutions`, model `OM-5`, orientation 6 and a GRBG CFA
- **THEN** they are reported and the demosaic accepts the image

#### Scenario: Hostile files [orf_hostile]

- **WHEN** ORFs of each coding are cut at every length, their header and directories are corrupted, the crop exceeds the plane, or the width exceeds the image budget
- **THEN** reading returns without panicking, and a cut, an oversized crop and an oversized width fail with an error

#### Scenario: Real camera files [orf_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory of user-supplied `.ORF` and `.ORI` files
- **THEN** each is named ORF, each compressed file is refused as such, and each other reads non-flat, with the two green positions within 5 % of each other, and decodes with colour when it has a white balance

### Requirement: An ORF's embedded matrix supplies its colour

The maker note's 3 x 3 matrix over 256, its rows normalized to sum 1, SHALL
be taken as white-balanced camera RGB to linear sRGB and reported as a
forward matrix and a colour matrix anchored on the 6600 K white balance as
D65, else the fine-weather balance as D55, else the as-shot balance as D65.
A file without a usable matrix or without any of those balances SHALL report
no matrices, so that opening it is refused as unsupported camera colour.

#### Scenario: The matrix renders as the camera meant [orf_colour]

- **WHEN** an ORF with a matrix and different as-shot and 6600 K balances is converted at its as-shot balance
- **THEN** a camera neutral is neutral in ProPhoto and a white-balanced colour equals the matrix's sRGB colour taken to ProPhoto, within 0.002

#### Scenario: The anchor sets the temperature [orf_anchor]

- **WHEN** the as-shot balance equals the 6600 K balance, equals only a fine-weather balance, or is the only balance, and when there is no balance at all
- **THEN** the as-shot temperature is within 100 K of 6504 K, 5503 K and 6504 K, and the last has no matrices and is refused as unsupported camera colour

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
(4095 for words and RawFormat 3 and 4, the bit depth's top otherwise), and
for RawFormat 2 at most 3967, where its data saturates (observed on the FZ8
and FZ18). Word files SHALL scale both by 16. The as-shot neutral SHALL be the green level
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

- **WHEN** a decodable DMC-FZ1 file, a body the matrix table does not hold, goes through `decode_raw_source`
- **THEN** it is refused as unsupported, saying the camera colour for Panasonic DMC-FZ1 is not supported yet

#### Scenario: RawFormat 2 saturates at 3967 [rw2_saturation]

- **WHEN** a RawFormat 2 file and a 12-bit word file, each with white tag 4095, hold 3967 at every photosite
- **THEN** the RawFormat 2 plane is 65535 throughout and the word plane is 3967 / 4095 of 65535

#### Scenario: Hostile files fail cleanly [rw2_hostile]

- **WHEN** files of each plain coding are cut at every length, have header and directory bytes corrupted, declare reversed or outside borders, or declare a width over the budget
- **THEN** every cut, bad border and oversize file is an error and none panics

#### Scenario: Real camera files [rw2_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` holds RW2, RWL or Panasonic RAW files
- **THEN** each one of a known coding reads, is not flat and has green phases that agree, and each other one is refused naming its RawFormat

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

### Requirement: A CR3's main image is the first frame of its largest raw track

The CR3 reader SHALL walk the file's boxes as ISO/IEC 14496-12 sizes them
(32-bit sizes, 64-bit sizes after a size of 1, a size of 0 running to the
end), and SHALL refuse a box that overruns its parent. Each `moov/trak` whose
first `stsd` entry is `CRAW` and holds a `CMP1` box is a raw track. Its
first sample SHALL lie at the first `co64` chunk offset, sized by `stsz`'s
fixed sample size or else its first entry. Among the raw tracks whose `CMP1`
header is valid, the reader SHALL take the one of most bits per frame (bit
depth times width times height), the first of equals, and SHALL decode its
first sample. The `CMP1` header SHALL be refused when it is shorter than 40
bytes, its version is neither 0x100 nor 0x200, its header size is zero, its
tile is larger than the frame, it declares more than three wavelet levels,
its frame or tile size is odd, its colour layout is above 3, or its bit depth
is 8. A frame that is not of four planes SHALL be refused as unsupported,
naming its plane count, and an encoding other than 0 and 3, or a bit depth
above 14, as unsupported, naming the coding and depth. A file without a
movie box or without a raw track SHALL be refused as invalid; when every raw
track was refused, the first track's refusal SHALL be reported.

#### Scenario: The largest track's first frame [cr3_tracks]

- **WHEN** a synthetic CR3 holds a small preview raw track, then two full-size raw tracks of different planes, each with two samples, and a `CTMD` track
- **THEN** it reads the first full-size track's first sample

#### Scenario: Refused headers [cr3_refused]

- **WHEN** a CR3's `CMP1` declares one plane, encoding 1, encoding 2, 15 bits, version 0x300, four levels, or 39 bytes, and when a CR3 has no movie box or no raw track
- **THEN** the plane count and the encodings are refused as unsupported naming them, and the others are refused as invalid

### Requirement: CRX frames are decoded as LibRaw decodes them

The CRX decoder SHALL decode a frame as LibRaw 0.22.1's `crx.cpp` does. The
four colour planes, each half the frame's width and height, SHALL be split
into tiles of half the header's tile size. Each tile and plane SHALL be
coded as one low band plus three bands per wavelet level, each band a bit
stream of adaptive Golomb-Rice line codes with runs. A plane's low band SHALL
be predicted from the line above when the plane header's partial flag is
set, and coded without that reference otherwise. Version 1 band headers
SHALL scale each band by their quantizer, and version 2 tiles by their
coded per-block quantizer tables. One to three levels SHALL be inverted
with the reversible 5/3 lifting, sharing coefficients across tile edges as
LibRaw does. Encoding 0 samples SHALL be the decoded values plus half the
range, clamped to the bit depth. Encoding 3 SHALL combine the four planes
into red, two greens and blue by LibRaw's integer formula, at the header's
median precision. The planes SHALL be laid out in the header's colour
layout. Malformed headers, codes that run past a line or a band's bytes, and
quantizer tables that index outside themselves SHALL fail without a panic.

#### Scenario: Lossless lines [crx_lossless]

- **WHEN** 14-bit planes with flat runs and noise are coded losslessly with prediction from the line above, in one tile and in 2 × 2 tiles, under each colour layout
- **THEN** each decodes to the written plane

#### Scenario: Lines without reference [crx_no_ref]

- **WHEN** the same planes are coded with the partial flag clear
- **THEN** each decodes to the written plane

#### Scenario: Wavelet levels [crx_wavelet]

- **WHEN** a plane of odd tile size is transformed with one, two and three 5/3 levels and coded with version 1 quantizer 4 (a unit step)
- **THEN** each decodes to the written plane

#### Scenario: Quantized bands [crx_quantized]

- **WHEN** three-level coefficients are coded with version 1 quantizer 10 on every band
- **THEN** the plane equals the textbook inverse of the coefficients doubled

#### Scenario: Per-block quantizer tables [crx_qp_table]

- **WHEN** three-level coefficients are coded in a version 2 tile with a constant quantizer table, step base 0 and multiplier 16
- **THEN** the plane equals the textbook inverse of the coefficients doubled

#### Scenario: Encoding 3 [crx_encoding3]

- **WHEN** encoding 3 planes hold a level only in the first plane, then values in all four
- **THEN** red, both greens and blue take the values LibRaw's formula gives, clamped to the median precision

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
