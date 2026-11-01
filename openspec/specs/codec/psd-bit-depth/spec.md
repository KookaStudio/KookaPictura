# psd-bit-depth Specification

## Purpose
Reads depth-16 and depth-32 PSD data, narrows layer channels with the document, and reports the normalized depth.

## Requirements

### Requirement: Depth 16 and 32 are read and normalized to 8-bit

`read_psd` SHALL accept bit depth 16 and bit depth 32 for the Grayscale, RGB,
CMYK, and Lab modes, decode their samples big-endian, narrow each sample to
8-bit, and set the returned document's `depth` to `Eight`. A header bit depth
other than 1, 8, 16, or 32, and bit depth 16 or 32 for a Bitmap or Indexed
document, SHALL return `PsdError::Unsupported`. After narrowing, the document
SHALL be normalized for its color mode exactly as an 8-bit document (see
`psd-color-modes`).

#### Scenario: A 16-bit RGB document opens as 8-bit RGB

- **WHEN** a depth-16 RGB PSD is read
- **THEN** `read_psd` returns a document whose `mode` is `Rgb`, whose `depth` is `Eight`, and whose composite is the 8-bit narrowing of the stored `u16` samples

#### Scenario: A 32-bit RGB document opens as 8-bit RGB

- **WHEN** a depth-32 RGB PSD is read
- **THEN** `read_psd` returns a document whose `mode` is `Rgb`, whose `depth` is `Eight`, and whose composite is the 8-bit narrowing of the stored `f32` samples

#### Scenario: A 16-bit CMYK document is narrowed then converted

- **WHEN** a depth-16 CMYK PSD is read
- **THEN** the samples are narrowed to 8-bit and the document is normalized to RGB by the profile-free CMYK conversion

#### Scenario: Bitmap and Indexed remain depth 1 and 8

- **WHEN** a Bitmap document declares depth 16/32, or an Indexed document declares depth 16/32
- **THEN** `read_psd` returns `PsdError::Unsupported` and does not panic

### Requirement: Depth-16 samples are big-endian u16 with a two-byte row stride

For a depth-16 document each sample SHALL be an unsigned big-endian 16-bit
integer and each channel row SHALL occupy `2 * width` bytes. A depth-16
scanline's RLE byte count SHALL count its compressed bytes and its decode SHALL
write exactly `2 * width` bytes. ZIP-with-prediction at depth 16 SHALL be
inverted as a per-row running sum of big-endian `u16` values, `v[x] = (v[x] +
v[x-1]) mod 2^16`. Each 16-bit sample SHALL narrow to 8-bit as `sample >> 8`.

#### Scenario: A 16-bit sample narrows by its high byte

- **WHEN** a 16-bit sample stores `0`, `255`, `256`, `32768`, or `65535`
- **THEN** the narrowed 8-bit value is `0`, `0`, `1`, `128`, or `255` respectively

#### Scenario: A 16-bit RLE row decodes at a two-byte stride

- **WHEN** a depth-16 composite uses RLE compression
- **THEN** each scanline's PackBits decode writes `2 * width` bytes into the row before narrowing

#### Scenario: 16-bit ZIP-with-prediction is a per-u16 running sum

- **WHEN** a depth-16 composite uses ZIP-with-prediction
- **THEN** after inflating, each `u16` is added to its left neighbour within the row modulo `2^16` before narrowing

### Requirement: Depth-32 samples are big-endian f32 with a four-byte row stride

For a depth-32 document each sample SHALL be a big-endian IEEE-754 `f32` and
each channel row SHALL occupy `4 * width` bytes. ZIP-with-prediction at depth 32
SHALL store the four byte planes of each row shuffled together and delta'd
byte-wise, so the read SHALL invert the byte-wise delta over the row and
un-shuffle the four planes back into big-endian floats. Each `f32` sample SHALL
narrow to 8-bit as `clamp(trunc(sample * 256), 0, 255)`. The narrowing SHALL be
display-referred: a value at or above 1.0 SHALL clip to 255, a value at or below
0.0 SHALL clip to 0, and no HDR tone mapping or transfer function SHALL be
applied.

#### Scenario: A 32-bit sample narrows by `sample * 256`, clipped

- **WHEN** a 32-bit sample stores `0.0`, `0.5`, `1.0`, `1.5`, or `-0.5`
- **THEN** the narrowed 8-bit value is `0`, `128`, `255`, `255`, or `0` respectively

#### Scenario: 32-bit ZIP-with-prediction is un-shuffled then delta-decoded

- **WHEN** a depth-32 composite uses ZIP-with-prediction
- **THEN** the byte-wise delta is inverted over the `4 * width`-byte row and the four byte planes are un-shuffled back into big-endian floats

### Requirement: Layer channels are narrowed with the document

