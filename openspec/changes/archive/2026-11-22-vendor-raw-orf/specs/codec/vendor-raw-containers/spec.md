## MODIFIED Requirements

### Requirement: Raw containers are recognized and read through one entry point

`sniff_raw` SHALL name a supported raw container from its leading bytes: a
TIFF or BigTIFF whose IFD0 carries `DNGVersion` is DNG, a little-endian
TIFF with `CR` at byte 8 and major version 2 at byte 10 is Canon CR2, and a
classic TIFF without `DNGVersion` whose IFD0 lists `SubIFDs` and whose IFD0
`Make` begins with `NIKON` in any case is Nikon NEF, and any other TIFF without
`DNGVersion` whose IFD0 `Make` begins with `SONY` is Sony ARW. A file whose
first four bytes are `IIRO`, `IIRS` or `MMOR`, with at least eight bytes, is
Olympus ORF whatever its make. Any other input SHALL be named as no raw
container. `read_raw` SHALL read a DNG exactly as
`read_dng` does, a CR2 with the CR2 reader, a NEF with the NEF reader, an
ARW with the ARW reader and an ORF with the ORF reader, all returning the same normalized CFA image type, and
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

## ADDED Requirements

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
