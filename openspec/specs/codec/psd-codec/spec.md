# psd-codec Specification

## Purpose
The PSD/PSB codec: signature detection, header validation, raw and RLE composite read/write, and typed errors.

## Requirements

### Requirement: PSD/PSB signature and version detection
The system SHALL read the 4-byte `8BPS` signature and SHALL reject any other
signature with `PsdError::BadSignature`; it SHALL accept version 1 (PSD) and
version 2 (PSB) and SHALL reject any other version with `PsdError::Unsupported`.

#### Scenario: Wrong signature
- **WHEN** `read_psd` is given bytes that do not start with `8BPS`
- **THEN** it returns `PsdError::BadSignature` and does not panic

#### Scenario: Unsupported version marker
- **WHEN** the version field is neither 1 nor 2
- **THEN** it returns `PsdError::Unsupported`

### Requirement: Header validation for the composite subset
The system SHALL parse the channel count, height, width, bit depth, and color
mode, and SHALL accept 1–56 channels, non-zero dimensions within the PSD/PSB
dimension limit, color modes Bitmap (0), Grayscale (1), Indexed (2), RGB (3),
CMYK (4), or Lab (9), and bit depth 8 for every accepted mode, bit depth 1 for
Bitmap only, and bit depths 16 and 32 for the Grayscale, RGB, CMYK, and Lab
modes (see `psd-bit-depth`). It SHALL return `PsdError::Unsupported` for
Multichannel (7), Duotone (8), any other color-mode code, any bit depth other
than 1, 8, 16, or 32, and bit depth 16 or 32 for a Bitmap or Indexed document;
it SHALL return `PsdError::Invalid` for a channel count or dimension outside
the accepted range.

#### Scenario: Zero dimension
- **WHEN** the header declares a width or height of zero
- **THEN** it returns `PsdError::Invalid`

#### Scenario: Depth 16 and 32 are accepted
- **WHEN** the header declares bit depth 16 or 32 for Grayscale, RGB, CMYK, or Lab
- **THEN** the header is accepted and the samples are read and narrowed to 8-bit

#### Scenario: Unsupported depth
- **WHEN** the header declares a bit depth other than 1, 8, 16, or 32, or bit depth 16/32 for a Bitmap or Indexed document
- **THEN** it returns `PsdError::Unsupported`

#### Scenario: Bitmap depth 1 is accepted
- **WHEN** the header declares Bitmap mode with bit depth 1
- **THEN** the header is accepted and the composite is decoded as 1-bit rows

#### Scenario: Channel count out of range
- **WHEN** the header declares zero channels or more than 56 channels
- **THEN** it returns `PsdError::Invalid`

#### Scenario: A non-RGB color mode is accepted
- **WHEN** the header declares Indexed (2), CMYK (4), or Lab (9)
- **THEN** the header is accepted and the document is normalized to RGB

#### Scenario: Multichannel and Duotone are unsupported
- **WHEN** the header declares Multichannel (7) or Duotone (8)
- **THEN** it returns `PsdError::Unsupported`

### Requirement: Composite image read with raw and RLE compression
The system SHALL read the color-mode data and image-resource sections by their
declared lengths and SHALL retain their bytes verbatim on the document for
lossless re-save (instead of discarding them), except that a color-mode-data
section belonging to an Indexed document SHALL be interpreted as the 768-byte
palette (see `psd-color-modes`) and SHALL NOT be retained on the normalized
document. It SHALL read the image-data section and decode the composite color
planes for compression `0` (raw), `1` (PackBits RLE), `2` (ZIP), and `3`
(ZIP-with-prediction), returning `PsdError::Unsupported` for any other
compression method. Each channel's row byte stride SHALL be
`ceil(width * depth / 8)`, so a depth-1 Bitmap row is `ceil(width / 8)` bytes
and a depth-16 or depth-32 row is `2 * width` or `4 * width` bytes. ZIP is a
zlib-framed deflate stream (a raw-deflate stream SHALL also be accepted);
ZIP-with-prediction SHALL invert the per-scanline delta after inflating,
byte-wise at depth 8, per-big-endian-`u16` at depth 16, and through the
four-byte-plane shuffle at depth 32 (see `psd-bit-depth`). ZIP compression at
depth 1 SHALL be `PsdError::Unsupported`. A malformed deflate stream SHALL
return a typed error, never a panic.

