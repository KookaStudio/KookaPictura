# psd-layer-io Specification

## Purpose
Reads and writes the PSD layer-and-mask section, with raw and PackBits channels, luni names, and lsct group markers.

## Requirements

### Requirement: Read the PSD layer-and-mask section

`read_psd` SHALL parse the Layer and Mask Information section when present: the
outer section length, the layer info length, the signed layer count (its
magnitude is the record count), the layer records, then each record's channel
image data in layer order, then the global layer mask info. A zero-length layer
section SHALL produce an empty layer list.

#### Scenario: Layerless documents still parse

- **WHEN** a PSD has a zero-length layer-and-mask section
- **THEN** `read_psd` succeeds with `doc.layers` empty and a valid composite

#### Scenario: Oracle fixtures parse with the expected dimensions and mode

- **WHEN** each of `two_layers.psd`, `group.psd`, `masked.psd`, `gray.psd`,
  `adjustment.psd` is read
- **THEN** it yields an 8×8 document with the color mode the generator used

#### Scenario: Global layer mask info is consumed

- **WHEN** a layer section carries a global layer mask info block
- **THEN** the codec advances past it by its declared length and still parses the
  image data section

### Requirement: Channel image data supports raw and PackBits RLE
Layer channel image data SHALL support compression code `0` (raw), `1`
(PackBits/RLE), `2` (ZIP), and `3` (ZIP-with-prediction). RLE scanline
byte-count entries SHALL be 2 bytes in PSD and 4 bytes in PSB. RLE decoding
SHALL be lossless and SHALL reproduce the raw bytes exactly. ZIP is a
zlib-framed deflate stream (a raw-deflate stream SHALL also be accepted); for
code `3` the byte-wise per-scanline delta SHALL be inverted after inflating,
using the channel width as the scanline length. Any other compression code SHALL
return an unsupported error, and a malformed deflate stream SHALL return a typed
error without panicking.

#### Scenario: RLE decodes to the raw bytes
- **WHEN** a channel is encoded with PackBits literal and repeat runs
- **THEN** decoding yields the original byte sequence

#### Scenario: PSB uses four-byte scanline counts
- **WHEN** a PSB (`version == 2`) channel is RLE-decoded
- **THEN** the scanline count table is read as 4-byte entries and the row decodes correctly

#### Scenario: ZIP channel decodes
- **WHEN** a layer channel declares compression 2 and holds a deflate stream
- **THEN** the channel bytes are the inflated payload

#### Scenario: ZIP-with-prediction channel decodes
- **WHEN** a layer channel declares compression 3
- **THEN** the inflated bytes have the per-row delta inverted to recover the channel

#### Scenario: ZIP compression is rejected, not misread
- **WHEN** a layer channel declares an unknown compression code
- **THEN** `read_psd` returns `PsdError::Unsupported` and does not panic

### Requirement: luni Unicode layer names

The codec SHALL read a layer's Unicode name from the `'luni'` additional-layer
tagged block and SHALL prefer it over the legacy Pascal name. Writing SHALL emit
a `'luni'` block for each layer.

#### Scenario: Unicode names from the oracle are used

- **WHEN** `psd-tools`-authored `two_layers.psd` is read
- **THEN** the layers are named `Red` and `Blue` from their `'luni'` blocks

#### Scenario: Names round-trip through write/read

- **WHEN** a layer named with non-ASCII text is written and read back
- **THEN** the name is preserved

### Requirement: lsct group markers build the group tree

The codec SHALL read group structure from `'lsct'` section markers: a section
divider (type 3) bounds a group and folder records (types 1 and 2) close it, with
open and closed folders treated the same for M1. When the `'lsct'` block carries
a `'8BIM'` blend key, that key SHALL take precedence over the folder record's own
key, so a Pass Through group loads as `BlendMode::PassThrough`.

#### Scenario: Oracle group reads as a pass-through group

- **WHEN** `group.psd` is read
- **THEN** the top-level entry is a group named `Group A` with
  `blend == BlendMode::PassThrough` and two children

#### Scenario: A Pass Through group round-trips

- **WHEN** a document with a `PassThrough` group is written and read back
- **THEN** the group is still a group with `BlendMode::PassThrough` and its
  children are unchanged

