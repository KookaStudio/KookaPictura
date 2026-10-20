## MODIFIED Requirements

### Requirement: Projective (quad) layer transform op

The system SHALL provide
`pictura_render::transform_layer_quad(doc: &mut Document, path: &str, quad:
[(f64, f64); 4]) -> bool`, where `quad` gives the document-space target of the
source layer rect's four corners in top-left, top-right, bottom-right,
bottom-left order. On success the operation SHALL resample every channel plane
(and the layer mask, about its own rect) with the bilinear inverse of the
projective map sending the source corners to `quad`, SHALL set the layer `rect`
to the integer bounding box of `quad` (`floor(min x)`, `floor(min y)`,
`ceil(max x)`, `ceil(max y)`), and SHALL return `true`. A destination pixel whose
inverse-mapped source point lies outside `[0, w) × [0, h)` SHALL be written as
`0`, and bilinear taps SHALL be edge-clamped. A destination pixel whose
inverse-mapped source point lies on the far side of the projective horizon SHALL
also be written as `0`. The front side SHALL be the side holding the majority of
the source rect's four corners' forward homogeneous divisors, or the centre's
side on a tie. The live projective preview SHALL clip to the same front side.
The operation SHALL refuse and MUST leave `doc` bit-identical for every case
`transform_layer` refuses, and also when any corner is non-finite, the
projective map is singular or near-singular, or the result rect has zero area or
exceeds the op's pixel ceiling. It SHALL NOT recomposite the document and MUST
NOT panic on any input.

#### Scenario: An identity quad is the source rect

- **WHEN** `transform_layer_quad` is called with `quad` equal to the source rect corners
- **THEN** it returns true and the layer rect and every plane are unchanged

#### Scenario: A translated quad matches the similarity translation

- **WHEN** `transform_layer_quad` is called with every corner translated by an integer `(dx, dy)`
- **THEN** the resulting layer rect and channel planes equal those produced by `transform_layer` with the same translation

#### Scenario: A perspective quad maps the corners onto the targets

- **WHEN** a projective quad with a non-affine corner is applied
- **THEN** the source corners map onto the requested `quad` within a small tolerance

#### Scenario: Degenerate quads are refused

- **WHEN** `quad` has a non-finite corner, three collinear corners, or zero area
- **THEN** it returns false and every field of `doc` equals its pre-call value

#### Scenario: Pixels outside the source are transparent

- **WHEN** a projective quad pulls part of the destination outside the source rect
- **THEN** every destination pixel whose source point is outside the source rect has all channels equal to `0`

#### Scenario: Past the horizon only the front side is drawn [quad_front_side]

- **WHEN** a projective quad leaves part of the source rect beyond the horizon
- **THEN** every destination pixel whose inverse-mapped source point is on the far side is transparent, while the front side is drawn

#### Scenario: The projective preview clips at the horizon [preview_front_side]

- **WHEN** a live projective preview's map has a horizon crossing the source
- **THEN** the preview draws only the front side and never the mirrored far side

### Requirement: Skew, Distort, and Perspective transform modes

The Free Transform session SHALL expose a mode of `Free` (the existing
similarity), `Skew`, `Distort`, or `Perspective`. In `Free` the existing scalars
drive the transform; in a projective mode the session SHALL hold the live target
quad and the scalars SHALL be unused. A corner drag in `Distort` SHALL move that
corner by the press-to-pointer delta, preserving the grab offset, and SHALL stop
where the quad would become concave or fold over itself. A `Distort` corner drag
SHALL carry the reference point with the quad, keeping its position within the
box. When document guides are visible, a `Distort` corner SHALL snap to a
vertical guide and/or a horizontal guide within 8 screen pixels of its proposed
position, and the snapped position SHALL be the one committed. `Distort` SHALL
be refused for an editable type layer unless the session targets pixels lifted
by a selection. A corner drag in `Perspective` SHALL set that corner to the
pointer and move the opposite corner by the negated delta about the quad centre.
In `Skew`, a corner drag SHALL move that corner alone along whichever of its two
adjacent edges the drag follows most, and an edge drag SHALL slide that edge's
two endpoints only along the edge's own axis. Holding Alt in `Skew` SHALL move
the opposite corner, or the opposite edge, by the same displacement in the
opposite direction; Shift SHALL have no effect in `Skew`. A `Skew` corner drag on
an editable type layer SHALL slide the whole followed edge, keeping the quad a
parallelogram so the text stays type. A `Skew` corner drag SHALL stop where the
quad would become concave or fold over itself. A projective-mode gesture that
would make the quad degenerate SHALL be clamped to the last convex quad, or
refused when no finite convex result exists. Hit testing in a projective mode
SHALL accept only the eight bounding handles; a press away from a handle SHALL
not move or rotate. Committing SHALL call `transform_layer_quad` for the
projective modes and `transform_layer` for `Free`, recomposite, and record
exactly one `"Free Transform"` history state; an identity commit SHALL record
none; Escape SHALL leave the document bit-identical with no history state added.
The precise gesture semantics are inferred from Photoshop's Distort and Skew
modes (no reference image oracle); the projective map itself is exact.