#### Scenario: Read a raw RGB composite
- **WHEN** an RGB PSD stores the image data section with compression 0
- **THEN** `read_psd` returns a document whose composite planes equal the stored bytes

#### Scenario: Read a PackBits RLE composite
- **WHEN** the image-data section uses compression 1 with per-scanline byte counts
- **THEN** `read_psd` decodes the PackBits literal and repeat runs into the composite planes

#### Scenario: Read a ZIP composite
- **WHEN** the image-data section uses compression 2 and holds a zlib deflate stream
- **THEN** `read_psd` decodes the composite planes from the inflated bytes

#### Scenario: Read a ZIP-with-prediction composite
- **WHEN** the image-data section uses compression 3
- **THEN** `read_psd` inflates the stream and inverts the byte-wise per-row delta to recover the planes

#### Scenario: Read a depth-16 composite
- **WHEN** a depth-16 image-data section stores a raw or RLE composite
- **THEN** each row is decoded at a `2 * width` stride and the big-endian `u16` samples are narrowed to 8-bit

#### Scenario: Read a depth-32 ZIP-with-prediction composite
- **WHEN** a depth-32 image-data section uses compression 3
- **THEN** the stream is inflated, the four-byte-plane shuffle and the byte-wise delta are undone, and each big-endian `f32` sample is narrowed to 8-bit

#### Scenario: Read a depth-1 Bitmap RLE composite
- **WHEN** a depth-1 Bitmap document stores compression 1
- **THEN** each scanline's RLE decode writes `ceil(width / 8)` bytes and the row expands to black and white pixels

#### Scenario: Color-mode and image-resource bytes are retained
- **WHEN** a non-Indexed PSD carries a non-empty color-mode-data or image-resource section
- **THEN** those exact bytes are exposed on the returned document

#### Scenario: An Indexed palette is interpreted, not retained
- **WHEN** an Indexed PSD carries a 768-byte color-mode-data palette
- **THEN** the palette expands the composite to RGB and the normalized document's color-mode-data is empty

#### Scenario: Reject an unsupported compression method
- **WHEN** the image-data compression method is none of 0, 1, 2, or 3
- **THEN** it returns `PsdError::Unsupported`

#### Scenario: Reject ZIP at depth 1
- **WHEN** a depth-1 Bitmap document declares compression 2 or 3
- **THEN** it returns `PsdError::Unsupported`

### Requirement: PSB uses 64-bit lengths and 4-byte RLE counts
The system SHALL read version-2 (PSB) files using 8-byte section lengths and
4-byte per-scanline RLE byte counts, versus 4-byte and 2-byte respectively for
version-1 (PSD) files.

#### Scenario: RLE PSB
- **WHEN** a PSB header declares compression 1
- **THEN** the scanline byte-count table is read as 4-byte entries and the composite decodes correctly

### Requirement: Composite PSD write and round-trip

The system SHALL serialize an 8-bit RGB or Grayscale `Document` to a valid PSD
(version 1) containing the header, the document's color-mode-data and
image-resource bytes (empty for a freshly constructed document), the layer/mask
section, and an image-data section whose composite is encoded with the document's
recorded composite compression (PackBits RLE by default, or ZIP or
ZIP-with-prediction when the document was read from a file using one), except
that a document whose merged composite is absent SHALL write no image-data
section; reading the output with `read_psd` SHALL produce a document equal to the
input.

#### Scenario: Round-trip RGB and Grayscale

- **WHEN** a document is written with `write_psd` and read back with `read_psd`
- **THEN** the resulting document equals the original

#### Scenario: Randomized composite round-trip

- **WHEN** a fixed-seed sequence of random small RGB and Grayscale documents is written and read back
- **THEN** every document round-trips equal

#### Scenario: Preserved header sections are re-emitted

- **WHEN** a document read from a file with non-empty color-mode-data or image-resource sections is written
- **THEN** the output carries those same bytes in their sections

#### Scenario: A constructed document matches the RLE golden

- **WHEN** a default document with empty preservation storage is written
- **THEN** its recorded compression defaults to RLE and the output is byte-identical to the refreshed fixed PackBits-RLE golden fixture

#### Scenario: A ZIP-composite document round-trips with its compression