### Requirement: Write the PSD layer-and-mask section

`write_psd` SHALL emit a layer-and-mask section for a document with pixel layers,
groups, and raster masks, such that read-write-read returns a `Document` equal to
the input. Groups SHALL be emitted as a section divider followed by folder
records, in depth-first bottom-first order, with a `'luni'` name and an `'lsct'`
marker per record. Raster mask data SHALL be written as a `-2` channel. A layer
whose lock flags are set SHALL emit a `'lspf'` block, a layer whose color label
is not `None` SHALL emit a `'lclr'` block, and a layer whose fill opacity is not
`255` SHALL emit an `'iOpa'` block; each block SHALL be omitted at its default.
A document that uses only default fill, no lock, and no color label MUST
serialize byte-for-byte identically to a build without these blocks, so
`write_psd`'s output for existing documents is unchanged.

#### Scenario: Layers, a group, and a mask round-trip

- **WHEN** a document with a pixel layer, a group of two children, and a masked
  layer is written and read back
- **THEN** the reconstructed `Document` equals the original, including names,
  blends, opacity, bounds, group children, and mask rect/disabled/data

#### Scenario: Grayscale layer round-trips

- **WHEN** a grayscale document with one pixel layer is written and read back
- **THEN** the reconstructed document equals the original

#### Scenario: Adjustment blocks are preserved opaquely

- **WHEN** a layer carries an unrecognized additional-layer adjustment block
- **THEN** the block's 4-byte key and payload are written back unchanged on
  round-trip, without the codec interpreting them

#### Scenario: Default attributes preserve the byte layout

- **WHEN** a document whose every layer has fill 255, no lock flags, and no
  color label is written
- **THEN** it emits no `'lspf'`, `'lclr'`, or `'iOpa'` block and the bytes are
  identical to the output before this change

### Requirement: Round-trip verified against the psd-tools oracle

The fixtures SHALL be authored by the independent `psd-tools` library, not by
the codec. Tests SHALL assert the codec reconstructs the expected tree from those
fixtures, and a PSD written by the codec SHALL open and parse in `psd-tools`.

#### Scenario: Two-layer tree agrees with the oracle

- **WHEN** `two_layers.psd` is read
- **THEN** its two layers, names, and bounds match the values `psd-tools` wrote

#### Scenario: Masked fixture exposes its mask

- **WHEN** `masked.psd` is read
- **THEN** one layer named `Masked` is returned with a raster layer mask

#### Scenario: psd-tools opens codec output

- **WHEN** `pictura-codec` writes a PSD carrying an extra channel and
  `psd-tools` opens it
- **THEN** `psd-tools` reports the bumped channel count and its composite alpha
  equals the extra plane

### Requirement: Malformed input errors, never panics

Malformed or truncated layer data SHALL return a `PsdError` and SHALL NOT panic,
index out of bounds, or loop forever. This includes a bad file signature, a
section whose declared length exceeds the file, an impossible layer count, a
bogus channel length, a blend signature that is not `8BIM`, and a tagged-block
signature that is neither `8BIM` nor `8B64`.

#### Scenario: Truncated file is an error

- **WHEN** a valid PSD is truncated inside or before the layer section
- **THEN** `read_psd` returns an error rather than panicking

#### Scenario: Bogus layer count is an error

- **WHEN** a layer record count claims more records than the section holds
- **THEN** `read_psd` returns an error

#### Scenario: Bogus channel length is an error

- **WHEN** a channel-info length field is set to an impossible value
- **THEN** `read_psd` returns an error

#### Scenario: Bad signatures are errors

- **WHEN** the file signature is not `8BPS`, the layer blend signature is not `8BIM`, or a tagged-block signature is neither `8BIM` nor `8B64`
- **THEN** `read_psd` returns the corresponding error

### Requirement: Read layer lock, color, and fill attributes

`read_psd` SHALL parse the `'lspf'` (protected setting), `'lclr'` (sheet color),
and `'iOpa'` (fill opacity) additional-layer-info blocks. `'lspf'` SHALL be read
as the low bits of its 4-byte value: transparency (`0x01`), image pixels
(`0x02`), and position (`0x04`); a value with all three bits sets Lock All.
`'lclr'` SHALL be read from its leading 2-byte color value, mapping the CS6
palette `None, Red, Orange, Yellow, Green, Blue, Violet, Gray` and treating any
unrecognized value as `None`. `'iOpa'` SHALL be read from the first byte of its
payload, independent of the block's declared length. An absent block SHALL mean
the default: no lock, no color, fill `255`. The parser SHALL NOT fail on a
length-variant or unrecognized payload; it SHALL take the default instead.