When a 16- or 32-bit document carries layers, `read_psd` SHALL decode each layer
channel — its color channels, the `-1` transparency channel, the `-2` mask
channel, and unmodeled channels — at the document depth and SHALL narrow it to
an 8-bit plane before assembling the layer, so the layer path agrees with the
narrowed composite. The layer's color channels SHALL then be converted by the
document's color-mode normalization. For a Grayscale or RGB document the
read path SHALL also retain the decoded source-depth samples of each layer
channel so an unchanged channel can be re-emitted at the source depth on save
(see the source-depth-on-save requirement).

#### Scenario: A 16-bit layer color channel is narrowed

- **WHEN** a depth-16 document carries a pixel layer whose color channel stores `u16` samples
- **THEN** the returned layer's color channel holds the 8-bit narrowing of those samples

#### Scenario: A 16-bit mask channel is narrowed

- **WHEN** a depth-16 document carries a `-2` mask channel
- **THEN** the returned mask plane is the 8-bit narrowing of the stored samples and is not decoded as an 8-bit row

#### Scenario: A 16-bit unmodeled channel is narrowed for the engine and restored on save

- **WHEN** a depth-16 document carries a layer channel the engine does not model (for example a `-3` or positive spot channel)
- **THEN** the channel is decoded at the document depth and narrowed to an 8-bit plane for the engine, and a save of the unedited document re-emits it at the source depth

#### Scenario: Layers stored in a tagged block are narrowed the same way

- **WHEN** a depth-16 or depth-32 document stores its layers in an `Lr16` or `Lr32` block
- **THEN** the returned layers, their narrowed channels, and their retained source-depth samples equal those of the same layers stored in the layer-info subsection

### Requirement: The application reports a normalized bit depth

The application SHALL expose the source bit depth of an opened document and
SHALL present a conversion notice to the user when a 16/32-bit file is
normalized, so the user knows the document is edited at 8-bit working precision
and, for a Grayscale or RGB document, that a save preserves the source depth
(with unchanged planes exact and edited planes widened). For a mode the read
path converted (such as CMYK or Lab) the notice SHALL report the conversion
without claiming the save preserves the depth, because it saves 8-bit.

#### Scenario: Opening a 16-bit file shows a notice

- **WHEN** the application opens a depth-16 Grayscale or RGB PSD
- **THEN** the view reports that the document was converted from 16-bit, the document's depth is 8-bit, and the notice does not claim the save will be 8-bit

#### Scenario: Opening a 16-bit converted-mode file still shows a notice

- **WHEN** the application opens a depth-16 CMYK or Lab PSD
- **THEN** the view reports the conversion from 16-bit and does not claim the save preserves the depth

#### Scenario: Opening an 8-bit file shows no depth notice

- **WHEN** the application opens an 8-bit PSD
- **THEN** no depth-conversion notice is shown

### Requirement: Depth fixtures agree with an independent decoder

The system SHALL commit the fixtures `crates/pictura-codec/tests/fixtures/rgb16.psd`
and `crates/pictura-codec/tests/fixtures/rgb32.psd`, regenerated byte-stably by
`scripts/generate-fixtures.py`, and a test SHALL verify that `read_psd`'s
narrowed composite agrees exactly (0 tolerance) with the independent
`psd-tools` decoder for both fixtures. The oracle SHALL self-skip with a clear
message when `psd-tools` is unavailable and SHALL NOT fail the suite in that
case.

#### Scenario: The 16-bit fixture agrees with psd-tools

- **WHEN** `read_psd` decodes `rgb16.psd` and `psd-tools` decodes the same file
- **THEN** every narrowed 8-bit pixel is identical

#### Scenario: The 32-bit fixture agrees with psd-tools

- **WHEN** `read_psd` decodes `rgb32.psd` and `psd-tools` decodes the same file
- **THEN** every narrowed 8-bit pixel is identical

#### Scenario: The oracle self-skips without psd-tools

- **WHEN** the `psd-tools` package is not available
- **THEN** the oracle reports a skip and the suite passes

### Requirement: A normalized document records its source depth and preserves it on save

`read_psd` SHALL set `Document.source_depth` to `Some(BitDepth::Sixteen)` or
`Some(BitDepth::ThirtyTwo)` for any 16/32-bit document it accepts (so the
application can report the conversion), and to `None` for an 8-bit document, a
depth-1 Bitmap document, and a constructed document. For a 16/32-bit Grayscale,
RGB, Lab, or CMYK document it SHALL retain the decoded source-depth samples of
the composite color planes, the document extra channels, and every layer
channel. `write_psd` SHALL write the output header at the document's source depth
when the document retained samples, else 8. For each plane, when the retained
source-depth samples narrow to the plane's current 8-bit bytes the writer SHALL
re-encode the retained samples at the source depth; when they do not (the plane
was edited, or the layer moved) the writer SHALL widen the current 8-bit plane
to the source depth and encode it, so a save of a 16/32-bit Grayscale, RGB, Lab,
or CMYK document is not a silent downgrade to 8-bit. Widening SHALL be `v * 257`
at 16-bit and the 8-bit value scaled to `[0, 1]` at 32-bit and SHALL be
documented as an approximation that cannot recover the source low bits or HDR
range. Compression SHALL follow the document's recorded kinds at the source
depth: raw, PackBits RLE, ZIP, and ZIP-with-prediction, whose forward predictor
SHALL be the depth-specific inverse of the read predictor (a per-`u16` difference
at 16-bit and the byte difference plus the four-byte-plane shuffle at 32-bit). A
document with no recorded source depth, and a mode with no retained native
store, SHALL save as 8-bit as before.

