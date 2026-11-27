## MODIFIED Requirements

### Requirement: An ORF's sensor plane is decoded from IFD0's strips

The ORF reader SHALL take the sensor image from IFD0 and its coding from the
strips' total size for `n` photosites: `2n` bytes SHALL be 16-bit words in
file order; `1.6n` bytes SHALL be 16-byte groups of ten 12-bit samples packed
least significant bit first followed by one pad byte; `1.5n` bytes SHALL be
12-bit samples packed most significant bit first, over little-endian 32-bit
words under an `IIRO` header and over bytes under an `IIRS` header, whose
strips SHALL be taken as two fields, the even rows and then the odd, each
strip holding whole rows whatever `RowsPerStrip` says. One strip of fewer
than `1.5n` bytes is the Olympus compressed stream and SHALL be decoded as
"Olympus-compressed ORF data is decoded as LibRaw decodes it" requires, at
12 or 14 valid bits; without the maker note's coding parameters it SHALL be
refused as unsupported, with a message naming the coding parameters. Any
other size, packed data of other than 12 valid bits, and compressed data of
other than 12 or 14, SHALL be refused as unsupported, and declared sizes
SHALL be charged against the image budget before the plane is allocated.

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

- **WHEN** 12-bit and 14-bit ORFs whose one strip holds one byte per photosite, and whose maker note has no coding parameters, are opened
- **THEN** each is named ORF and refused as unsupported, naming the coding parameters

#### Scenario: Refused variants [orf_unsupported]

- **WHEN** an ORF's strips total 1.75 bytes per photosite, several strips total fewer than 1.5, a packed ORF names 14 valid bits, or the CFA is not Bayer
- **THEN** reading fails with an unsupported-format error

### Requirement: An ORF takes levels, crop and white balance from its maker note

The reader SHALL read the maker note of each of the headers `OLYMP` (offsets
from the file start), `OLYMPUS` and `OM SYSTEM` (offsets from the note), and
from it, preferring ImageProcessing, then RawInfo, then the main note: the
black per 2 x 2 position, the valid bits, the as-shot red and blue
multipliers over 256, and the crop, which SHALL become the default crop of
the whole plane. Maker-note entries whose values lie outside the file SHALL
be skipped, as some bodies (E-P1, E-PL1) record one. The white SHALL be the first SensorCalibration value when it
lies above the black and within the precision, else the precision's ceiling.
At 14 valid bits the black and SensorCalibration, recorded at 12 bits, SHALL
be scaled by 4 first; 16-bit words SHALL scale both levels by their left
alignment. The CFA SHALL be Exif `CFAPattern`, 2 x 2 red, green, green, blue
in some phase, any other being refused as unsupported. Make, model and
orientation SHALL come from IFD0.

#### Scenario: Every maker-note layout [orf_maker_notes]

- **WHEN** the same levels, crop and multipliers are written in an `OLYMPUS` note, an `OM SYSTEM` note, an `OLYMP` note with ImageProcessing as a value, one with RawInfo, and one with only the main note's tags, the first and third also holding an entry whose value lies before the file
- **THEN** all decode to the same plane and neutral, and all but the last report the crop

#### Scenario: Camera tags and the demosaic [orf_tags]

- **WHEN** an ORF names make `OM Digital Solutions`, model `OM-5`, orientation 6 and a GRBG CFA
- **THEN** they are reported and the demosaic accepts the image

#### Scenario: Hostile files [orf_hostile]

- **WHEN** ORFs of each coding are cut at every length, their header and directories are corrupted, the crop exceeds the plane, or the width exceeds the image budget
- **THEN** reading returns without panicking, and a cut, an oversized crop and an oversized width fail with an error

#### Scenario: Real camera files [orf_corpus]

- **WHEN** `PICTURA_RAW_CORPUS` names a directory of user-supplied `.ORF` and `.ORI` files
- **THEN** each is named ORF, and each, compressed or not, reads non-flat, with the two green positions within 5 % of each other, and decodes with colour when it has a white balance

## ADDED Requirements

### Requirement: Olympus-compressed ORF data is decoded as LibRaw decodes it

The ORF reader SHALL decode Olympus-compressed sensor data as LibRaw
0.22.1's `olympus14.cpp` does, with the coding parameters of the maker
note's ImageProcessing tags `0x0640`-`0x0649` and `0x0650`-`0x0653`, each 0
when absent. The strip SHALL start with `0x0653` bytes holding a big-endian
1 and two zero bytes, else the data SHALL be refused as invalid. One bit
stream SHALL follow over the whole plane, read most significant bit first
from big-endian words and running on past the strip to the end of the
file. Each photosite SHALL be decoded from a sign, `0x0643` fine bits,
`0x0640` raw low bits, a unary or escaped high part and an adaptive number
of low bits, folded with LibRaw's fine context and added to LibRaw's
prediction from the same-colour neighbours two columns left and two rows
up. Parameters that would make a shift undefined (`0x0640` or `0x0643`
above 16, `0x0641` above 30, `0x0644` above 15, `0x0645` modulo 32 above
30, `0x0651` above 15) or a header length of 0 SHALL be refused as
unsupported, and a strip shorter than `2 + 0x0643 + 0x0640` bits per
photosite SHALL be refused as invalid before decoding. Corrupt streams
SHALL fail or decode without a panic.

#### Scenario: Compressed data [orf_compressed_decode]

- **WHEN** planes of noise beside a smooth ramp are coded at 12 bits with 2, 1 and 0 fine bits, at 14 bits with a SensorCalibration of 4000, and at 14 bits with two raw low bits and the fine context off
- **THEN** each decodes to the written plane, normalized by its black and white, scaled by 4 at 14 bits

#### Scenario: Notes carrying the coding [orf_compressed_notes]

- **WHEN** the same compressed plane's coding is written in an `OLYMPUS` note, an `OM SYSTEM` note, and an `OLYMP` note with ImageProcessing as a value
- **THEN** all decode to the written plane

#### Scenario: Coding checks [orf_compressed_coding]

- **WHEN** a compressed ORF has no coding parameters, a parameter outside the decoder's bounds, 16 valid bits, or a header that does not hold 1
- **THEN** the first three are refused as unsupported and the header as invalid

#### Scenario: Hostile compressed files [orf_compressed_hostile]

- **WHEN** a compressed ORF is cut at every length, has bytes of its stream flipped, or has a strip far shorter than its plane
- **THEN** reading returns without panicking, and the cuts and the short strip fail with an error

#### Scenario: Real files match LibRaw [orf_compressed_libraw]

- **WHEN** `PICTURA_RAW_CORPUS` holds compressed ORFs and python3 with rawpy is available
- **THEN** each decodes to LibRaw's raw plane of the same size, sample for sample after the same normalization
