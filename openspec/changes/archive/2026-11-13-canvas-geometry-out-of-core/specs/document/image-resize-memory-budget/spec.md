## MODIFIED Requirements

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
