## MODIFIED Requirements

### Requirement: Raw containers are recognized and read through one entry point

`sniff_raw` SHALL name a supported raw container from its leading bytes: a
TIFF or BigTIFF whose IFD0 carries `DNGVersion` is DNG, a little-endian
TIFF with `CR` at byte 8 and major version 2 at byte 10 is Canon CR2, and a
classic TIFF without `DNGVersion` whose IFD0 lists `SubIFDs` and whose IFD0
`Make` begins with `NIKON` in any case is Nikon NEF, and any other TIFF without
`DNGVersion` whose IFD0 `Make` begins with `SONY` is Sony ARW. A file whose
first four bytes are `IIRO`, `IIRS` or `MMOR`, with at least eight bytes, is
Olympus ORF whatever its make, and one whose first four bytes are
`IIU\0`, with at least eight bytes, is Panasonic RW2 whatever its make. Any
other input SHALL be named as no raw
container. `read_raw` SHALL read a DNG exactly as
`read_dng` does, a CR2 with the CR2 reader, a NEF with the NEF reader, an
ARW with the ARW reader, an ORF with the ORF reader and an RW2 with the RW2
reader, all returning the same normalized CFA image type, and
SHALL refuse any other input with an unsupported-format error.
`sniff_raw_prefix` SHALL agree with `sniff_raw` on a prefix holding IFD0's
entry table and its `Make` value.

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

## ADDED Requirements

### Requirement: An RW2's sensor plane is decoded from its plain packings

The RW2 reader SHALL read IFD0 at the offset stored at byte 4. The sensor
plane SHALL be `SensorWidth` (`0x0002`) by `SensorHeight` (`0x0003`), stored at
`RawDataOffset` (`0x0118`) or, without it, `StripOffsets`, in exactly
`StripByteCounts` bytes. It SHALL decode three codings:

- compression 34828 or 34830 without `RawFormat` (`0x002D`), at 12 or 14
  bits: little-endian 16-bit words;
- compression 34316 with RawFormat 2 at 12 bits: 16-byte groups of ten samples;
- compression 34316 with RawFormat 7 at 14 bits: 16-byte groups of nine samples.

A group's samples SHALL be taken from its least significant bit up, the group
read as a little-endian number, and its remaining bits ignored. Any other
RawFormat SHALL be refused as unsupported, naming the RawFormat. Any other
compression or bit depth, a packed width that is no whole number of groups,
and a length other than the coding's SHALL be refused as unsupported.

#### Scenario: Sixteen-bit words [rw2_words]

- **WHEN** 12-bit samples shifted left by four and 14-bit samples shifted left by two are stored as words under compression 34828 and 34830
- **THEN** the plane equals the stored words normalized by the levels times 16

#### Scenario: Sixteen-byte groups [rw2_packed]

- **WHEN** a plane is packed as RawFormat 2 and as RawFormat 7 with every padding bit set, and placed by `StripOffsets` and by `RawDataOffset`
- **THEN** each plane equals the normalized samples

#### Scenario: Other codings are refused by name [rw2_refused]

- **WHEN** files declare RawFormat 3, 4, 5, 6 and 8, with and without a compression tag
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
`0x001C`-`0x001E` for red, green and blue, 0 when absent; the white SHALL be
the green linearity limit `0x000F` when it lies above the blacks and within
the coding's range, else that range's top (4095 for words). Word files SHALL
scale both by 16. The as-shot neutral SHALL be the green level over each of
the red, green and blue white-balance levels `0x0024`-`0x0026`, and absent
when one is zero or missing. The crop `0x002F`-`0x0032` (top, left, bottom,
right in sensor coordinates) SHALL become the default crop when it lies in
the active area, else the whole active area is the crop. Make, model and
orientation SHALL come from IFD0. Colour matrices SHALL come only from the
fitted camera table. A malformed file SHALL fail without a panic and without
an allocation beyond the image budget.

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

- **WHEN** files of each coding are cut at every length, have header and directory bytes corrupted, declare reversed or outside borders, or declare a width over the budget
- **THEN** every cut, bad border and oversize file is an error and none panics

#### Scenario: Real camera files [rw2_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` holds RW2, RWL or Panasonic RAW files
- **THEN** each plain-packed one reads, is not flat and has green phases that agree, and each other one is refused naming its RawFormat