- **WHEN** a document whose composite compression is ZIP is written and read back
- **THEN** the image-data section declares compression `2`, the composite bytes are recovered exactly, and the re-read document equals the input

#### Scenario: A document without a merged composite writes none

- **WHEN** a document read from a file that ended after the layer section is written
- **THEN** the output has no image-data section and reading it back yields an equal document with no merged composite

#### Scenario: Reject an unwritable document

- **WHEN** `write_psd` is given a document that is not 8-bit or not RGB/Grayscale
- **THEN** it returns `PsdError::Unsupported`

### Requirement: Malformed input returns errors, never panics
The system SHALL validate every length and offset before use and SHALL return a
typed `PsdError`, never a panic or an out-of-bounds read, for truncated files,
impossible counts, malformed RLE runs, and size arithmetic overflow.

#### Scenario: Truncated file
- **WHEN** any required field or section is missing bytes
- **THEN** `read_psd` returns `PsdError::Truncated`

#### Scenario: PackBits run overrun
- **WHEN** an RLE control byte would decode past the scanline or past the input
- **THEN** `read_psd` returns `PsdError::Invalid`

### Requirement: Typed codec errors
The system SHALL define a `PsdError` enum with `thiserror` whose variants
distinguish bad signature, unsupported features, truncation, and invalid data.

#### Scenario: Failure class is matchable
- **WHEN** a caller matches on the error returned by `read_psd` or `write_psd`
- **THEN** `BadSignature`, `Unsupported`, `Truncated`, and `Invalid` are distinguishable

### Requirement: Absent merged composite
The system SHALL still return a document carrying the parsed layer tree when the
image-data section is absent (a document saved with "Maximize Compatibility"
off), synthesizing a zero-filled composite of the header dimensions and mode
rather than failing with a truncation error. A header whose data ends before the
layer section SHALL still return an error.

#### Scenario: Layers parse without a composite
- **WHEN** a layered PSD's bytes end after the layer-and-mask section
- **THEN** `read_psd` returns the layers and a zero-filled composite of the header size

#### Scenario: Truncation before the layers is still an error
- **WHEN** the file ends before the layer-and-mask section completes
- **THEN** `read_psd` returns a typed error

### Requirement: PSB write

`write_psd` SHALL emit a version-1 PSD for a document whose width and height are
each at most 30 000 (`MAX_DIM_PSD`) and whose source was a PSD, leaving its
output bytes unchanged, and SHALL emit a version-2 PSB when the source document
was a PSB (`Document.is_psb`) or when either dimension exceeds 30 000, because a
PSD can neither represent those dimensions nor carry the widened length fields a
source PSB's preserved tagged blocks require. A new
`write_psb(doc: &Document) -> Result<Vec<u8>, PsdError>` SHALL always emit a
version-2 PSB; both entry points SHALL share one container writer. A PSB SHALL
write the header version word `2` and SHALL use 8-byte (`u64`) big-endian fields
for the layer-and-mask section length, the layer-info length, and each layer
record's per-channel data length, and 4-byte (`u32`) big-endian entries for the
RLE scanline byte-count table in the composite image-data section and in every
layer channel and mask stream. The global layer-mask info length SHALL remain a
4-byte (`u32`) field in both containers. A per-layer additional-layer-information
(tagged) block SHALL declare an even length with the pad byte included in that
length, so a pad byte is never emitted outside the declared length. A
document-level (global) additional-layer-information block SHALL declare its
exact data length and SHALL be padded externally to a 4-byte boundary. In a PSB,
such a block whose key is a PSB big key (the psd-tools `_BIG_KEYS` set,
including `lnk2`/`lnk3`/`lnkE`, `Lr16`/`Lr32`/`Layr`, `LMsk`, `Alph`,
`FMsk`, `PxSD`, `pths`, `Mtrn`/`Mt16`/`Mt32`, `cinf`, `extd`/`extn`, `artd`,
`FXid`/`FEid`/`FELS`) SHALL write an 8-byte length; non-big keys SHALL stay
4 bytes in both containers. A preserved document-level tagged block SHALL be
re-framed to the output container's length width (8 bytes in a PSB, 4 bytes in a
PSD) and re-padded to a 4-byte boundary before it is emitted. The `iOpa`
blend-fill-opacity block SHALL be written as a 4-byte block (`[fill, 0, 0, 0]`),
matching the Photoshop `B3x` layout. Dimensions SHALL be
accepted up to 300 000
(`MAX_DIM_PSB`), and a width or height above 300 000 SHALL return
`PsdError::Unsupported`. RLE (compression `1`) SHALL remain the only compression
written. A stream preserved on `Layer.raw_channels` SHALL be re-emitted
byte-for-byte with its own compression header, widening only its declared length
field, and repeated writes of the same document SHALL be byte-identical.

