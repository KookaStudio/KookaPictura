# document/image-resize-memory-budget Specification

## Purpose
Image Size and the resize bridge refuse a target that does not fit the document memory budget, report the shortfall, and leave the document unchanged instead of being OOM-killed.

## Requirements

### Requirement: Interactive Image Size refuses a target over the document budget

The application SHALL refuse `Image > Image Size` when the requested pixel size
and the document's depth do not fit the `ImageBudget` dimension and
per-allocation caps, or when the target's `width × height × 4 × depth` bytes
exceed the memory governor's room (`document/memory-governor`). The check MUST
run before any layer, composite, or display plane is allocated. A refused resize
MUST leave every document field, the history, and the modified flag unchanged.
The `PictureView::resize_image` bridge SHALL apply the same gate. A
resolution-only change (Resample off) allocates no pixels and MUST NOT be
refused by the gate.

#### Scenario: An over-budget Image Size is refused [rs_refuses]

- **WHEN** `image_size_apply` is asked to resample a small document to
  30000 × 30000 at 8 bits
- **THEN** it returns false, the document keeps its width, height and history,
  and no state is recorded

#### Scenario: The resize bridge refuses too [rs_bridge_refuses]

- **WHEN** `PictureView::resize_image` is asked for an over-budget size
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

### Requirement: The refusal names the memory shortfall

The Image Size handler SHALL tell the person that the target cannot be resized
because the memory budget does not allow it, naming the requested width and
height, and MUST NOT apply the resize.

#### Scenario: The refusal message names the target [rs_message]

- **WHEN** the budget refuses 30000 × 30000
- **THEN** `imageSizeFitsBudget` is false and `imageSizeBudgetMessage` names
  30000 × 30000 and the memory shortfall

#### Scenario: The handler shows the warning [rs_warning_shown]

- **WHEN** the Image Size dialog is accepted with a target the budget refuses
- **THEN** the handler shows one warning naming the target, records no command,
  and leaves the document unchanged

### Requirement: One budget predicate for every document allocation

New Document, Image Size, the `resize_image` bridge, and the control server
SHALL share one `fits` implementation of the dimension, per-allocation, and
per-document byte caps. The resize path SHALL supply the document-byte limit
bounded by the memory governor's room, without the default minimum of one
maximum allocation; New Document and a decode keep the default limit, and each
is then gated by the governor (`document/memory-governor`).

#### Scenario: The shared predicate judges sides then bytes [rs_shared_budget]

- **WHEN** the shared predicate is given synthetic budgets and a room bound
- **THEN** a side over the dimension limit is refused, a document over the
  per-allocation or per-document byte limit is refused, a document inside both
  is accepted, and the resize limit drops the default minimum allocation floor