#### Scenario: A lock, color, and fill file reads back

- **WHEN** a PSD whose layer carries `'lspf'` position, `'lclr'` Red, and
  `'iOpa'` 128 is read
- **THEN** the layer reports the position lock, the Red color label, and fill 128

#### Scenario: Missing tags default

- **WHEN** a layer record has no `'lspf'`, `'lclr'`, or `'iOpa'` block
- **THEN** the layer reports no lock, no color label, and fill 255

#### Scenario: Attributes round-trip

- **WHEN** a document with non-default fill, lock, and color is written and read
  back
- **THEN** the reconstructed document equals the original

#### Scenario: psd-tools agrees with codec output

- **WHEN** the codec writes a document with fill 128, a lock, and a color label
  and `psd-tools` opens it
- **THEN** `psd-tools` reports the same `fill_opacity`, lock flags, and
  `sheet_color`

#### Scenario: Unknown color falls back

- **WHEN** a `'lclr'` payload holds a value outside the CS6 palette
- **THEN** the layer reports no color label and reading does not error

### Requirement: Unknown blend keys degrade to Normal
A layer whose 4-byte blend key is not one of the 27 modes or `pass` SHALL be
read with `BlendMode::Normal` instead of failing the whole document. A blend
signature that is not `8BIM` SHALL still be an error. The original key is not
preserved by this change; opaque preservation is a later roadmap phase.

#### Scenario: Unknown key does not abort the file
- **WHEN** a layer record carries an unrecognized blend key such as `zzzz`
- **THEN** `read_psd` succeeds and that layer's blend is `BlendMode::Normal`

#### Scenario: Bad blend signature still errors
- **WHEN** a layer record's blend signature is not `8BIM`
- **THEN** `read_psd` returns a typed error

### Requirement: Opaque layer blocks and channels are preserved
The codec SHALL retain, and write back byte-for-byte, the layer-and-mask data
the engine does not model: each layer's original 4-byte blend key when it is not
a recognized mode, each layer's layer-blending-ranges bytes, each layer's
additional-layer-info tagged blocks whose keys are not modeled, each layer mask's
bytes beyond the fixed rectangle/default/flags fields, the global layer mask
block, the trailing global additional-layer information, and each layer channel
whose id is not a modeled color/transparency/mask id (for example the `-3` real
user mask), stored with its compression header intact. A recognized blend key
SHALL NOT be stored, so a document constructed in memory still equals one read
from disk after a write/read round trip.

#### Scenario: Unknown tagged blocks survive
- **WHEN** a layer carries additional-layer-info blocks the codec does not model
- **THEN** read→write→read preserves those blocks' keys and payloads exactly

#### Scenario: An unmodeled channel survives
- **WHEN** a layer carries a channel with an id outside the modeled set, such as `-3`
- **THEN** read→write→read preserves that channel's bytes

#### Scenario: An unknown blend key survives
- **WHEN** a layer's blend key is not one of the 27 modes or `pass`
- **THEN** read→write→read preserves the original 4-byte key

#### Scenario: A recognized blend key is not stored
- **WHEN** a layer uses a recognized blend mode and the document is written and read
- **THEN** the reconstructed document equals the original (no spurious preserved key)

#### Scenario: Blending ranges and mask extras survive
- **WHEN** a layer carries non-empty blending ranges or a mask block longer than the fixed fields
- **THEN** read→write→read preserves those bytes

### Requirement: Decode the `vmsk` vector mask into a derived layer view

`read_psd` SHALL decode a layer's `'vmsk'` additional-layer-info block into a
derived `Layer.vector_mask` view. The view SHALL carry the mask's `invert` and
`disabled` flags and its subpaths; each subpath SHALL carry whether it is closed,
its `operation`, its fill rule, and its flattened document-pixel points. The
view SHALL be derived only: the `'vmsk'` block SHALL remain in
`Layer.extra_blocks` and SHALL be written back verbatim, and the derived view
SHALL NOT be serialized, so an open→save round trip leaves the block
byte-identical.