#### Scenario: Automatic PSB selection above the PSD limit

- **WHEN** `write_psd` is given a document whose width or height is above 30 000 and at most 300 000
- **THEN** the output header version word is `2` and `read_psd` recovers a document equal to the input

#### Scenario: write_psb always emits version 2

- **WHEN** `write_psb` is given a document within the PSD dimension limit
- **THEN** the output header version word is `2` and `read_psd` recovers a document equal to the input

#### Scenario: A PSB source re-saves as a PSB

- **WHEN** a document read from a version-2 PSB (`Document.is_psb` true) with both dimensions at most 30 000 is written with `write_psd`
- **THEN** the output is a version-2 PSB whose bytes round-trip through `read_psd` equal to the source document

#### Scenario: PSB length fields widen and RLE counts widen

- **WHEN** a PSB is written for a document with a layer and a mask
- **THEN** the layer-and-mask section length, the layer-info length, and each layer channel data length are 8-byte fields, each RLE scanline byte-count entry is 4 bytes in both the composite and the layer/mask streams, and the global layer-mask info length stays 4 bytes

#### Scenario: Big-key block length widens in a PSB

- **WHEN** a PSB is written for a layer carrying a preserved additional-layer-information block with a PSB big key (for example `Lr16` or `lnk2`)
- **THEN** that block's length field is 8 bytes, and the block round-trips through `read_psd` equal to the input

#### Scenario: PSB with an authored smart object is readable by psd-tools

- **WHEN** a document with an embedded smart object (authored by the writer) is serialized with `write_psb`
- **THEN** psd-tools opens the file and parses its document-level `lnk2` big-key block without a framing error

#### Scenario: An odd-length block does not corrupt following blocks

- **WHEN** a PSB is written for a layer carrying an odd-length additional-layer-information block followed by a later block
- **THEN** the odd block declares an even length with the pad byte inside that length and psd-tools reads the later block

#### Scenario: A document-level block declares its exact length and pads externally to 4

- **WHEN** a PSB is written for a document carrying a preserved document-level tagged block whose data length is not a multiple of 4
- **THEN** the block declares that exact length and is padded externally to a 4-byte boundary, and psd-tools reads the written file including a later block

#### Scenario: A PSD-sourced document with a preserved big-key block is readable when written as a PSB

- **WHEN** `write_psb` is given a document read from a PSD whose preserved document-level `lnk2` block has a 4-byte length
- **THEN** the `lnk2` block is re-framed with an 8-byte length and psd-tools reads the written file

#### Scenario: Large-dimension round-trip

- **WHEN** a document wider than 30 000 (for example 30 001×1) is written and read back
- **THEN** the resulting document equals the original

#### Scenario: Preserved verbatim channel length widens

- **WHEN** a document carrying a `Layer.raw_channels` stream is written as a PSB
- **THEN** the stream bytes are emitted byte-for-byte with their own compression header and only the declared length field is 8 bytes

#### Scenario: Dimension above the PSB limit errors

- **WHEN** `write_psb` is given a document whose width or height exceeds 300 000
- **THEN** it returns `PsdError::Unsupported` and does not panic

### Requirement: Engine-encoded PSD channels are written with the document's recorded compression

