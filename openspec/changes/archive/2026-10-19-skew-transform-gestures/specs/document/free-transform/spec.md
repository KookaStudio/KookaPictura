## MODIFIED Requirements

### Requirement: Skew, Distort, and Perspective transform modes

The Free Transform session SHALL expose a mode of `Free` (the existing
similarity), `Skew`, `Distort`, or `Perspective`. In `Free` the existing scalars
drive the transform; in a projective mode the session SHALL hold the live target
quad and the scalars SHALL be unused. A corner drag in `Distort` SHALL set that
corner to the pointer; a corner drag in `Perspective` SHALL set that corner to
the pointer and move the opposite corner by the negated delta about the quad
centre. In `Skew`, a corner drag SHALL move that corner alone along whichever of
its two adjacent edges the drag follows most, and an edge drag SHALL slide that
edge's two endpoints only along the edge's own axis. Holding Alt in `Skew` SHALL
move the opposite corner, or the opposite edge, by the same displacement in the
opposite direction; Shift SHALL have no effect in `Skew`. A `Skew` corner drag on
an editable type layer SHALL slide the whole followed edge, keeping the quad a
parallelogram so the text stays type. A `Skew` corner drag SHALL stop where the
quad would become concave or fold over itself. A projective-mode gesture that
would make the quad degenerate SHALL be refused. Hit testing in a projective mode
SHALL accept only the eight bounding handles; a press away from a handle SHALL
not move or rotate. Committing SHALL call `transform_layer_quad` for the
projective modes and `transform_layer` for `Free`, recomposite, and record
exactly one `"Free Transform"` history state; an identity commit SHALL record
none; Escape SHALL leave the document bit-identical with no history state added.
The precise gesture semantics are inferred from Photoshop's Skew mode (no
reference image oracle); the projective map itself is exact.

#### Scenario: Distort commits the dragged corner

- **WHEN** a `Distort` session drags one corner and commits
- **THEN** the layer's source corner is at the pointer's document position and exactly one `"Free Transform"` state is added

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
