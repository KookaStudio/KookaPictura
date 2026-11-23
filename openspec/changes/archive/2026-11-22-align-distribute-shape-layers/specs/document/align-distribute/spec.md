# document/align-distribute Specification

## MODIFIED Requirements

### Requirement: Align and Distribute layers

Layer > Align SHALL line the selected layers' content (the box round each
layer's non-transparent pixels, or a shape layer's vector outline bounds) up
along the chosen edge against the box round all of them, given two layers with
content; Layer > Align Layers To Selection SHALL line them up against the
selection's bounds, given a selection and one layer. Layer > Distribute SHALL
space three or more layers so the chosen edge of each falls at even steps
between the two outermost, which stay put, and SHALL include shape layers by
their vector outline bounds. Aligning or distributing a shape layer SHALL
translate its outline and leave its canvas-sized fill rectangle in place. A
position-locked layer or the Background SHALL hold still. Each command SHALL
record one history state named for its edge.

#### Scenario: Align and distribute by content

- **WHEN** the `align` unit tests align squares by top, right, and horizontal centre, align one square to a selection, align a square painted inside a larger transparent layer, align with a position-locked layer, and distribute four squares by their left edges
- **THEN** every edge meets the outermost or the middle of the union, the single square meets the selection's edge, the painted square (not the layer) lines up, the locked layer stays put, and the middle squares move to even steps while the ends stay

#### Scenario: Shape layers align and distribute by their outlines

- **WHEN** the `align` unit tests align three shape layers by top and distribute them by left, and the `tst_align_distribute` test selects shape layers and uses Align ▸ Top and Distribute ▸ Left with the Move tool's buttons enabled
- **THEN** each shape's outline bounds meet the target edge, the middle outline moves to the even step, the fill rectangles stay canvas-sized, and the commands record the named history states

#### Scenario: Menu commands enable and act on the selected layers

- **WHEN** the `tst_align_distribute` test selects one, two, and three layers and triggers Align ▸ Top, Distribute ▸ Left, and, with a selection, Align Layers To Selection ▸ Bottom
- **THEN** Align needs two layers, Distribute three, Align Layers To Selection a selection, and each records one "Align Top Edges" / "Distribute Left Edges" / "Align Bottom Edges" state that moves the layers as expected
