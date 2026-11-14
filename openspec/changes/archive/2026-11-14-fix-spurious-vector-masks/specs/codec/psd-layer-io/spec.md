# Spec Delta

## MODIFIED Requirements

### Requirement: Decode the `vmsk` vector mask into a derived layer view

`read_psd` SHALL decode a layer's `'vmsk'` additional-layer-info block into a
derived `Layer.vector_mask` view, except when the block is a rasterized
vector-mask source path as defined below. The view SHALL carry the mask's
`invert` and `disabled` flags and its subpaths; each subpath SHALL carry whether
it is closed, its `operation`, its fill rule, and its flattened document-pixel
points. The view SHALL be derived only: the `'vmsk'` block SHALL remain in
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

A layer's `'vmsk'` block SHALL NOT be decoded into `Layer.vector_mask` when all
of the following hold:

- the layer record's bit 4 ("pixel data irrelevant to the appearance of the
  document") is clear, so the layer's appearance comes from its pixels;
- the layer's raster mask record has mask flag bit 3 (`0x08`, "the user mask
  actually came from rendering other data") set;
- the layer's channel list carries no `-3` real-user-mask channel.

Such a block is the source path that was rendered into the raster mask, not a
live vector mask. The layer's raster `Layer.mask` SHALL be unchanged, the
`'vmsk'` block SHALL remain in `Layer.extra_blocks`, and an open→save round trip
SHALL re-emit the block verbatim. A layer whose record bit 4 is set SHALL keep
the derived view, because its block may be the layer's content (a shape
outline). A layer carrying a `-3` channel, or a raster mask whose bit 3 is
clear, SHALL keep the derived view because a live raster and vector mask
composite together.

When the view is suppressed, the mask record's vector density and feather
fields SHALL NOT be required to round-trip; the writer normalizes them to their
defaults, while the raster mask's user density and feather SHALL round-trip
unchanged.

A pixel layer whose model carries a live vector mask SHALL be written with its
raster mask's render flag (`0x08`) clear when no `-3` channel is present, so a
save→read keeps the derived view. A suppressed source path SHALL keep the flag.
This covers a mask authored after import and a live mask whose `-3` channel was
dropped by an edit.

The codec SHALL commit a `derived_raster_mask.psd` fixture carrying a pixel
layer with record bit 4 clear, a rendered raster mask whose flags have bit 3
set, a mask-parameters block with non-default user density and vector
parameters, a parseable `vmsk`, and no `-3` channel.

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

#### Scenario: A rasterized source path is not a live vector mask [vmk_derived_source]

- **WHEN** a pixel layer with record bit 4 clear carries a parseable `vmsk` and a
  raster mask whose mask flags have bit 3 set, and carries no `-3` channel
- **THEN** `read_psd` succeeds with the layer's raster mask present and its
  `vector_mask` unset

#### Scenario: A shape layer keeps its outline [vmk_derived_shape]

- **WHEN** a layer with record bit 4 set carries the same parseable `vmsk` and a
  raster mask whose mask flags have bit 3 set
- **THEN** its derived `vector_mask` is present

#### Scenario: A real-user-mask composite keeps the view [vmk_derived_composite]

- **WHEN** a pixel layer carries the same `vmsk` and rendered raster mask and
  also carries a `-3` real-user-mask channel
- **THEN** its derived `vector_mask` is present

#### Scenario: An ordinary raster mask keeps the view [vmk_derived_raster]

- **WHEN** a pixel layer carries a `vmsk` and a raster mask whose flags have
  bit 3 clear
- **THEN** its derived `vector_mask` is present

#### Scenario: The suppressed source path round-trips [vmk_derived_round_trip]

- **WHEN** a document read from the rasterized-source fixture is written and
  read back
- **THEN** the layer again has no vector mask, its raster mask is unchanged, and
  the raw `vmsk` block is preserved byte-for-byte

#### Scenario: The fixture carries the source-path facts [vmk_derived_fixture]

- **WHEN** the committed rasterized-source fixture is parsed with `psd-tools`
- **THEN** its layer carries a `vmsk` block, a layer mask whose
  `user_mask_from_render` flag is set, and non-default density/feather parameters

#### Scenario: An authored live mask survives a save [vmk_derived_authored]

- **WHEN** a pixel layer imported with a rendered source path receives a live
  vector mask and the document is written and read back
- **THEN** the layer's vector mask is present and its raster mask's render flag
  is clear
