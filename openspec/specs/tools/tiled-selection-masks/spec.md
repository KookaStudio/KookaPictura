# tools/tiled-selection-masks Specification

## Purpose
A selection is a sparse tiled mask on the document grid — `Empty` where nothing is selected, `Uniform` where a tile is selected whole, dense only along edges — and every selection operation works per tile or in bands of tiles with a margin as wide as its reach, byte-identical to the dense operations it replaces.

## Requirements

### Requirement: Selections are sparse tiled masks

A selection's coverage SHALL be stored as a `TiledMask`: 256 × 256 tiles on
the document grid, each `Empty`, `Uniform` or `Dense`, as defined by
`document/tiled-planes`. Each constructor SHALL write only the tiles its shape
crosses:
- an empty selection, and the tiles a shape does not reach, SHALL be `Empty`;
- Select All SHALL be `Uniform(255)`;
- a rectangle's or an ellipse's interior tiles SHALL be `Uniform(255)`, with
  dense tiles only along the shape's edge.

#### Scenario: A marquee's interior is uniform [tsm_marquee]

- **WHEN** a 3000 × 2000 rectangle and an ellipse are selected on a 4250 ×
  4250 document
- **THEN** every tile inside the shape is `Uniform(255)`, every tile outside
  it is `Empty`, dense tiles lie only on the shape's edge, and the bytes equal
  the per-pixel shape

#### Scenario: A polygon writes only its tiles [tsm_polygon]

- **WHEN** a lasso polygon is selected
- **THEN** its coverage equals the dense even-odd fill, and the tiles outside
  its bounds are `Empty`

### Requirement: Selection operations work per tile

Combine (replace, add, subtract, intersect), inverse, translate and bounds
SHALL work per tile, and SHALL decide a uniform tile without reading its
samples. Feather, expand, contract, border and smooth SHALL work in bands of
tile rows, each reading a margin around its tiles as wide as the operation's
reach. An output tile whose margin-grown window lies inside the canvas and
holds one value SHALL be that value without computation. Each result SHALL be
byte-identical to the dense implementation, and SHALL be a `TiledMask`. None of
these operations SHALL materialize a tiled plane.

#### Scenario: Boolean operations and inverse match the dense versions [tsm_boolean]

- **WHEN** seeded soft masks are combined with each operation, inverted and
  translated
- **THEN** every result equals the dense reference, and the uniform tiles of
  the inputs give uniform tiles in the output

#### Scenario: Neighbourhood operations match the dense versions [tsm_margin_ops]

- **WHEN** seeded masks are feathered (radii below and above the box-stack
  switch), expanded, contracted, bordered and smoothed at several radii
- **THEN** every result equals the dense reference, byte for byte

#### Scenario: The ImageMagick oracle still holds [tsm_oracle]

- **WHEN** the select oracle tests (`scripts/select_oracle.py`) run against
  the tiled masks
- **THEN** expand, contract and feather match ImageMagick within the
  documented tolerances, unchanged

#### Scenario: Selection operations do not materialize [tsm_no_materialize]

- **WHEN** a tiled selection is combined, inverted, feathered, expanded,
  contracted, bordered, smoothed, translated and bounded under the
  materialization guard
- **THEN** no tiled plane is materialized

### Requirement: Quick Mask keeps the tiles

Entering and leaving Quick Mask SHALL keep the selection a `TiledMask`, and a
round-trip with no edit SHALL return the same coverage.

#### Scenario: A Quick Mask round-trip keeps the tiles [tsm_quick_mask]

- **WHEN** a marquee selection enters and leaves Quick Mask without an edit
- **THEN** the selection is tiled, its interior tiles are `Uniform(255)`, and
  its bytes are unchanged
