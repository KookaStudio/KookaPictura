## MODIFIED Requirements

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

## ADDED Requirements

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
over little-endian 32-bit words. Fewer bytes than the packed size is
Fujifilm-compressed data and SHALL be refused as unsupported, with a message
saying that compressed sensor data is not supported yet. Any other size or
bit depth SHALL be refused as unsupported, and declared sizes SHALL be
charged against the image budget before the plane is allocated.

#### Scenario: Sixteen-bit words [raf_words]

- **WHEN** 14-bit planes are written as little-endian words in a little-endian section and as big-endian words in a big-endian section
- **THEN** each decodes to its samples normalized by the black and the 14-bit ceiling

#### Scenario: Packed samples [raf_packed]

- **WHEN** a 12-bit plane is packed least significant bit first and a 14-bit plane most significant bit first over 32-bit words, each with rows padded beyond the packed length, at an even and an odd width
- **THEN** each decodes to its samples normalized by the black and the ceiling

#### Scenario: Compressed data is refused by name [raf_compressed]

- **WHEN** a RAF's data holds one byte per photosite
- **THEN** it is named RAF and refused as unsupported, naming compressed sensor data

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
- **THEN** each is named RAF, each compressed, SuperCCD or EXR file is refused as unsupported, and each other reads non-flat and demosaics with each photosite's own channel equal to its sample
