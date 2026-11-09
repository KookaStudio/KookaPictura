# Spec Delta

## MODIFIED Requirements

### Requirement: The document pattern library is decoded from the Patt resource

`pictura-codec` SHALL expose `decode_patterns(&Document) -> Vec<PatternPixels>`
that decodes the document's Patterns resource into row-major RGBA pattern
pixels carrying each pattern's id, width, and height. The resource SHALL be read
from the `Patt`, `Pat2`, and `Pat3` additional-layer-information tagged blocks
in the Layer and Mask Information section (preserved in
`Document.layer_section_extra`), NOT from `Document.image_resources` (image
resource 1039 is the ICC profile). Each block SHALL be parsed as a `Patterns`
list of length-prefixed version-1 patterns: image mode, width/height point,
Unicode name, Pascal pattern id, and a `VirtualMemoryArrayList` whose declared
`num_channels` is the number of leading colour slots and which is followed by
two alpha slots.

Photoshop writes 24 colour slots plus two alpha slots for every pattern,
regardless of the image mode, and writes only the planes the pattern uses; the
decoder SHALL NOT require `num_channels` to equal the image mode's colour-plane
count. It SHALL require at least as many colour slots as the mode's colour-plane
count and SHALL skip a pattern declaring more than 24 colour slots. The first
`num_channels` slots are the colour region and the last two are the alpha
region. The leading written colour slots in that region SHALL be the pattern
colour (the first written channel replicated for a grayscale pattern and the
first three taken as R, G, B for an RGB pattern); written colour slots beyond
the mode's colour-plane count SHALL be ignored without decoding. A written
channel in the alpha region SHALL be the pattern's transparency with 255 when
absent, and the last written alpha slot SHALL win. Only 8-bit planes SHALL be
decoded: a written colour or alpha channel whose declared depth or pixel depth
is not 8 SHALL cause the pattern to be skipped. An RGB pattern SHALL have
exactly three written leading colour planes and a grayscale pattern exactly
one; a pattern with a missing leading colour plane, a colour-slot count below
the mode's colour-plane count or above 24, or whose declared rectangle disagrees
with its written channel rectangles or exceeds a bounded pixel cap, SHALL be
skipped. A malformed or unrecognised block SHALL be skipped, keeping the
patterns parsed so far, and the decoder SHALL NOT panic or allocate without
bound.

#### Scenario: A Patt block decodes to pattern pixels

- **WHEN** a document carries a `Patt` tagged block with a version-1 RGB pattern whose channels hold known 8-bit planes
- **THEN** `decode_patterns` returns one `PatternPixels` with that pattern's id, size, and RGBA pixels in row-major order

#### Scenario: A Photoshop 24-slot pattern decodes its written planes [pat_photoshop_slots]

- **WHEN** a `Patt` pattern declares Photoshop's 24 colour slots, writes its RGB planes in the first three slots and its transparency in the last slot, and leaves the other colour slots unwritten
- **THEN** `decode_patterns` returns the pattern with the three written colour planes as R, G, B and the last written alpha slot as transparency, rather than skipping it

#### Scenario: A colour-slot count above Photoshop's layout is skipped [pat_slot_count_cap]

- **WHEN** a pattern declares more than 24 colour slots
- **THEN** `decode_patterns` skips it without decoding an unbounded number of channels and does not panic

#### Scenario: A grayscale pattern replicates its single channel

- **WHEN** a pattern's image mode is Grayscale with one written colour channel
- **THEN** the decoded pixel's R, G, and B components all equal that channel

#### Scenario: A malformed Patt block is skipped

- **WHEN** a `Patt` block is truncated or one pattern's channel list is malformed
- **THEN** `decode_patterns` returns the patterns parsed before the failure and does not panic

#### Scenario: Non-8-bit pattern planes are skipped

- **WHEN** a `Patt` block's written channel declares a depth or pixel depth other than 8
- **THEN** `decode_patterns` skips that pattern and returns the patterns parsed before it, without decoding the plane or panicking

#### Scenario: A crafted oversized pattern rectangle is skipped

- **WHEN** a pattern's declared rectangle is inconsistent with its written channel rectangles or exceeds the decoder's pixel cap
- **THEN** `decode_patterns` skips the pattern without an unbounded allocation and does not panic

#### Scenario: The trailing alpha slot is the pattern transparency

- **WHEN** both slots of a pattern's two-slot alpha region are written
- **THEN** the decoded alpha is the last written slot, matching psd-tools' trailing-plane rule

#### Scenario: A pattern with a missing colour plane is skipped

- **WHEN** an RGB pattern does not have exactly three written leading colour planes, or a grayscale pattern does not have exactly one
- **THEN** `decode_patterns` skips it rather than filling the absent channel with zero
