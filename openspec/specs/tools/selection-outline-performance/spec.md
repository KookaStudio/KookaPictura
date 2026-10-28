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

### Requirement: Live selection outline is clipped to the canvas

While a selection tool drags, the live outline of a closed selection shape
SHALL be the dragged shape intersected with the canvas rectangle, so that a
shape leaving the canvas is drawn as the part of it inside the canvas instead
of stopping at the border. The intersection SHALL be closed along the canvas
border. A shape wholly inside the canvas SHALL be drawn unchanged, and a shape
wholly outside it SHALL draw no outline at all. The live outline shows the
shape being dragged, not the result of combining it with an existing selection.
A shape that overlaps itself SHALL be drawn as the boundary of the region it
encloses, which is the region the release selects; a shape that encloses
nothing inside the canvas SHALL keep its trace, cropped at the border, rather
than lose its band. This applies to the
rectangular and elliptical marquee, the freehand lasso, the magnetic lasso, and
the content-aware-move and patch region traces. The polygonal lasso's band
SHALL NOT be closed along the border: it is an open rubber band with no closing
edge while the gesture is open. A preview that draws an existing outline or a
non-selection overlay (the zoom marquee, the red-eye ring) SHALL NOT be clipped
to the canvas.

#### Scenario: A shape past the canvas edge closes along the border [live_outline_clipped]

- **WHEN** a selection shape is dragged from inside the canvas past its edge
- **THEN** the live outline runs to the canvas border, turns the corner there, and follows the border instead of stopping at it

#### Scenario: The live outline matches the released selection [live_outline_matches_committed]

- **WHEN** a marquee is dragged past the canvas edge and released in New mode with no prior selection
- **THEN** the region the live outline covered before the release is the region the committed outline covers after it

#### Scenario: A shape inside the canvas is unchanged [live_outline_inside]

- **WHEN** a selection shape is dragged wholly inside the canvas
- **THEN** its live outline is the dragged shape, with no border segment added

#### Scenario: A shape outside the canvas draws nothing [live_outline_outside]

- **WHEN** a selection shape is dragged wholly outside the canvas
- **THEN** no live outline is drawn

#### Scenario: An open band is not closed along the border [live_outline_open_band]

- **WHEN** the polygonal lasso's band is dragged past the canvas edge
- **THEN** the band stops at the border and gains no closing edge along it

#### Scenario: A combine mode shows the new shape [live_outline_combine_mode]

- **WHEN** a selection shape is dragged while the tool's combine mode is Add, Subtract, or Intersect
- **THEN** the live outline shows the shape being dragged, not the combination with the existing selection

#### Scenario: A shape that overlaps itself is drawn as its region [live_outline_self_overlapping]

- **WHEN** a selection shape is dragged so that it overlaps itself and crosses the canvas edge
- **THEN** the live outline is the boundary of the region it encloses, closed along the border, and no part of it lies outside the canvas

#### Scenario: A non-selection overlay is not clipped [live_outline_other_unclipped]

- **WHEN** the zoom tool drags its marquee past the canvas edge
- **THEN** its rubber band is drawn unclipped

### Requirement: Committed outline visible at the canvas border

The committed marching-ants outline SHALL remain continuously visible when a
selection edge coincides with the canvas border, at every pan and zoom. The
traced contour MUST remain the exact pixel boundary, including the canvas-border
edges; only the drawing clip may allow the cosmetic stroke to straddle the
border. A fractional pan or zoom SHALL NOT make a border segment flicker or
disappear.

#### Scenario: The border segment survives fractional pan and zoom [committed_outline_border]

- **WHEN** a selection touches the canvas border and the canvas is panned and
  zoomed to fractional offsets
- **THEN** the committed outline's border segment is drawn at every offset, with
  no frame where it disappears

#### Scenario: The visible segment is still the traced boundary [committed_outline_trace]

- **WHEN** the committed outline of an edge-touching selection is compared with
  its traced contour
- **THEN** the drawn loops and vertices are the contour's, with no segment added
  or moved by the drawing clip

### Requirement: Revision-keyed committed outline

The committed selection outline SHALL be cached against a selection revision
that changes whenever the selection is written. An edit that does not write the
selection MUST NOT retrace the outline, and viewing that unchanged outline MUST
NOT repaint the canvas. A selection write with a different mask MUST invalidate
the cached outline and retrace it from the new mask.

#### Scenario: An edit that does not write the selection keeps the outline [contour_cache_hit]

- **WHEN** a notifying edit commits and does not write the selection
- **THEN** the committed outline is not retraced and the canvas is not repainted
  because of the outline

#### Scenario: A paint stroke commit does not write the selection [contour_stroke_commit]

- **WHEN** a paint stroke commits without writing the selection
- **THEN** no committed outline is retraced

#### Scenario: A selection write with a different mask retraces the outline [contour_invalidate]

- **WHEN** the selection is written with a different mask
- **THEN** the committed outline is retraced from the new mask and shown on every
  canvas of the document

### Requirement: Bounded ants repaint

The marching-ants animation SHALL repaint only the canvas region covered by the
animated outline or live preview, expanded to cover the cosmetic stroke and
clipped to the widget. It MUST NOT repaint outside that clipped region. When the
animated outline spans the canvas, the whole widget may be repainted.

#### Scenario: The ants tick repaints only the outline region [ants_partial_region]

- **WHEN** the marching-ants animation advances on a canvas that shows a
  selection, live preview, or Move-tool outline
- **THEN** the repainted widget region covers the animated outline's bounds plus
  the stroke and, for a selection that does not span the canvas, is smaller than
  the full canvas
