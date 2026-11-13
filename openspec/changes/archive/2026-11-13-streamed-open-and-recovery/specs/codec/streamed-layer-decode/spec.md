## ADDED Requirements

### Requirement: Layer channels decode a row at a time into tiles

Reading a layered PSD or PSB SHALL decode every 8-, 16- and 32-bit layer
channel and user mask channel that becomes a plane a scanline at a time, into a
tiled plane, for raw, RLE, ZIP and ZIP-with-prediction channels. A channel task
SHALL hold at most one 256-row strip of each plane it builds contiguously, and
tiles whose samples are all equal SHALL hold no allocation. A 16- or 32-bit
channel SHALL build its native plane and its 8-bit narrowing in the same pass.
The decoded samples, the compression word and every error SHALL be identical to
the dense decode of the same channel.

#### Scenario: Streamed channels match the dense decode [sd_identical]

- **WHEN** layer channels encoded as raw, RLE, ZIP and ZIP-with-prediction, at
  depth 8, 16 and 32, with widths that are and are not multiples of 256 and
  heights over one strip, are decoded through the streamed path
- **THEN** each 8-bit plane, and at 16/32 each native plane, equals the dense
  decode's output sample for sample

#### Scenario: Malformed channels report the dense errors [sd_errors]

- **WHEN** a raw channel is short, an RLE row overruns, or a ZIP payload is
  short, long, raw deflate, or garbage
- **THEN** the streamed decode returns the same `PsdError` variant and message
  as the dense decode, or decodes raw deflate to the same bytes

#### Scenario: Decoded layer channels are tiled [sd_tiled]

- **WHEN** an 8-bit layered PSD whose layer has one solid 1024 × 1024 channel
  and a user mask is read
- **THEN** every layer channel and the mask plane are tiled, and the solid
  channel allocates less than a quarter of its dense size

### Requirement: Tiled channels charge a tile budget

`ImageBudget` SHALL carry `max_tiled_bytes`, which defaults to
`max_document_bytes`. A channel decoded into tiles SHALL charge its decoded
bytes against `max_tiled_bytes`, and the strip buffers it holds against
`max_document_bytes`. Both charges SHALL apply the per-allocation cap and the
overflow check, happen before any channel decodes, and refuse with the same
"exceeds the decode budget" error.

#### Scenario: The tile budget admits and refuses [sd_tile_budget]

- **WHEN** a layered PSD whose layer planes exceed `max_document_bytes` but fit
  `max_tiled_bytes` is read, and then read again with `max_tiled_bytes` below
  its layer planes
- **THEN** the first read opens with byte-identical channels and the second
  fails with the decode budget error

### Requirement: A file can be read through a mapping

The codec SHALL offer a read-only memory map of a file that dereferences to its
bytes, so a reader can decode a PSD/PSB without copying the file into the heap.
A missing file SHALL be an I/O error, and an empty file SHALL map to an empty
slice.

#### Scenario: A mapped file reads as its bytes [sd_mapped]

- **WHEN** a written PSD is mapped and decoded, an empty file is mapped, and a
  missing path is mapped
- **THEN** the mapping equals the file's bytes and decodes to the same document
  as `read_psd`, the empty file maps to an empty slice, and the missing path is
  an error
