## ADDED Requirements

### Requirement: A document decodes within an allocation budget

Reading a PSD or PSB SHALL charge every composite plane, and every layer or
mask channel plane, against a budget derived from `ImageBudget` before
allocating it. A single allocation over `max_alloc_bytes`, a product that
overflows, or a document total over eight times `max_alloc_bytes` SHALL be
refused with a typed error rather than allocated. Layer and mask rects SHALL
be measured without integer overflow. A 16000²-class RGB document SHALL still
open under the default budget.

#### Scenario: A PSB header past the budget [ui_psb_header]

- **WHEN** a PSB declares 300000 × 300000 RGB with a tiny ZIP composite
- **THEN** the read fails with a typed error and no allocation over 4 MiB is
  requested

#### Scenario: Planes over the document total [ui_composite_total]

- **WHEN** a PSD declares 56 channels of 30000 × 30000
- **THEN** the read fails before reading the image data

#### Scenario: A huge layer or mask rect [ui_layer_rect]

- **WHEN** a layer or mask rect is 2³¹ pixels wide (RLE, ZIP, or depth 1), or
  spans the whole `i32` range
- **THEN** the read fails with a typed error and no allocation over 4 MiB is
  requested

#### Scenario: Layer planes add up [ui_document_total]

- **WHEN** eight layers of three 40 000-byte planes are read under a 64 KiB
  per-allocation budget
- **THEN** the read fails, and two such layers read

#### Scenario: A canvas-class document still opens [ui_canvas_opens]

- **WHEN** a layered 16000 × 16000 RGB PSD without a merged image is read
  under the default budget
- **THEN** it opens

### Requirement: Group nesting is capped

Reading a PSD SHALL refuse layer groups nested deeper than 64.

#### Scenario: Deep groups are refused [ui_group_depth]

- **WHEN** a PSD nests 65 groups, and another nests 64
- **THEN** the first read fails and the second opens with 64 nested groups, and
  a folder record with no open divider reads as an empty group

### Requirement: PDF streams and numbers are bounded

Inflating a PDF stream SHALL stop at a per-stream limit (`max_alloc_bytes`)
and a per-file budget, and an embedded Pictura PSD payload SHALL inflate
within `max_alloc_bytes`. Predictor `/Columns` and `/Colors`, object-stream
`/N` and offsets, and the Photoshop editing composite's size SHALL be bounded
and computed with checked arithmetic. AESV2 SHALL require a 128-bit key. The
object lexer SHALL refuse nesting deeper than 256. Page and form walks SHALL
stop at depth 64, and the first-leaf page walk SHALL end on a cycle.

#### Scenario: A zip bomb stream [ui_pdf_inflate]

- **WHEN** a stream inflates past the per-stream limit or the file's
  remaining budget, or an embedded payload past its limit
- **THEN** it decodes to nothing

#### Scenario: Hostile numbers [ui_pdf_numbers]

- **WHEN** a stream declares `/Columns 9223372036854775807`, a row size that
  overflows, `/Colors 4096`, an object stream with `/N` at `i64::MAX` and a
  negative offset, or an editing composite 2⁶² + 1 wide
- **THEN** each is refused or skipped without a panic, and sane values in the
  same file still decode

#### Scenario: A short AES key [ui_pdf_aes]

- **WHEN** an encrypted PDF names AESV2 with `/Length 40`
- **THEN** decryption reports an unsupported handler, and AES-CBC with a
  10-byte key decrypts to nothing

#### Scenario: Deep or cyclic structure [ui_pdf_depth]

- **WHEN** a PDF nests 200 000 arrays, chains 5 000 page-tree levels, or has
  a Pages node whose kid is itself
- **THEN** reading ends without a stack overflow or a hang, and nesting at
  the cap still reads

### Requirement: PDF import counts and render sizes are bounded

