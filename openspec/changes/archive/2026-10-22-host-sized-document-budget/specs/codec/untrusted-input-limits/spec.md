## MODIFIED Requirements

### Requirement: A document decodes within an allocation budget

Reading a PSD or PSB SHALL charge every composite plane, and every layer or
mask channel plane, against a budget derived from `ImageBudget` before
allocating it. A single allocation over `max_alloc_bytes`, a product that
overflows, or a document total over `max_document_bytes` SHALL be refused
with a typed error rather than allocated. By default `max_document_bytes`
SHALL be three quarters of the memory the host reports available when the
budget is read, and never less than one `max_alloc_bytes` plane. When the
host's available memory cannot be read, it SHALL be eight times
`max_alloc_bytes`. Layer and mask rects SHALL
be measured without integer overflow. A 16000²-class RGB document SHALL still
open under the default budget.

#### Scenario: A PSB header past the budget [ui_psb_header]

- **WHEN** a PSB declares 300000 × 300000 RGB with a tiny ZIP composite
- **THEN** the read fails with a typed error and no allocation over 4 MiB is
  requested

#### Scenario: Planes over the document total [ui_composite_total]

- **WHEN** a PSD declares 56 channels of 30000 × 30000 under a 16 GiB
  document limit
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

#### Scenario: The document limit follows the host [ui_host_budget]

- **WHEN** the host reports 48 GiB, 5 GiB, 1 GiB, or nothing available, with
  a 2 GiB plane cap
- **THEN** the document limit is 36 GiB, 3.75 GiB, 2 GiB, and 16 GiB, and
  `MemAvailable` is read in bytes from `/proc/meminfo`

#### Scenario: The document limit alone refuses [ui_document_limit]

- **WHEN** a 16 MiB plane is read under an 8 MiB document limit
- **THEN** the read fails, and the default limit is at least one 2 GiB plane
