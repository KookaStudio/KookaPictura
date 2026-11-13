# document/image-resize-memory-budget Specification

## Purpose
Image Size and the resize bridge refuse a target that does not fit the document memory budget, report the shortfall, and leave the document unchanged instead of being OOM-killed.

## Requirements

### Requirement: Interactive Image Size refuses a target over the document budget

The application SHALL refuse `Image > Image Size` when a requested side exceeds
the `ImageBudget` dimension limit, or when the resize does not pass the gate of
`document/out-of-core-image-size`. That gate refuses the resize when its
working set does not fit the memory governor's room, or when its resampled
planes do not fit that room plus the scratch disks' free space. The check MUST
run before any layer, composite, or display plane is allocated. A refused
resize MUST leave every document field, the history, and the modified flag
unchanged. The `PictureView::resize_image` bridge and the control server's
`image.image.size` SHALL apply the same gate. A resolution-only change
(Resample off) allocates no pixels and MUST NOT be refused by the gate.

#### Scenario: An over-budget Image Size is refused [rs_refuses]

- **WHEN** `image_size_apply` is asked to resample a small document to
  30000 × 30000 at 8 bits, with 260 MiB of memory available and no scratch space
- **THEN** it returns false, the document keeps its width, height and history,
  and no state is recorded

#### Scenario: The resize bridge refuses too [rs_bridge_refuses]

- **WHEN** `PictureView::resize_image` is asked for the same over-budget size
  under the same memory
- **THEN** it returns false and the document is unchanged

#### Scenario: A within-budget resize still applies [rs_allows]

- **WHEN** `image_size_apply` resamples a small document to a size inside the
  budget
- **THEN** it returns true, the document takes the new size, and one
  "Image Size" state is recorded

#### Scenario: A resolution-only change is not refused [rs_resolution_only]

- **WHEN** `image_size_apply` is called with Resample off to change only the
  resolution of a document
- **THEN** it returns true, the resolution changes, and the pixel dimensions are
  unchanged

#### Scenario: A side over the image budget is refused [rs_side_limit]

- **WHEN** `image_size_fits_budget` is asked about 30001 × 10 with ample memory
  and scratch space
- **THEN** it is false

### Requirement: The refusal names the memory shortfall

The Image Size handler SHALL tell the person that the target cannot be resized,
naming the requested width and height, and MUST NOT apply the resize. When the
refusal is a scratch-full refusal the message SHALL say that the scratch disks
are full; otherwise it SHALL say that there is not enough memory.

#### Scenario: The refusal message names the target [rs_message]

- **WHEN** the gate refuses 30000 × 30000 with 260 MiB of memory available
  and no scratch space
- **THEN** `imageSizeFitsBudget` is false and `imageSizeBudgetMessage` names
  30000 × 30000 and the memory shortfall

#### Scenario: The handler shows the warning [rs_warning_shown]

- **WHEN** the Image Size dialog is accepted with a target the gate refuses
- **THEN** the handler shows one warning naming the target, records no command,
  and leaves the document unchanged

#### Scenario: Full scratch disks are named [rs_scratch_message]

- **WHEN** the gate refuses a resize whose working set fits but whose planes do
  not fit the room plus the scratch disks
- **THEN** `imageSizeBudgetMessage` names the target and says the scratch disks
  are full

### Requirement: One budget predicate for every document allocation

New Document (from the menus or the control server) and a decode SHALL share
one `fits` implementation of the dimension, per-allocation, and per-document
byte caps. A budget bounded by the memory governor's room SHALL take the room as
its document-byte limit when that is lower, without the default minimum of one
maximum allocation. New Document and a decode are then gated by the governor
(`document/memory-governor`). Image Size and Canvas Size SHALL take only the
dimension limit from that budget. Image Size SHALL gate its bytes as
`document/out-of-core-image-size` describes, and Canvas Size SHALL gate its
bytes as `document/out-of-core-canvas-geometry` describes.

#### Scenario: The shared predicate judges sides then bytes [rs_shared_budget]

- **WHEN** the shared predicate is given synthetic budgets and a room bound
- **THEN** a side over the dimension limit is refused, a document over the
  per-allocation or per-document byte limit is refused, a document inside both
  is accepted, and the resize limit drops the default minimum allocation floor

#### Scenario: A Canvas Size side over the image budget is refused [rs_canvas_side_limit]

- **WHEN** `canvas_size_fits_budget` is asked about 30001 × 10 with ample memory
  and scratch space
- **THEN** it is false
