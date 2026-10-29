# document-model Specification

## Purpose
The Document and its bottom-first layer tree, the M1 layer property set, and the BlendMode and serialization contracts.

## Requirements

### Requirement: Document holds a bottom-first layer tree

The `Document` SHALL expose a `layers` collection of `Layer` values whose first
element is the bottom layer and whose last element is the top layer, matching the
PSD `'Layr'` on-disk z-order. The codec SHALL NOT reverse this ordering on read
or on write.

#### Scenario: Two oracle layers read bottom-first

- **WHEN** `two_layers.psd` (authored by `psd-tools`) is read
- **THEN** `doc.layers` has length 2, `layers[0].name == "Red"` with bounds
  `(top=0, left=0, bottom=4, right=4)`, and `layers[1].name == "Blue"` with
  bounds `(top=4, left=4, bottom=8, right=8)`

#### Scenario: Order survives a write/read round trip

- **WHEN** a document whose layers are `[Red, Blue]` bottom-to-top is written and
  read back
- **THEN** the two layers appear in the same `[Red, Blue]` order

#### Scenario: Groups nest their children bottom-first

- **WHEN** `group.psd` is read
- **THEN** `layers[0]` is a group named `"Group A"` and its `children` are
  `["Inner Green", "Inner Yellow"]` in that order

### Requirement: Layers carry the M1 property set

Each `Layer` SHALL carry a name, signed bounds (`PsdRect`), opacity (`0..=255`),
blend mode, clipping flag, visibility flag, optional raster mask, a channel list,
and group children. Visibility SHALL be derived from layer flag bit 1; clipping
SHALL come from the layer record's dedicated clipping byte, not from a flags bit.

#### Scenario: Oracle layer properties

- **WHEN** `two_layers.psd` is read
- **THEN** the bottom layer has `blend == BlendMode::Normal` and `opacity == 255`

#### Scenario: Visibility and clipping are read from their own fields

- **WHEN** a layer record has the visibility flag bit set, or the clipping byte
  set
- **THEN** the resulting `Layer.visible` / `Layer.clipping` reflect those fields
  independently, and the `0x08` flag psd-tools writes on ordinary layers is not
  mistaken for clipping

### Requirement: BlendMode maps all 27 PSD keys

`BlendMode` SHALL provide the 27 blend modes Photoshop CS6 exposes for a layer,
each mapping to exactly one 4-byte PSD key, plus the group-only `PassThrough`
mapping to `pass`. `PassThrough` SHALL be excluded from the 27-entry layer-mode
list. Parsing an unknown 4-byte key SHALL return `None` rather than a fallback
mode.

#### Scenario: All 27 keys round-trip and are unique

- **WHEN** every mode in `BlendMode::LAYER_MODES` is converted to its PSD key and
  parsed back
- **THEN** each key parses to its original mode and all 27 keys are distinct

#### Scenario: Pass Through is group-only

- **WHEN** `PassThrough` is converted to a key and back
- **THEN** the key is `pass`, the round trip succeeds, and `PassThrough` is not a
  member of `BlendMode::LAYER_MODES`

#### Scenario: Unknown keys are rejected

- **WHEN** `BlendMode::from_psd_key` is called with an unrecognized key such as
  `zzzz` or `nrml`
- **THEN** it returns `None`

### Requirement: Per-layer channels identify color, transparency, and mask

Layer channel data SHALL be identified by the PSD channel IDs: `0, 1, 2…` for
color, `-1` for the transparency mask, `-2` for the user layer mask, and `-3`
for the real user mask. Each `Channel` SHALL hold a planar, row-major buffer
whose length is the pixel count of the owning rectangle.

#### Scenario: Transparency channel round-trips

- **WHEN** a pixel layer carrying color channels and a `-1` transparency channel
  is written and read back
- **THEN** the `-1` channel is present with the same decoded bytes

#### Scenario: Layer mask channel is exposed as mask data

- **WHEN** `masked.psd` is read
- **THEN** the `-2` channel is exposed as `LayerMask.data` with length 64
  (an 8×8 mask), not as a color channel

### Requirement: Raster layer masks

A raster layer mask SHALL carry mask bounds (`PsdRect`), a default color, a
disabled flag, the raw mask flags byte, and optional decoded data. The `-2`
channel SHALL be sized by the mask rectangle, which may differ from the layer
rectangle.

#### Scenario: Mask bounds and data from the oracle

- **WHEN** `masked.psd` is read
- **THEN** the mask exists with bounds `(0, 0, 8, 8)` and `data` of length 64

#### Scenario: Disabled mask flag round-trips

- **WHEN** a layer mask with `disabled == true` is written and read back
- **THEN** `mask.disabled` is still `true` and the mask data is unchanged

### Requirement: Signed geometry and big-endian serialization contract

All multi-byte integers written to or read from disk SHALL be big-endian.
Rectangle edges SHALL be signed `i32` and MAY lie outside the canvas; the codec
SHALL NOT clamp layer or mask bounds to the document.

#### Scenario: Off-canvas and offset bounds keep their size

- **WHEN** a `PsdRect` has negative or beyond-canvas edges, for example
  `(top=-10, left=-20, bottom=30, right=40)`
- **THEN** `width()` is `60` and `height()` is `40`, derived from the signed
  edges without clamping

#### Scenario: Multi-byte fields are big-endian on disk

- **WHEN** a document of known dimensions is written
- **THEN** the header's height and width fields are encoded big-endian, so an
  independent reader recovers the same dimensions

### Requirement: Document and layer clones share their storage

