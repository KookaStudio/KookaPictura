# tools/selection-outline-performance Specification

## Purpose
Run-based selection outline: draw a selection's border from horizontal runs.

## Requirements

### Requirement: Run-based selection outline

The marching-ants outline SHALL be traced from each row's runs of pixels at or
above the coverage threshold, reusing the previous row's runs when the row's
coverage bytes are identical. The union of drawn segments MUST equal the exact
4-connected boundary of the thresholded pixels, every loop MUST be closed and
axis-aligned, and collinear vertices MUST be omitted.

#### Scenario: The outline equals the pixel boundary

- **WHEN** an arbitrary mask is traced
- **THEN** the unit lattice edges drawn by its loops equal the set of edges
  between an inside pixel and an outside pixel or the canvas border

#### Scenario: A rectangle is its four corners

- **WHEN** a rectangular selection that is not the whole document is traced
- **THEN** the outline is one loop of exactly four vertices

### Requirement: Bounded marquee rasterisation

`Selection::rect` and `Selection::ellipse` SHALL write only the rows and
columns of their clipped bounding box, and their output MUST be byte-identical
to the per-pixel definitions (half-open rectangle; ellipse pixel centre inside
the inscribed ellipse). A shape with no feather SHALL NOT be copied before it
is applied.

#### Scenario: Shapes match the per-pixel definitions

- **WHEN** a rectangle or ellipse is built for a box inside, overlapping, or
  outside the canvas
- **THEN** its mask equals the per-pixel definition byte for byte

### Requirement: Offset ants during a move-selection drag

While a selection is dragged from inside, the canvas SHALL draw the committed
outline translated by the drag offset rather than retracing the translated
mask on each move; the mask itself SHALL still move live. Release and cancel
MUST drop the offset, so the canvas shows the retraced committed outline or
the origin outline.

#### Scenario: Drag offsets, release retraces

- **WHEN** a selection is pressed inside and dragged by `(dx, dy)`
- **THEN** the selection bounds move by `(dx, dy)` while the drawn contour is
  the origin contour at offset `(dx, dy)`
- **AND WHEN** the drag is released
- **THEN** the offset is zero and the contour is the traced moved mask

#### Scenario: Cancel drops the offset

- **WHEN** a move drag is cancelled by switching tools
- **THEN** the offset is zero, the selection is the origin, and no history
  state is recorded
