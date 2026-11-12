## MODIFIED Requirements

### Requirement: The commit gate charges a move no layer planes

The transform commit's spillable cost SHALL charge a quad that is the original
rect moved by one whole-pixel offset only for the tiled composite of the union
of the old and new boxes (4 bytes a pixel). It SHALL charge any other quad for
its new layer tiles (4 bytes a pixel of the new box) as well.

#### Scenario: A whole-pixel move is charged only for the composite [tcm_gate_move]

- **WHEN** the commit cost of a 10 × 10 rect moved by (5, 0), and of the same rect moved by (5.5, 0), is computed
- **THEN** the first spillable cost is `4 · 150` and the second is `4 · 110 + 4 · 160`