The block SHALL begin with a big-endian `u32` version that MUST equal `3` and a
big-endian `u32` flags field whose bit 0 is `invert`, bit 1 is `not_link`, and
bit 2 is `disable`. The remainder SHALL be a sequence of records consumed while
at least 26 bytes remain, each beginning with a big-endian `u16` selector:

- selector `0` (closed subpath) or `3` (open subpath): a big-endian `u16` knot
  count, an `i16` operation, a big-endian `u16` fill-rule field, and 18 unused
  bytes, immediately followed by that many knot records;
- selector `1`, `2`, `4`, or `5` (knot): six big-endian `i32` 8.24 fixed-point
  values in the order preceding `(y, x)`, anchor `(y, x)`, leaving `(y, x)`;
- selector `6` (path fill rule): 24 unused bytes;
- selector `7` (clipboard): 24 unused bytes;
- selector `8` (initial fill): a big-endian `u16` value and 22 unused bytes.

A block whose version is not `3`, a record selector outside that set, a knot
record whose subpath is missing, or a truncated record SHALL leave
`Layer.vector_mask` unset and SHALL NOT return an error or panic; the preserved
block SHALL remain intact.

Fixed-point coordinates SHALL be scaled to document pixels as
`raw / 0x01000000 × document width` for x and
`raw / 0x01000000 × document height` for y, and each cubic segment SHALL be
flattened to a polyline. A subpath's fill rule SHALL be `even-odd` unless its
fill-rule field equals `2`, which SHALL be `non-zero`.

#### Scenario: The psd-tools rectangle fixture decodes to a closed subpath

- **WHEN** `vector_mask.psd` is read
- **THEN** the `Shape` layer has `vector_mask` set with the `invert` and
  `disabled` flags clear and one closed subpath whose flattened points span the
  authored rectangle in document pixels

#### Scenario: The invert flag is read from flags bit 0

- **WHEN** a layer's `vmsk` flags field has bit 0 set
- **THEN** its derived `vector_mask.invert` is true

#### Scenario: The disable flag is read from flags bit 2

- **WHEN** a layer's `vmsk` flags field has bit 2 set
- **THEN** its derived `vector_mask.disabled` is true

#### Scenario: The fill-rule field defaults to even-odd

- **WHEN** a closed subpath's fill-rule field is `0` or `1`
- **THEN** the subpath's fill rule is `even-odd`

#### Scenario: The ag-psd non-zero marker promotes the fill rule

- **WHEN** a closed subpath's fill-rule field is `2`
- **THEN** the subpath's fill rule is `non-zero`

#### Scenario: A malformed vector mask is unset and never panics

- **WHEN** a `vmsk` block has a version other than 3, an unknown selector, a knot
  without its subpath, or is truncated
- **THEN** `read_psd` succeeds, the layer's `vector_mask` is unset, and the raw
  block is still preserved

#### Scenario: The derived view does not change the serialized block

- **WHEN** a document carrying a `vmsk` block is written and read back
- **THEN** the block's bytes are preserved and the reconstructed document equals
  the input

### Requirement: Vector mask fixtures match independent decoders

The codec SHALL commit a `crates/pictura-codec/tests/fixtures/vector_mask.psd`
fixture carrying a closed-rectangle `vmsk` on a solid-fill shape layer and a
second solid-fill layer whose `vmsk` has the invert flag set, authored with
`psd-tools` and regenerated by `scripts/generate-fixtures.py`. A test SHALL read the fixture
with `psd-tools` and assert the block's version, flags, and closed-subpath
geometry, and a test SHALL read it with the independent `ag-psd` npm package
through `node` and assert the `invert` flag, the closed subpath, and its pixel
knots. The ag-psd test SHALL self-skip with a clear message when `node` or
`ag-psd` is unavailable and SHALL NOT fail the suite in that case.

#### Scenario: psd-tools reads the fixture geometry

- **WHEN** the fixture is parsed with `psd-tools`
- **THEN** each shape layer's vector mask reports version 3, the authored flags,
  and one closed subpath with the authored knots

#### Scenario: ag-psd reads the fixture geometry

