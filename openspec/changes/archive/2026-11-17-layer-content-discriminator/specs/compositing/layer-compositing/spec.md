# Spec Delta

## MODIFIED Requirements

### Requirement: Decode a preserved `vscg` block into typed fill content

The compositor SHALL decode a layer's preserved `'vscg'` additional-layer-info
block into the matching `FillContent` variant when the layer has
no modeled adjustment block. The block SHALL be a big-endian 4-byte fill key
followed by a big-endian `u32` version that MUST equal `16` and a descriptor. The
descriptor SHALL be decoded by the same decoders the top-level `'SoCo'`,
`'GdFl'`, and `'PtFl'` blocks use, dispatching on its content: a `Grad` object
SHALL decode to `FillContent::Gradient`, a `Ptrn` object SHALL decode to
`FillContent::Pattern`, and a `Clr ` object SHALL decode to
`FillContent::Solid`. The 4-byte key SHALL be consumed but SHALL NOT be the
dispatch authority, so a block whose key disagrees with its descriptor content
still decodes by content.

A missing key, a version other than `16`, a non-object descriptor, a descriptor
with none of those contents, or a malformed descriptor SHALL decode to `None`,
SHALL NOT change the backdrop, and SHALL NOT panic. The preserved `'vscg'` block
SHALL remain in `Layer.extra_blocks` and SHALL be re-emitted verbatim; the decode
SHALL be derived only.

#### Scenario: A `vscg` solid fill decodes to `SolidFill`

- **WHEN** a layer carries a `vscg` block whose descriptor holds a `Clr ` object
  with `Rd `/`Grn `/`Bl  ` doubles
- **THEN** the decoded fill is `FillContent::Solid` with those components and
  alpha 255

#### Scenario: A `vscg` gradient fill reuses the gradient decoder

- **WHEN** a layer carries a `vscg` block whose descriptor holds `Angl`, a `Type`
  enum, and a custom-stop `Grad` object
- **THEN** the decoded fill is `FillContent::Gradient` with the same kind,
  stops, and angle a top-level `GdFl` block would yield

#### Scenario: A malformed `vscg` block is a no-op

- **WHEN** a `vscg` block has a version other than 16, a non-object descriptor,
  or a descriptor with none of `Grad`/`Ptrn`/`Clr `
- **THEN** the decode yields `None` and does not panic

#### Scenario: The decoded block is not serialized

- **WHEN** a document carrying a `vscg` block is written and read back
- **THEN** the block's bytes are preserved and the reconstructed document equals
  the input