The Import PDF dialog SHALL list, probe, and thumbnail at most 500 pages, and
Images mode SHALL list at most 500 images. An embedded image SHALL render
scaled into the image budget, a page render or crop over the budget SHALL be
refused, and a non-positive image size SHALL not be wrapped. The PNG reader
for Ghostscript output SHALL refuse a header over the budget and inflate no
more than the rows its header implies.

#### Scenario: Many images [ui_pdf_counts]

- **WHEN** a page draws 520 image XObjects
- **THEN** 500 are listed

#### Scenario: Oversized renders [ui_pdf_render_size]

- **WHEN** an image declares `u32::MAX` square, or a crop asks for
  `u32::MAX` square, or an image is −5 wide
- **THEN** the render is scaled within 30 000 px and 2 GiB, the crop is
  empty, and the image is not wrapped

#### Scenario: Hostile Ghostscript output [ui_png_decode]

- **WHEN** a PNG declares `u32::MAX` square, or carries far more IDAT data
  than its 1 × 1 header implies
- **THEN** the first is refused and the second decodes its one pixel

### Requirement: Ghostscript runs are bounded

Each Ghostscript run SHALL be killed after 300 seconds. On Unix it SHALL run
under address-space, CPU-time, and file-size limits. At most 16 MiB of each
output stream SHALL be kept, and a rendered page file larger than 4 GiB SHALL
be refused. A Ghostscript older than 9.55 SHALL be refused. An input path
that contains the path-list separator SHALL be read through a private copy.

#### Scenario: A run that never ends [ui_gs_timeout]

- **WHEN** the interpreter does not exit before the timeout
- **THEN** it is killed and the run fails naming the timeout

#### Scenario: Resource and output limits [ui_gs_limits]

- **WHEN** a run writes 40 MB of output, or reports its own limits
- **THEN** the kept output is capped and the file-size and address-space
  limits are set

#### Scenario: An old Ghostscript [ui_gs_version]

- **WHEN** `gs --version` reports 9.26
- **THEN** it is refused as too old, and 10.06.0 is accepted

#### Scenario: A colon in the input path [ui_gs_separator]

- **WHEN** the input is `a:b.pdf`
- **THEN** Ghostscript is given a separator-free copy and arguments naming it

#### Scenario: A real infinite PostScript loop [ui_gs_real]

- **WHEN** a PDF whose content stream loops forever is opened with real
  Ghostscript
- **THEN** the open fails after the timeout and the app stays responsive

### Requirement: The Qt decode edge honors the budget

Qt image decoding SHALL refuse a declared or decoded size over
`ImageBudget::default()` before handing pixels on. Qt's allocation limit
SHALL be the budget. A raster file larger than `max_alloc_bytes` SHALL not be
read. The Import PDF dialog SHALL unpack a thumbnail only when its declared
size is at most 30 000 a side and fits the bytes that remain.

#### Scenario: Qt decode sizes [ui_qt_decode]

- **WHEN** valid PNGs of 29999 × 1, 30001 × 1, and 1 × 30001 are decoded, and
  a file one byte over a read cap is read
- **THEN** only the first decodes, and the capped read refuses the file

#### Scenario: Hostile thumbnail sizes [ui_thumb_unpack]

- **WHEN** packed thumbnails declare `u32::MAX` square, 2³⁰ wide, 40 000
  wide, or 2 × 2 with 15 bytes
- **THEN** none is unpacked, and 2 × 2 with 16 bytes is

### Requirement: A seeded mutation test guards the readers

A fixed-seed mutation pass SHALL run in the normal test suite over valid PSD,
PDF, and PNG files. A panic, or a single allocation over 64 MiB while
decoding under a 1 MiB per-allocation budget, SHALL fail the test.

#### Scenario: Mutated files [ui_fuzz]

- **WHEN** the PSD, PDF, and PNG readers parse thousands of byte-flipped,
  boundary-word, and truncated variants of their seeds
- **THEN** none panics or allocates past the bound
