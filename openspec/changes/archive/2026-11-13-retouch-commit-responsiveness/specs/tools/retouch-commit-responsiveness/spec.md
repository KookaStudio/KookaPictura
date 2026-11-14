## ADDED Requirements

### Requirement: A retouch heal touches only its window

`heal_layer` SHALL materialize only the layer pixels the operation can read, and
SHALL write back only the rectangle the operation changed. Callers SHALL supply
a window containing the operation's region, its margin and boundary, the
clamped clone or source-search area, or the move destination as applicable. A
heal on a small region of a large layer MUST NOT allocate or copy a layer-sized
image.

#### Scenario: A small heal on a large layer materializes a bounded window [rcr_window]

- **WHEN** a heal runs on a 256×256 layer with a 16×16 region and a 32×32 window
- **THEN** the engine passes an image of the window to the operation, and a
  pixel far outside the window keeps its value

### Requirement: The relaxation is parallel and deterministic

The Poisson and Laplace solves SHALL update one colour of the 4-neighbour grid
at a time, so the cells of a colour update in parallel without reading a cell
another thread writes. The result SHALL be deterministic and SHALL stay within
the existing tolerance contract against the former serial relaxation.

#### Scenario: The parallel solve matches the serial reference [rcr_solver]

- **WHEN** the Poisson and Laplace fills run on a masked rectangle
- **THEN** each result is within a few channel units of the former serial
  relaxation

#### Scenario: The solve is deterministic [rcr_solver_deterministic]

- **WHEN** the same masked rectangle is solved twice
- **THEN** both results are identical

### Requirement: Red Eye is box-local and parallel

`red_eye_layer` SHALL materialize only its clipped box and SHALL process the
box's rows in parallel with the existing per-pixel red-dominance formula.
Pixels outside the box SHALL be unchanged, and the result SHALL match the serial
formula pixel for pixel.

#### Scenario: A wide box matches the definition [rcr_redeye_box]

- **WHEN** Red Eye runs over a wide box on a patterned layer
- **THEN** every pixel in the box matches the formula, the returned rectangle is
  the box, and pixels outside keep their values

### Requirement: Selection bounds are found in parallel

The bounding box of a document-sized selection mask SHALL be computed by
independent row scans and SHALL equal the serial scan's bounds.

#### Scenario: Parallel bounds match a serial scan [rcr_bounds]

- **WHEN** a mask with a covered rectangle and one outlying covered pixel is
  scanned
- **THEN** the bounds cover both and the coverage has the bounds' area