- **WHEN** the fixture is read by ag-psd
- **THEN** the shape layer reports one closed path covering the authored
  rectangle, and the second layer reports `invert` true

#### Scenario: The ag-psd oracle self-skips without node

- **WHEN** `node` or the `ag-psd` package is not available
- **THEN** the test reports a skip and the suite passes

### Requirement: Vector fill fixtures match independent decoders

The codec SHALL commit a `crates/pictura-codec/tests/fixtures/vector_fill.psd`
fixture carrying a `'vscg'` solid-color vector fill on a shape layer and a
closed-rectangle `'vmsk'` clipping it, authored with `psd-tools` and regenerated
byte-stably by `scripts/generate-fixtures.py`. Existing fixtures SHALL be
unchanged. A test SHALL read the fixture with `psd-tools` and assert the
`'vscg'` block's 4-byte key, version, and decoded `Clr ` colour, and that a
whole-`Document` round trip preserves the block's bytes. A test SHALL read the
fixture with the independent `ag-psd` npm package through `node` and assert the
shape layer's `vectorFill` decodes to the authored solid colour. The ag-psd test
SHALL self-skip with a clear message when `node` or the `ag-psd` package is
unavailable and SHALL NOT fail the suite in that case.

#### Scenario: psd-tools reads the `vscg` fixture block

- **WHEN** `vector_fill.psd` is parsed with `psd-tools`
- **THEN** its shape layer carries a `vscg` block with the authored key and
  version and a `Clr ` object carrying the authored colour

#### Scenario: ag-psd reads the `vscg` fixture block

- **WHEN** `vector_fill.psd` is read by ag-psd
- **THEN** the shape layer's `vectorFill` is a color fill with the authored
  components

#### Scenario: The `vscg` block survives the document round trip

- **WHEN** `vector_fill.psd` is read and written by the codec
- **THEN** the reconstructed document equals the input and the `vscg` block is
  byte-identical

#### Scenario: The ag-psd oracle self-skips without node

- **WHEN** `node` or the `ag-psd` package is not available
- **THEN** the test reports a skip and the suite passes

### Requirement: A per-layer `8B64` tagged block is accepted

`read_psd` SHALL read a per-layer additional-layer-information block whose
signature is `8B64` exactly as it reads an `8BIM` block, preserving its key and
payload in the layer's opaque blocks, so a file carrying one opens rather than
erroring. The block's signature is not modeled and SHALL be written back as
`8BIM`, matching the document-level block behavior.

#### Scenario: An `8B64` block reads

- **WHEN** a layer record carries a tagged block with the `8B64` signature
- **THEN** `read_psd` succeeds and the block's key and payload appear in that layer's opaque blocks

#### Scenario: An unknown signature still errors

- **WHEN** a tagged-block signature is neither `8BIM` nor `8B64`
- **THEN** `read_psd` returns a typed error

### Requirement: Raster mask link state survives PSD read and save

PSD and PSB raster-mask flags SHALL be preserved in their on-disk
representation: bit 0 clear means linked, and bit 0 set means unlinked. Reading
or saving either state SHALL preserve the mask's document-space rectangle and
coverage without adding or subtracting the owning layer's offset. An unchanged
read-save-read SHALL preserve the link bit and the other mask-flag bits. A link
toggle followed by save and read SHALL change only bit 0 of the mask flags,
leaving other mask attributes unchanged. Independent decoder checks SHALL
verify the written bit values for both states.

#### Scenario: A linked external mask remains linked [pml_codec_linked]

- **WHEN** an independently authored PSD with mask bit 0 clear and different non-zero layer and mask origins is read, saved unchanged, and reopened
- **THEN** its mask reads as linked before and after saving, with the same flags, document-space rectangle, and coverage

#### Scenario: An unlinked external mask remains unlinked [pml_codec_unlinked]

- **WHEN** an independently authored PSD with mask bit 0 set and different non-zero layer and mask origins is read, saved unchanged, and reopened
- **THEN** its mask reads as unlinked before and after saving, with the same flags, document-space rectangle, and coverage

#### Scenario: Link edits persist in both document formats [pml_codec_toggle]

- **WHEN** a raster mask is linked and unlinked in turn, saving and reopening as PSD and PSB after each edit
- **THEN** each reopened state matches the edit, only bit 0 of its flags changes between states, and its other attributes are preserved

