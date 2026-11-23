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
`IIU\0`, with at least eight bytes, is Panasonic RW2 whatever its make. Any
other input SHALL be named as no raw
container. `read_raw` SHALL read a DNG exactly as
`read_dng` does, a CR2 with the CR2 reader, a CR3 with the CR3 reader, a NEF
with the NEF reader, an
ARW with the ARW reader, an ORF with the ORF reader and an RW2 with the RW2
reader, all returning the same normalized CFA image type, and
SHALL refuse any other input with an unsupported-format error.
`sniff_raw_prefix` SHALL agree with `sniff_raw` on a prefix holding IFD0's
entry table and its `Make` value, or a CR3's first twelve bytes.

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

## ADDED Requirements

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
active corner, and 0 without a mask. The white SHALL be `2^precision − 1`.
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
