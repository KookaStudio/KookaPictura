# psd-advanced-blending Specification

## Purpose
Advanced-blending blocks, layer blending ranges, and Blend If gating in the codec and compositor.

## Requirements

### Requirement: Advanced-blending tagged blocks decode into layer fields

The system SHALL parse a layer's `knko`, `clbl`, `infx`, `tsly`, `lmgm`,
`vmgm`, and `brst` additional-layer-info blocks into first-class layer fields
when present. `knko` SHALL decode into a knockout mode (`None`, `Shallow`, or
`Deep` from bytes 0, 1, or 2; any other byte SHALL yield `None`). The boolean
blocks SHALL decode with byte 1 meaning on and any other byte meaning off:
`clbl` into blend-clipping, `infx` into blend-interior, `tsly` into
transparency-shapes, `lmgm` into layer-mask-hides-effects, and `vmgm` into
vector-mask-hides-effects. `brst` SHALL decode into an ordered list of
big-endian `u32` restricted channel numbers, preserving file order. When a
block is absent, the fields SHALL default to knockout `None`, blend-clipping
`true`, blend-interior `false`, transparency-shapes `true`, both mask-hide flags
`false`, and an empty restricted-channel list. A boolean block with an empty
payload SHALL leave its field at the default. An empty `brst` payload SHALL
leave the list empty. A `brst` payload whose length is not a multiple of four
SHALL leave the list empty and MUST NOT fail the document read; the raw block
SHALL be retained in `extra_blocks`. These keys SHALL NOT otherwise be retained
in the opaque `extra_blocks` list after a successful read.

#### Scenario: A deep-knockout block decodes

- **WHEN** a layer carries `knko` with payload byte 0 equal to 2
- **THEN** the layer's knockout mode is `Deep` and `extra_blocks` does not contain a `knko` entry

#### Scenario: Absent blocks use CS6 defaults

- **WHEN** a layer has no `knko`, `clbl`, `infx`, `tsly`, `lmgm`, `vmgm`, or `brst` blocks
- **THEN** knockout is `None`, blend-clipping is `true`, blend-interior is `false`, transparency-shapes is `true`, both mask-hide flags are `false`, and the restricted-channel list is empty

#### Scenario: Empty knko payload does not fail the document

- **WHEN** a layer carries `knko` with zero-length data
- **THEN** document read succeeds and knockout remains `None`

#### Scenario: Empty boolean payloads keep the defaults [cbl_empty_bool]

- **WHEN** a layer carries zero-length `clbl`, `infx`, `tsly`, `lmgm`, and `vmgm` blocks
- **THEN** each field keeps its default value and the document read succeeds

#### Scenario: infx byte 1 turns the group option on [cbl_infx_on]

- **WHEN** a layer carries `infx` with payload byte 0 equal to 1
- **THEN** the layer's blend-interior flag is `true` and the block is not retained in `extra_blocks`

#### Scenario: A tsly zero byte turns transparency shaping off [cbl_tsly_off]

- **WHEN** a layer carries `tsly` with payload byte 0 equal to 0
- **THEN** the layer's transparency-shapes flag is `false`

#### Scenario: The mask-hide blocks decode independently [cbl_lmgm_vmgm]

- **WHEN** a layer carries `lmgm` with byte 1 and no `vmgm`
- **THEN** layer-mask-hides-effects is `true` and vector-mask-hides-effects is `false`

#### Scenario: Channel restrictions decode in file order [cbl_brst]

- **WHEN** a layer carries `brst` with big-endian values 2 and 0
- **THEN** the restricted-channel list is `[2, 0]` and the block is not retained in `extra_blocks`

### Requirement: Advanced-blending fields write back only when non-default

On write, the system SHALL emit `knko` only when knockout is not `None`, `clbl`
only when blend-clipping is `false`, `infx` only when blend-interior is `true`,
`tsly` only when transparency-shapes is `false`, `lmgm` only when
layer-mask-hides-effects is `true`, `vmgm` only when vector-mask-hides-effects is
`true`, and `brst` only when the restricted-channel list is non-empty. Each
boolean tag's payload SHALL be four bytes: the value byte (1 for a true flag, 0
for a false flag) followed by three padding zeros. The `brst` payload SHALL be
the restricted channel numbers as big-endian `u32` values in ascending order,
with no count prefix. A document whose fields are all at their defaults SHALL
not contain those keys after a write.

#### Scenario: Deep knockout round-trips

- **WHEN** a document with knockout `Deep` is written and read back
- **THEN** the written file contains a `knko` block with value 2 and the layer's knockout is `Deep`

#### Scenario: Default fields are omitted

- **WHEN** every layer is at the default advanced-blending values and the document is written
- **THEN** the output contains no `knko`, `clbl`, `infx`, `tsly`, `lmgm`, `vmgm`, or `brst` keys

#### Scenario: Blend Interior is emitted only when on [cbl_infx_write]

- **WHEN** a layer with blend-interior `true` is written and read back
- **THEN** the output contains `infx` with value 1 and the layer's blend-interior is `true`

#### Scenario: Non-default mask and transparency flags round-trip [cbl_flags_roundtrip]

