## ADDED Requirements

### Requirement: Align and Distribute layers

Layer > Align SHALL line the selected layers' content (the box round each
layer's non-transparent pixels) up along the chosen edge against the box round
all of them, given two layers with content; Layer > Align Layers To Selection
SHALL line them up against the selection's bounds, given a selection and one
layer. Layer > Distribute SHALL space three or more layers so the chosen edge
of each falls at even steps between the two outermost, which stay put. A
position-locked layer or the Background SHALL hold still. Each command SHALL
record one history state named for its edge.

#### Scenario: Align and distribute by content

- **WHEN** the `align` unit tests align squares by top, right, and horizontal centre, align one square to a selection, align a square painted inside a larger transparent layer, align with a position-locked layer, and distribute four squares by their left edges
- **THEN** every edge meets the outermost or the middle of the union, the single square meets the selection's edge, the painted square (not the layer) lines up, the locked layer stays put, and the middle squares move to even steps while the ends stay

#### Scenario: Menu commands enable and act on the selected layers

- **WHEN** the `tst_align_distribute` test selects one, two, and three layers and triggers Align ▸ Top, Distribute ▸ Left, and, with a selection, Align Layers To Selection ▸ Bottom
- **THEN** Align needs two layers, Distribute three, Align Layers To Selection a selection, and each records one "Align Top Edges" / "Distribute Left Edges" / "Align Bottom Edges" state that moves the layers as expected

### Requirement: Move tool Align and Distribute buttons

The Move tool's options bar SHALL show six Align and six Distribute buttons
that act on the selected layers, aligning against the selection when there is
one, and SHALL enable them as the corresponding menu commands are enabled.

#### Scenario: The buttons align and distribute

- **WHEN** the `tst_align_distribute` test activates the Move tool and clicks Align Left Edges and Distribute Top Edges with one, two, and three layers selected
- **THEN** the buttons enable with two and three layers respectively and record "Align Left Edges" and "Distribute Top Edges"