#### Scenario: A 16-bit document preserves its depth on save

- **WHEN** a depth-16 Grayscale or RGB document is read and written unchanged
- **THEN** the output header declares bit depth 16, the composite and layer channels round-trip to the same 8-bit pixels, and an independent decoder reads the source-depth samples

#### Scenario: An edited plane keeps the source depth, widened

- **WHEN** a plane of a depth-16 document is edited to new 8-bit bytes and the document is written
- **THEN** the output still declares bit depth 16, the edited plane is the 8-bit bytes widened by `v * 257`, and reading it back narrows to those 8-bit bytes

#### Scenario: A depth-32 document preserves its depth

- **WHEN** a depth-32 Grayscale or RGB document is read and written unchanged
- **THEN** the output header declares bit depth 32 and an independent decoder reads the source-depth samples

#### Scenario: An 8-bit or constructed document is unchanged

- **WHEN** an 8-bit or constructed document is written
- **THEN** the output header declares bit depth 8 and the bytes are as before

#### Scenario: A normalized CMYK or Lab color mode preserves its depth

- **WHEN** a 16-bit CMYK or Lab document (whose planes the read path converts to the working RGB) is read and written unchanged
- **THEN** the output header declares bit depth 16 and header color mode CMYK (4) or Lab (9), and the retained native color planes round-trip byte-identically

#### Scenario: The psd-tools oracle reads the output depth and samples

- **WHEN** a written 16/32-bit file is decoded by `psd-tools`
- **THEN** the header depth and the native samples agree with the retained source-depth samples, and the oracle self-skips when `psd-tools` is unavailable

### Requirement: High-depth layer info is read from its tagged block

When the layer-info subsection is empty, `read_psd` SHALL read the document's
layers from the trailing tagged block keyed for the document depth: `Lr16` at
depth 16 and `Lr32` at depth 32, under either the `8BIM` or the `8B64` signature.
The contents SHALL be parsed as layer info, including group records. The parsed
block SHALL NOT remain in `Document.layer_section_extra`, and every other trailing
block SHALL stay verbatim. A block whose key does not match the document depth
SHALL be preserved unparsed. A malformed block SHALL fail the read.

#### Scenario: Grouped layers are read from a tagged block

- **WHEN** a depth-16 document stores a group of two layers in an `Lr16` block
- **THEN** the layer stack holds the group and its two layers

#### Scenario: The parsed block leaves the preserved blocks

- **WHEN** an `Lr16` block sits between two other trailing blocks
- **THEN** `layer_section_extra` holds exactly those two blocks

#### Scenario: A mismatched key stays preserved

- **WHEN** a depth-16 document carries an `Lr32` block and an empty layer-info subsection
- **THEN** the document has no layers and the block stays in `layer_section_extra`

#### Scenario: A PSB block with the 8B64 signature is read

- **WHEN** a depth-16 PSB stores its layers in an `Lr16` block signed `8B64` with an 8-byte length
- **THEN** the layer stack is read from it

#### Scenario: A malformed tagged block fails the read

- **WHEN** an `Lr16` block declares channel data past its end
- **THEN** `read_psd` returns an error and does not panic

### Requirement: High-depth layer info is written as a tagged block

When `write_psd` writes a document with layers at output depth 16 or 32, it SHALL
write a zero-length layer-info subsection and SHALL store the layers in one
`Lr16` or `Lr32` block ahead of the other trailing blocks. `read_psd` SHALL still
read layers stored in the subsection at those depths.

#### Scenario: An unchanged high-depth document saves in tagged layout

- **WHEN** a layered depth-16 or depth-32 document is written
- **THEN** the layer-info length is zero, the file has exactly one layer block, and reading it back returns an equal document

#### Scenario: A high-depth file with layers in the subsection still opens

- **WHEN** a depth-16 file stores its layers in the layer-info subsection
- **THEN** `read_psd` returns those layers

#### Scenario: An independent decoder reads tagged layers

- **WHEN** a tagged depth-16 fixture is read and written
- **THEN** `psd-tools` reports the same native composite planes and layer channels