`write_psd` SHALL encode every channel it owns with the compression recorded on
the document: the merged composite's color planes and any document extra channels
in the image-data section use the recorded composite compression, and every
engine-encoded layer channel — the layer's color channels and its raster mask
(`-2`) — uses the recorded layer-channel compression. The supported kinds are
PackBits RLE (`1`), ZIP (`2`), and ZIP-with-prediction (`3`); a document with no
recorded kind SHALL write RLE. RLE SHALL use the standard layout: the 2-byte
compression word, one 2-byte big-endian byte count per scanline (PSD widths, so
`u16` counts), then the PackBits rows in channel-major, row-major order. ZIP SHALL
zlib-wrap the concatenated planar bytes; ZIP-with-prediction SHALL apply the
reversible per-row delta (`out[i] = data[i] - data[i-1]` within each `width`-byte
row) before deflating. Channels preserved verbatim on `Layer.raw_channels` SHALL
be re-emitted byte-for-byte and SHALL NOT be re-encoded. The encoding SHALL be
deterministic. An RLE row whose PackBits encoding would exceed the `u16`
byte-count limit SHALL return a `PsdError` and SHALL NOT be truncated. A document
whose channels mix compression kinds within a category normalizes to the first
kind recorded for that category.

#### Scenario: Composite declares the document's compression

- **WHEN** an 8-bit RGB or Grayscale document is written with `write_psd`
- **THEN** the image-data section's 2-byte compression word is the document's recorded composite compression and `read_psd` recovers the composite pixels exactly

#### Scenario: Layer color channel and raster mask use the recorded compression

- **WHEN** a document with a pixel layer and a raster mask whose layer compression is ZIP is written
- **THEN** the layer's channel records declare compression `2` and `read_psd` reconstructs the channel and mask bytes exactly

#### Scenario: Document extra channels share the composite compression

- **WHEN** a document carrying a saved-selection extra channel is written
- **THEN** the extra plane shares the image-data section's compression and round-trips unchanged

#### Scenario: Preserved verbatim channels are re-emitted byte-for-byte

- **WHEN** a layer carries a `Layer.raw_channels` stream with its own compression header
- **THEN** read→write→read preserves that stream's exact bytes and the encoder does not re-encode it

#### Scenario: RLE and ZIP write are lossless

- **WHEN** a document read from disk is written and read back
- **THEN** the decoded pixel data of the composite, extra channels, layer channels, and raster mask equals the input

#### Scenario: ZIP-with-prediction writes reversibly

- **WHEN** a document whose compression is ZIP-with-prediction is written and read back
- **THEN** the section declares compression `3`, the predictor is inverted, and the pixels equal the input

#### Scenario: Output is deterministic and standard-reader readable

- **WHEN** the same document is written twice, or a ZIP or ZIP-with-prediction document is written and opened by the `psd-tools` oracle
- **THEN** the two outputs are byte-identical and `psd-tools` decodes the composite and channels to the same pixels

#### Scenario: Oversized scanline errors instead of truncating

- **WHEN** an RLE scanline's PackBits encoding exceeds the `u16` byte-count limit
- **THEN** `write_psd` returns a `PsdError` and writes no truncated count

#### Scenario: Section and folder records do not set the layer compression

- **WHEN** a document read from a file whose first layer record is a section divider is written
- **THEN** the recorded layer compression comes from a surviving layer channel (or defaults to RLE) and the round-trip document is equal

### Requirement: Bitmap documents with extra channels write

`write_psd` SHALL write a depth-1 Bitmap document that carries extra channels.
While the document is a flat, unedited Bitmap (no layer tree and its retained
packed composite still expands to the working composite), the writer SHALL emit
header mode Bitmap (0) at depth 1 with the retained packed color plane and every
extra channel at the depth-1 packed stride (`ceil(width / 8) * height`),
preserving the channel bytes. When the composite has changed, or the document is
otherwise not a flat unchanged Bitmap, and a merged composite is present, the
writer SHALL fall back to 8-bit RGB and SHALL expand every packed extra channel
to one 8-bit sample per pixel before writing. A Bitmap document without a merged
composite SHALL keep the existing no-image-data behaviour, because the extra
channels ride with the composite. When a merged composite is present, the write
SHALL succeed in both cases and `read_psd` SHALL return the document's extra
channels.

#### Scenario: An unchanged Bitmap keeps its packed extra channel

- **WHEN** a depth-1 Bitmap document with one packed extra channel is written and read back without changing its composite
- **THEN** the output header declares mode Bitmap at depth 1 and the extra channel's selected/unselected pattern survives

#### Scenario: An edited Bitmap falls back to RGB and expands the extra channel

- **WHEN** the composite of a Bitmap document with one packed extra channel is changed and the document is written and read back
- **THEN** the output writes 8-bit RGB and the extra channel has one sample per pixel with the same selected/unselected pattern
