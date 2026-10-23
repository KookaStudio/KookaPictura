# Spec Delta

## REMOVED Requirements

### Requirement: Expand and contract

**Reason**: The edge-exempt rule is wrong for CS6: contracting the all-pixels selection must shrink it from the canvas border, and Border must keep an inner band on an edge-touching selection.

**Migration**: Use *Expand and contract at the canvas edge* below; it keeps the same `expand` and `contract` calls and the same square structuring element, and only out-of-canvas taps change from clamped to zero.

## ADDED Requirements

### Requirement: Expand and contract at the canvas edge

`expand(radius)` SHALL dilate the coverage and `contract(radius)` SHALL erode
it, using a separable square structuring element of side `2 * radius + 1`. A
radius of 0 MUST be the identity. Sampling MUST treat a tap outside the canvas
as zero coverage: `contract` SHALL erode an edge-touching selection inward,
while `expand` remains unchanged at the edge. `expand` and `contract` by the
same radius MUST be inverses on a binary selection clear of the canvas edge.

#### Scenario: Zero radius is identity

- **WHEN** `expand(0)` or `contract(0)` is applied
- **THEN** the selection is returned unchanged

#### Scenario: Expand then contract restores a binary block

- **WHEN** a binary rectangular selection clear of the canvas edge is expanded and then contracted by the same radius
- **THEN** the result equals the original selection

#### Scenario: Contract pulls away from the canvas edge [sm_contract_edge]

- **WHEN** the all-pixels selection is contracted by a radius
- **THEN** every side shrinks inward by that radius instead of staying selected
  along the canvas border

### Requirement: Border band

`border(width)` SHALL return the symmetric difference of
`expand(ceil(width / 2))` and `contract(floor(width / 2))`, a band centred on
the input edge. A width of 0 SHALL return the empty selection. At the canvas
edge the band SHALL keep its inner half.

#### Scenario: Border keeps its inner band at the edge [sm_border_edge_band]

- **WHEN** a selection touching the canvas edge is given a border width
- **THEN** the returned band covers the inner half of the width along that edge
  and the outer half is clipped by the canvas
