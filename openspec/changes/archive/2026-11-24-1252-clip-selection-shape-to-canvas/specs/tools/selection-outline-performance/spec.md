# Spec Delta

## ADDED Requirements

### Requirement: Engine-owned selection preview clipping

The Qt-free selection engine SHALL own clipping of selection-preview traces to a canvas rectangle. Closed traces SHALL use the even-odd fill rule, and the operation SHALL preserve loop order for traces that need no boolean or path operation. The C++ shell SHALL keep `pictura::clipSelectionShapeToCanvas` as its only clipping entry point and SHALL delegate the geometry to the engine.

#### Scenario: The shell delegates to the engine [preview_clip_engine]

- **WHEN** the canvas applies a selection-preview trace
- **THEN** the shell passes the loops and canvas size to the Qt-free engine and draws the returned loops
- **AND THEN** no second C++ clipping implementation remains in the production path

#### Scenario: An inner loop is unchanged [preview_clip_inner]

- **WHEN** every vertex of a loop lies inside or on the canvas boundary
- **THEN** the engine returns that loop with exact ordered equality and adds no border segment

#### Scenario: A wholly outside loop is dropped [preview_clip_outside]

- **WHEN** a loop lies wholly outside the canvas and does not touch it
- **THEN** the engine returns no loop for that input

#### Scenario: A short trace is unchanged [preview_clip_short]

- **WHEN** a loop has fewer than three points, including empty, one-point, and two-point traces
- **THEN** the engine returns it with exact ordered equality

#### Scenario: A closed trace that encloses nothing keeps its band [preview_clip_zero_area]

- **WHEN** a closed crossing trace has no filled area inside the canvas
- **THEN** the engine returns the original trace with exact ordered equality so the paint crop can retain its band

#### Scenario: An open zero-area band is clipped as a path [preview_clip_open_band]

- **WHEN** an open trace has zero signed area and crosses the canvas boundary
- **THEN** the engine clips its path to the canvas, includes the boundary, and returns no vertex outside the canvas
- **AND WHEN** no clipped path remains
- **THEN** the engine returns the original trace unchanged

#### Scenario: A crossing closes along the border [preview_clip_border]

- **WHEN** a filled loop crosses the canvas boundary
- **THEN** the returned boundary follows the intersected shape and closes along the canvas border
- **AND THEN** near-border coordinates are snapped to the exact edge, every returned vertex is clamped inside the canvas, and no vertex remains outside it

#### Scenario: A self-overlap resolves to its even-odd region [preview_clip_self_overlap]

- **WHEN** a filled loop crosses itself and the canvas boundary
- **THEN** the engine returns the boundary of the even-odd region inside the canvas
- **AND THEN** contours that share an exact arrangement endpoint are spliced into one connected outline

#### Scenario: Near-touching contours stay separate [preview_clip_near_touch]

- **WHEN** distinct contours have endpoints closer than `0.000001` document pixels but do not share an exact arrangement endpoint
- **THEN** the engine keeps those contours separate

#### Scenario: An empty or invalid canvas leaves the traces unchanged [preview_clip_empty_canvas]

- **WHEN** the canvas width or height is zero, negative, NaN, or infinite
- **THEN** the engine returns the input loops unchanged

#### Scenario: Degenerate intersections keep the current arity rules [preview_clip_degenerate]

- **WHEN** a boolean intersection returns no contour for a closed loop or a loop with nonzero signed area
- **THEN** the engine returns the original loop unchanged
- **AND WHEN** a nonempty intersection contains a contour with fewer than two points
- **THEN** the engine discards that contour

#### Scenario: Bridged clipping matches the Qt oracle [preview_clip_parity]

- **WHEN** the six pinned traces and the deterministic differential corpus are clipped through both the engine and a local copy of the complete old clipping function
- **THEN** preserved fast paths match with exact ordered equality
- **AND THEN** filled intersections match after simplifying collinear runs, splitting implicit edge crossings, and matching unordered undirected boundary segments, with every corresponding endpoint differing by at most 0.000001 document pixels
- **AND THEN** the comparison accepts vertex-order, ring-rotation, direction, loop-order, collinear-segmentation, and explicit-versus-implicit-crossing differences but rejects a topology change or a larger endpoint displacement