#### Scenario: An independent decoder reads the saved flags [pml_codec_oracle]

- **WHEN** files saved with linked and unlinked raster masks are opened in an independent PSD decoder
- **THEN** their raw mask bit 0 values are clear and set respectively, and the decoded mask rectangles and coverage agree with the saved masks

### Requirement: Layer record pixel-data-irrelevant flag round-trips

The codec SHALL read bit 4 (`0x10`, "pixel data irrelevant to the appearance of
the document") of each modeled PSD and PSB layer record's flags into the layer
model and SHALL write that stored value back into the record. A section-divider
record is not a modeled layer and SHALL continue to be written with flags `0`.
When bit 4 is written, the record SHALL also set bit 3 (`0x08`, "flags valid")
so Photoshop treats bit 4 as meaningful; the writer SHALL leave bit 3 clear for
a layer whose stored bit 4 is clear, so documents whose layers never carry bit 4
serialize byte-for-byte as before. An unchanged read-save-read SHALL preserve
bit 4 for both set and clear values and SHALL leave the layer's other modeled
attributes unchanged; the remaining flag bits keep the writer's current behavior
(bit 3 is not preserved on layers without bit 4). The codec SHALL NOT infer the
bit from the layer's vector blocks: a layer read with bit 4 clear SHALL be
written with bit 4 clear even when it carries a vector mask, vector fill,
stroke, or origination block. The flag SHALL be absent at its default so
documents whose layers never carry it serialize byte-for-byte as before. An
independent PSD decoder SHALL classify a saved layer that carries bits 3 and 4
and shape vector blocks as a shape layer, and a vector-masked fill layer whose
bit is clear as a fill layer.

#### Scenario: Reading records the flag for both values [slf_read]

- **WHEN** `vector_fill.psd` is read
- **THEN** its `Shape` layer has the flag set and its `Base` layer has the flag clear

#### Scenario: Saving preserves a set flag in PSD and PSB [slf_round_trip]

- **WHEN** the document read from `vector_fill.psd` is written and read back as PSD and as PSB
- **THEN** the PSD reconstruction equals the input, the PSB reconstruction equals the input once the expected container flag is normalized, and its `Shape` layer still has the flag set

#### Scenario: A clear flag is not inferred from vector blocks [slf_clear_stays_clear]

- **WHEN** `vector_mask.psd`, whose vector-masked fill layers carry bit 4 clear, is read, written, and read back as PSD and as PSB
- **THEN** those layers still have the flag clear

#### Scenario: An independent decoder still sees a shape [slf_oracle_shape]

- **WHEN** the written copy of `vector_fill.psd` is opened in `psd-tools`
- **THEN** `Shape` is reported as a `shape` layer, `Base` as a `pixel` layer, and the shape record's raw flags are `0x18` (bits 3 and 4 set)

#### Scenario: An independent decoder still sees fill layers [slf_oracle_fill]

- **WHEN** the written copy of `vector_mask.psd` is opened in `psd-tools`
- **THEN** its vector-masked solid-color layers are still reported with the exact kind `solidcolorfill`

#### Scenario: Documents without the flag serialize unchanged [slf_default_bytes]

- **WHEN** a document whose layers all have bit 4 clear is written
- **THEN** its bytes are identical to the existing golden output

### Requirement: Authored shape layers carry the flag

A shape layer created by the shape tools as a solid-color fill clipped by an
authored vector mask SHALL be written with bits 3 and 4 (`0x18`) set. An
independent PSD decoder SHALL classify the saved layer as a shape layer. Other
fill layers and raster layers with vector masks SHALL NOT gain the flag.

#### Scenario: A drawn shape opens as a shape [slf_authored]

- **WHEN** a rectangle is drawn in Shape mode and the document is saved
- **THEN** `psd-tools` reports the layer as a `shape` layer and its record flags are `0x18`

#### Scenario: Ordinary fills and vector-masked pixels do not gain the flag [slf_no_gain]

- **WHEN** an ordinary solid-color fill layer created with `add_solid_fill` and a pixel layer carrying a vector mask with no fill adjustment are written and read back
- **THEN** both layers keep the pixel-data-irrelevant flag clear