- **WHEN** a document sets transparency-shapes `false`, layer-mask-hides-effects `true`, and vector-mask-hides-effects `true`
- **THEN** the `tsly`, `lmgm`, and `vmgm` blocks are present with values 0, 1, and 1, and every field reads back unchanged

#### Scenario: Channel restrictions write in ascending order [cbl_brst_write]

- **WHEN** a layer's restricted-channel list is `[2, 0, 2]` and the document is written and read back
- **THEN** the file contains one `brst` block with values 0 and 2 and the read list is `[0, 2]`

### Requirement: Layer blending ranges decode into a typed Blend If view

The system SHALL parse a layer's non-empty blending-ranges payload into a typed Blend If view: composite gray source and destination ranges as two `(u16, u16)` pairs each, then zero or more channel groups of source and destination ranges in the same layout, all big-endian. An empty payload SHALL leave the typed view `None` while still round-tripping the raw field. A payload whose length is not a multiple of eight bytes, or that is otherwise truncated, SHALL leave the view `None` and MUST NOT fail the document read. The raw `blending_ranges` bytes SHALL remain the serialization source for an unmodified open→save.

#### Scenario: Non-empty ranges decode

- **WHEN** a layer carries a blending-ranges body with a composite source pair and at least one channel group
- **THEN** the typed view exposes those `u16` ranges and the raw field is unchanged

#### Scenario: Empty ranges keep the raw field

- **WHEN** a layer's blending-ranges length is zero
- **THEN** document write emits an empty blending-ranges field and read succeeds

#### Scenario: Truncated ranges leave the view unset

- **WHEN** the blending-ranges body has a length that is not a valid multiple of range groups
- **THEN** document read succeeds, the typed view is `None`, and the raw bytes still round-trip on write

### Requirement: encode_blend_if rebuilds a well-formed payload

`pictura-codec` SHALL expose `encode_blend_if(&BlendIf) -> Vec<u8>` that emits the composite source pair, composite destination pair, and each channel group as big-endian `u16` pairs. Parsing the encoded bytes SHALL recover the same ranges.

#### Scenario: Encode then parse preserves ranges

- **WHEN** `encode_blend_if` output is parsed back as a blending-ranges body
- **THEN** the composite and channel ranges equal the input view

### Requirement: Blend If ranges gate the CPU compositor

The CPU compositor SHALL apply a layer's typed `BlendIf` view as a per-pixel
gate on that layer's blend weight. The composite source range SHALL gate on the
source pixel's gray and the composite destination range on the running
backdrop's gray; each per-channel group SHALL gate on that channel's source and
backdrop values. A range at the full default (`0, 65535`) SHALL be inactive. A
range SHALL contribute factor `0` when the gated value is at or below its black
endpoint or at or above its white endpoint, and `1` otherwise; the layer's
weight SHALL be multiplied by the product of the active ranges. A layer whose
typed view is absent or at the full default SHALL composite byte-identically to
before.

#### Scenario: Default ranges are a no-op

- **WHEN** a layer's blending ranges are the full default or the typed view is absent
- **THEN** the composited output is byte-identical to the same stack without the ranges

#### Scenario: A source range hides the layer below its black point

- **WHEN** a layer's composite source range has a black endpoint above a source pixel's gray
- **THEN** that layer does not contribute to that pixel

#### Scenario: A destination range gates on the backdrop

- **WHEN** a layer's composite destination range excludes the backdrop's gray at a pixel
- **THEN** that layer does not contribute to that pixel

#### Scenario: A per-channel range gates that channel

- **WHEN** a layer carries a per-channel range whose black endpoint exceeds that channel's source value
- **THEN** that layer does not contribute to that pixel

### Requirement: The GPU compositor declines a non-default Blend If layer

The GPU compositor SHALL report an advanced-blending error for a stack containing
a layer, at any depth, whose typed `BlendIf` view is present and not at the full
default, so the caller composites on the CPU. A stack with no such layer SHALL
continue to use the GPU path.

#### Scenario: A Blend If layer forces the CPU oracle

- **WHEN** a document contains a layer with a non-default Blend If range
- **THEN** the GPU compositor declines and the CPU composite is the result

#### Scenario: No Blend If keeps the GPU path

- **WHEN** no layer carries a non-default Blend If range
- **THEN** the GPU compositor is attempted as before

### Requirement: Malformed and unknown channel restrictions stay lossless

A restriction block with an unusable payload SHALL be preserved for re-save
rather than dropped or interpreted. Unknown channel numbers in an otherwise
valid `brst` payload SHALL remain in the model and SHALL be re-emitted on write,
even when the compositor does not render them.

#### Scenario: An unknown channel index round-trips [cbl_brst_unknown]

- **WHEN** a layer carries `brst` with big-endian values 0 and 7
- **THEN** the restricted-channel list is `[0, 7]` and a written file re-emits both values

#### Scenario: A malformed payload stays opaque [cbl_brst_malformed]

- **WHEN** a layer carries a two-byte `brst` payload
- **THEN** document read succeeds, the restricted-channel list is empty, and the original two-byte payload is re-emitted unchanged