#### Scenario: Distort commits the dragged corner

- **WHEN** a `Distort` session drags one corner and commits
- **THEN** the layer's source corner is at its press-time position plus the pointer delta and exactly one `"Free Transform"` state is added

#### Scenario: A Distort corner keeps the grab offset [distort_grab_offset]

- **WHEN** a `Distort` corner is grabbed a few pixels away from its exact handle position and dragged
- **THEN** the corner moves by the pointer delta and does not jump to the pointer

#### Scenario: A Distort corner stops before folding [distort_convex]

- **WHEN** a `Distort` session drags a corner past the point where the quad would become concave
- **THEN** the stored quad stays convex and the dragged corner has not crossed to the other side

#### Scenario: A Distort corner carries the reference point [distort_pivot]

- **WHEN** a `Distort` session drags a corner
- **THEN** the reference point keeps the same position within the moving box

#### Scenario: A Distort corner snaps to a visible guide [distort_guide_snap]

- **WHEN** a `Distort` corner is dragged within 8 screen pixels of a visible document guide
- **THEN** the corner lands on the guide and the committed quad uses that position

#### Scenario: Distort is refused on editable type [distort_type]

- **WHEN** `Distort` is begun on an editable type layer without a selection that lifts its pixels
- **THEN** no session begins and the document is unchanged

#### Scenario: Perspective mirrors the opposite corner

- **WHEN** a `Perspective` session drags a corner by a delta and commits
- **THEN** the opposite corner moves by the negated delta and the quad centre is unchanged

#### Scenario: A Skew corner follows its dominant edge [skew_corner]

- **WHEN** a `Skew` session drags a corner mostly along one of its two adjacent edges
- **THEN** only that corner moves, along that edge, and the other corners stay put

#### Scenario: Skew slides one edge

- **WHEN** a `Skew` session drags an edge handle with an off-axis component
- **THEN** the edge's two endpoints move only along the edge's axis and the opposite edge is unchanged

#### Scenario: Alt mirrors the opposite corner or edge [skew_alt]

- **WHEN** a `Skew` session drags a corner or edge with Alt held
- **THEN** the opposite corner or edge moves by the same displacement in the opposite direction

#### Scenario: Skew keeps editable type a parallelogram [skew_type]

- **WHEN** a `Skew` session on an editable type layer drags a corner
- **THEN** the corner's whole followed edge slides and the quad remains a parallelogram

#### Scenario: A Skew corner stops before folding [skew_convex]

- **WHEN** a `Skew` session drags a corner past the point where the quad would become concave
- **THEN** the stored quad stays convex and the dragged corner has not crossed to the other side

#### Scenario: Escape cancels a projective mode exactly

- **WHEN** a projective-mode session is begun, a gesture is applied, and Escape is pressed
- **THEN** the document equals its pre-session value and the history count is unchanged

#### Scenario: A degenerate drag is refused

- **WHEN** a projective gesture would place three corners on one line
- **THEN** the session's quad is not collapsed to a degenerate shape

### Requirement: Skew, Distort, and Perspective menu commands

The application SHALL provide implemented commands `Edit > Transform > Skew`,
`Edit > Transform > Distort`, and `Edit > Transform > Perspective` that begin a
Free Transform session in the matching mode on the active layer, using the same
active-layer resolution and refusal as `Edit > Free Transform`. The Skew and
Perspective commands SHALL be enabled exactly when `Edit > Free Transform` is
enabled, including for a target whose selected pixels Free Transform lifts. The
Distort command SHALL additionally be disabled when the target is an editable
type layer whose whole layer would move; a selection that lifts its pixels keeps
the command enabled.

#### Scenario: The Distort command enters a Distort session

- **WHEN** `Edit > Transform > Distort` is invoked with exactly one transformable layer active
- **THEN** a session begins in `Distort` mode on that layer

#### Scenario: The Distort command is disabled for editable type [distort_menu_type]

- **WHEN** the active layer is an editable type layer and no selection lifts its pixels
- **THEN** the Distort command is disabled and invoking it begins no session

#### Scenario: A non-transformable target leaves the command disabled

- **WHEN** the active layer is a group, an adjustment layer, or position-locked, or is the Background with no active selection
- **THEN** the Distort/Skew/Perspective commands are disabled and invoking them begins no session
