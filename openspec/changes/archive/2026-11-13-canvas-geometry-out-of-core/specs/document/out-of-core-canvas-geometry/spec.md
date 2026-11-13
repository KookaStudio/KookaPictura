## ADDED Requirements

### Requirement: Canvas Size writes into spillable tiles

`Image > Canvas Size` SHALL move every layer, mask and layer store rect by the
anchor offset without copying any layer plane. It SHALL write every document
alpha channel, every plane of the document's retained store, and the extended
Background one strip of 256 rows at a time, into tiled planes. Tiles whose
samples are all equal SHALL hold no allocation. The output SHALL be
byte-identical to the dense paths: `extend_channel` for the alpha channels,
`extend_samples` for the retained store, and the dense Background extension.

#### Scenario: Tiled Canvas Size matches the dense paths [ocg_canvas_exact]

- **WHEN** a seeded 8-bit 600 × 300 document with an alpha channel and a
  Background is resized to 900 × 700 at Center, to 300 × 200 at Bottom Right,
  and to 601 × 299 at Top Left, and its Background is extended with a colour
- **THEN** every alpha channel and Background channel is tiled and equals the
  dense output byte for byte, and every other layer's planes are the same
  shared planes as before

#### Scenario: The retained document store shifts through tiles [ocg_canvas_deep]

- **WHEN** a 16-bit and a 32-bit document, each with a three-plane retained
  document store, are resized to 900 × 700 at Center
- **THEN** every store plane is tiled and equals `extend_samples`' output
  byte for byte

#### Scenario: A uniform extension holds no allocation [ocg_canvas_uniform]

- **WHEN** a 1024 × 1024 document whose Background is solid white is resized to
  2048 × 2048 at Center and extended in white
- **THEN** each Background channel's allocated bytes are less than a quarter of
  the dense plane's size

### Requirement: Quarter turns and flips remap through tiles

`Image > Image Rotation` 90° CW, 90° CCW and 180°, and Flip Canvas Horizontal
and Vertical, SHALL remap every layer channel, every layer mask, every document
alpha channel, every plane of each layer's retained store (the `-2` plane at
the mask rect) and the document's retained store one strip of 256 rows at a
time into tiled planes. Each block SHALL read only the source window its
destination maps to. The output SHALL be byte-identical to the dense exact
remaps.

#### Scenario: Tiled turns and flips match the exact remap [ocg_orient_exact]

- **WHEN** a seeded 8-bit 600 × 300 document has a full-canvas layer, an offset
  layer with a mask of another rect, and an alpha channel, and is turned 90° CW,
  90° CCW and 180° and flipped both ways
- **THEN** every channel, mask and alpha channel is tiled and equals the dense
  remap byte for byte

#### Scenario: Retained stores turn through tiles [ocg_orient_deep]

- **WHEN** a 16-bit and a 32-bit document, each with a retained layer store
  holding a `-2` mask plane and a retained document store, are turned 90° CW and
  flipped horizontally
- **THEN** every retained plane is tiled and equals `remap_samples`' output byte
  for byte

### Requirement: Arbitrary Rotate resamples through tiles

`Image > Image Rotation > Arbitrary` SHALL resample every layer channel, every
layer mask, every document alpha channel, every plane of each layer's retained
store and the document's retained store one strip of 256 rows at a time into
tiled planes. Each block SHALL read only the source window that the inverse map
of its pixels reaches. The output SHALL be byte-identical to the dense bilinear
inverse map.

#### Scenario: Tiled Rotate matches the dense inverse map [ocg_rotate_exact]

- **WHEN** a seeded 8-bit 600 × 300 document has a full-canvas layer, an offset
  layer with a mask, and an alpha channel, and is rotated about its centre by
  15°, 45°, −30°, 123.4° and 270.5°, and about the point (100, 50) by 33°
- **THEN** every channel, mask and alpha channel is tiled and equals the dense
  inverse map byte for byte

#### Scenario: Retained stores rotate through tiles [ocg_rotate_deep]

- **WHEN** a 16-bit and a 32-bit document, each with a retained layer store
  holding a `-2` mask plane and a retained document store, are rotated by 20°
- **THEN** every retained plane is tiled and equals the dense inverse map's
  output byte for byte

#### Scenario: The rotated tiles can be spilled [ocg_output_spills]

- **WHEN** a 1024 × 1024 seeded document is rotated by 15° with a scratch
  volume configured, and eviction passes are run that spill every live tile
- **THEN** some of the layer's tiles are spilled, and reading the layer back
  equals the dense inverse map's output

#### Scenario: A large canvas command completes under a hard memory limit [ocg_memory_max]

- **WHEN** memory scenarios M16, M17 and M18 run at 17000 × 17000 under
  `MemoryMax=2G`, and again under `MemoryMax=4G`
- **THEN** each run applies its command without an OOM kill or a refusal

### Requirement: Canvas commands are gated on RAM plus scratch

Canvas Size, the quarter turns, the flips and arbitrary Rotate SHALL each be
admitted only when its working set fits the governor's room. The working set
is one source window per worker thread, one output strip of the widest plane,
and the dense composite when the document's composite is authoritative. The
planes the command writes, at their new sizes and sample widths, SHALL fit that
room plus the free space on the usable scratch volumes. When the working set
does not fit, the command SHALL be refused as a memory refusal. When the
planes do not fit, it SHALL be refused as a scratch-full refusal. A refused
command SHALL leave the document and the history unchanged.

#### Scenario: A large canvas command spills instead of refusing [ocg_gate_spills]

- **WHEN** the costs of a Canvas Size to 150 %, a 90° turn and a 15° Rotate of a
  17000 × 17000 document with a Background and one full-canvas layer are gated
  with the governor's room pinned at 1.75 GiB and 64 GiB of scratch available
- **THEN** each command is admitted and is expected to spill

#### Scenario: Full scratch disks refuse a canvas command [ocg_gate_scratch_full]

- **WHEN** the same commands are gated with 1.75 GiB of room and no scratch
  available
- **THEN** each is refused, and the refusal is recorded as a scratch-full
  refusal

#### Scenario: A refused Canvas Size names the shortfall [ocg_canvas_refused]

- **WHEN** Canvas Size is accepted for 30000 × 30000 with 260 MiB of memory
  available and no scratch space
- **THEN** the document keeps its size and history, and the status bar says
  the canvas could not be resized for lack of memory

#### Scenario: A refused rotation names the shortfall [ocg_rotate_refused]

- **WHEN** Rotate 90° CW, Flip Horizontal and Arbitrary Rotate 15° run from the
  menus with 260 MiB of memory available and no scratch space
- **THEN** none changes the document or the history, and the status bar says
  the canvas could not be rotated or flipped for lack of memory

#### Scenario: Full scratch disks are named [ocg_scratch_message]

- **WHEN** the Canvas Size gate is asked about 30000 × 30000 for a document with
  a Background, with 2 GiB of memory available (1.75 GiB of room) and no scratch
  space
- **THEN** it is false, and the refusal message for resizing the canvas says
  the scratch disks are full