Cloning a `Document` or `Layer` SHALL be a refcount bump for every bulk store it
owns: pixel planes, retained native sample stores, smart-object payloads and the
raw linked-section bytes (`Document.layer_section_extra`), preserved byte blocks
(including `Document.source_icc`, `Document.document_icc`,
`Document.color_mode_data`, `Document.image_resources`,
`Document.global_layer_mask`, `Layer.blending_ranges`, `SmartFilter.options`, and
the `TypeTool` descriptors), and the layer tree. A clone MUST NOT copy those
buffers. A write SHALL fork only the store it touches; untouched stores SHALL
remain shared. Layer-tree structural edits SHALL fork the tree once, leaving the
source tree's nodes unchanged.

#### Scenario: Cloning a document shares native samples and payload

- **WHEN** a document carrying a 16-bit sample store and an embedded smart-object payload is cloned
- **THEN** the clone shares both allocations and equals the original field for field

#### Scenario: A smart-object payload and its raw linked section share

- **WHEN** a document with an embedded smart object is cloned
- **THEN** the clone shares `SmartObject.payload` and `Document.layer_section_extra` with the original, and both remain byte-identical

#### Scenario: A layer clone shares its byte blocks and layer tree

- **WHEN** a layer with `extra_blocks`, `raw_channels`, `blending_ranges`, a mask, adjustment data, smart-filter options, type descriptors, and child layers is cloned
- **THEN** the clone shares those block and child-layer allocations, and a later write to either side does not change the other

#### Scenario: Preserved document byte blocks share on clone

- **WHEN** a document with `source_icc`, `document_icc`, `color_mode_data`, `image_resources`, `global_layer_mask`, and `layer_section_extra` is cloned
- **THEN** the clone shares each of those allocations with the original

#### Scenario: A structural edit forks the tree

- **WHEN** a layer is added, removed, or reordered in a cloned document
- **THEN** the edit forks the layer tree once and the other document's tree is unchanged

#### Scenario: A written store forks alone

- **WHEN** a shared sample, payload, raw linked-section, or byte-block store is written
- **THEN** only that store is copied; every other shared store keeps its allocation

### Requirement: Pixel planes are band-granular copy-on-write

A pixel plane SHALL store its samples either as one contiguous allocation or as
one base allocation plus a sequence of 64-row bands. A band SHALL be either a
shared range into the plane's base allocation or an owned allocation holding a
forked copy of that band. Cloning a plane SHALL share every band and SHALL NOT
copy pixel bytes. Fracturing a plane SHALL record the plane's width and create
its band ranges without copying pixel bytes. A write to a band SHALL fork at
most that band's bytes and SHALL leave every other band shared with any other
holder of the plane. A band write SHALL write in place when the band is an owned
allocation with no other owner, or when the band is a shared range and the base
allocation has no other owner; otherwise it SHALL copy that band's range into an
owned allocation.

A whole-plane mutable write, through `as_mut_slice` or `DerefMut`, SHALL
materialize one contiguous allocation, replace the plane's band view with that
allocation, and MAY copy the plane once. Region and row writes SHALL go through
the band writer and SHALL fork only the bands they touch.

A contiguous read of a plane SHALL return the plane's logical bytes: the base
allocation while no band has diverged, and otherwise one materialized contiguous
view, cached until the next write. Per-band and per-row reads SHALL return the
same bytes as the contiguous read. A band's geometry SHALL be 64 rows of the
plane's width, and the last band MAY be shorter.

Fracturing SHALL record the width. A plane of length zero or width zero SHALL
fracture to no bands and SHALL read as empty. When the length is not a multiple
of the width, the final band SHALL hold the remaining samples, including any
trailing partial row; tracked-plane capture covers whole rows, so a trailing
partial row is not part of the tile grid. Re-fracturing an already fractured plane at the same width
SHALL be a no-op; re-fracturing at a different width, or after a band has
diverged, SHALL flatten once and rebuild the bands.

`shares(a, b)` SHALL be true only when every band of both planes is the same
allocation, meaning the same base with no band diverged. `shares_band(i, other)`
SHALL report whether band `i` is the same allocation as the corresponding band
of `other`.

#### Scenario: Fracturing copies no pixels [dm_band_fracture]

- **WHEN** a 4000×2000 plane is fractured so it can be shared
- **THEN** the band handles are created without allocating or copying pixel
  bytes, and together they cover the plane's original allocation

#### Scenario: A band write forks only its band [dm_band_write_forks_one]

- **WHEN** a fractured plane is cloned and one band of the clone is written
- **THEN** only that band's allocation changes, every other band is still one
  allocation shared with the clone, and the clone's bytes are unchanged

#### Scenario: Contiguous and band reads agree [dm_band_reads_agree]

- **WHEN** a plane is fractured, shared, and written, and its bytes are then read
  contiguously and per band
- **THEN** both reads equal the result of the same operations on an unfractured
  plane, and a second contiguous read with no intervening write allocates
  nothing

#### Scenario: Fracture preserves every byte [dm_band_round_trip]

- **WHEN** a plane holding arbitrary bytes is fractured, cloned, written, and
  compared with the same sequence applied to an unfractured plane
- **THEN** every read returns byte-identical samples

#### Scenario: A whole-plane write flattens [dm_band_whole_write_flattens]

- **WHEN** a fractured plane is written through `as_mut_slice` or `DerefMut`
- **THEN** the plane becomes one contiguous allocation, its band view is
  replaced, and any other holder of the previous bands keeps its bytes

#### Scenario: Degenerate planes stay well-defined [dm_band_degenerate_planes]

- **WHEN** an empty plane, a width-zero plane, or a plane whose length is not a
  multiple of its width is fractured, or an already fractured plane is
  fractured again
- **THEN** the empty and width-zero planes have no bands and read as empty, the
  final band holds the remaining whole rows, a trailing partial row is not part
  of the tracked-plane tile grid, and re-fracturing at the same width is a no-op
  while a different width flattens once and rebuilds
