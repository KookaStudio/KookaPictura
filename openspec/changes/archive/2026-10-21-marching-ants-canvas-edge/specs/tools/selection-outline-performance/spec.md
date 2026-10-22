# Spec Delta

## ADDED Requirements

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
