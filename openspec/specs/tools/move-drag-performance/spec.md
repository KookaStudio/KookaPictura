# tools/move-drag-performance Specification

## Purpose
The Move tool's selection drag stays interactive on large documents: bounds are
computed without a full-plane scan, a drag stores only its offset instead of a
translated plane, and a mask content move touches only the selection's
rectangle.

## Requirements

### Requirement: Selection bounds avoid a full-plane scan

The app SHALL compute the active selection's bounding box row by row: each row
is scanned only up to its first and last covered pixel, and an empty row SHALL
cost no more than one pass over that row. The reported rectangle MUST equal the
extent a per-pixel scan of the whole plane produces.

While a Move-tool selection drag is active, the reported bounds SHALL be the
drag origin's bounds translated by the drag offset and clipped to the canvas,
and the app MUST NOT touch the coverage plane to produce them. A drag that
pushes the selection past a canvas edge MUST report the clipped extent, equal to
a fresh scan of the translated plane.

#### Scenario: Bounds equal a full scan with empty padding rows [mdp_bounds_row_scan]

- **WHEN** a selection covers a rectangle inset from the canvas edges by empty rows and columns
- **THEN** the reported bounds are the covered rectangle's extent and equal a per-pixel scan of the same plane

#### Scenario: Bounds follow the drag offset [mdp_bounds_translated]

- **WHEN** a selection drag has moved the selection by `(dx, dy)`
- **THEN** the reported bounds equal a fresh scan of the selection translated by `(dx, dy)`

#### Scenario: Bounds clip at the canvas edge [mdp_bounds_clipped]

- **WHEN** a drag moves part of the selection past the canvas edge
- **THEN** the reported bounds are clipped to the canvas and still equal a fresh scan of the translated plane

### Requirement: A selection drag stores only the offset

A Move-tool selection drag SHALL keep the drag origin's coverage plane and store
the drag offset; it MUST NOT build a translated coverage plane per pointer move.
The translated plane SHALL be built exactly once, when the drag commits, and a
cancel MUST leave the origin selection in place. A zero-offset commit MUST record
no history state.

Coverage hit tests during a drag SHALL sample the origin plane at
`(x - dx, y - dy)`, so the reported coverage is the same as the translated
plane's.

#### Scenario: A preview leaves the plane unchanged [mdp_drag_keeps_origin]

- **WHEN** a selection drag previews an offset of `(dx, dy)`
- **THEN** the stored selection plane is still the origin plane and the offset is recorded

#### Scenario: Coverage follows the offset during a drag [mdp_drag_coverage]

- **WHEN** a drag has previewed `(dx, dy)` and a point inside the moved selection is hit-tested
- **THEN** the hit test samples the origin at `(x - dx, y - dy)` and reports the same coverage as the translated plane

#### Scenario: A commit materialises once and records one state [mdp_drag_commit]

- **WHEN** a drag that previewed `(dx, dy)` commits
- **THEN** the selection plane is the origin translated by `(dx, dy)` and exactly one history state is recorded

### Requirement: A mask content move is bounded to the selection

Moving mask coverage SHALL visit only the coverage's bounding rectangle and its
destination rectangle, each clipped to the mask, in contiguous row slices
without a per-pixel bounds check. Pixels outside the union of those rectangles
MUST keep their previous value, and the result MUST be byte-identical to the
full-plane implementation for the same input, including partial coverage, a
duplicate move, and an offset in either direction.

#### Scenario: The bounded move equals the full-plane move [mdp_mask_move_equal]

- **WHEN** the same fixed-seed random coverage plane, mask, offset, and duplicate flag are applied by the bounded move and by the full-plane reference
- **THEN** every byte of the resulting mask plane is identical

#### Scenario: Pixels outside the move keep their value [mdp_mask_move_outside]

- **WHEN** a mask is moved by `(dx, dy)` with a selection that covers part of it
- **THEN** every mask pixel outside the union of the source and destination rectangles is unchanged
