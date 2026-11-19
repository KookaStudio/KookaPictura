# region-present-backend delta

## ADDED Requirements

### Requirement: A small region refresh composites on the CPU

The display's region refresh SHALL composite a region on the GPU only when GPU
compute is on and the region's aligned area is at least `REPAIR_PIXELS`
(4,000,000 level-0 pixels). A smaller region SHALL composite on the CPU oracle.
The render crate's composite request policy SHALL be unchanged.

#### Scenario: A stroke-sized region uses the CPU [rpb_small_cpu]

- **WHEN** GPU compute is on and a region refresh covers fewer than 4,000,000 pixels
- **THEN** the region composites on the CPU

#### Scenario: A region at the floor may use the GPU [rpb_large_gpu]

- **WHEN** GPU compute is on and the region covers at least 4,000,000 pixels
- **THEN** the region is offered to the GPU, and with GPU compute off it never is
