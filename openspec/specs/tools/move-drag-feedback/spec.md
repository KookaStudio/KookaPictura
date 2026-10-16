# tools/move-drag-feedback Specification

## Purpose
The Move tool's drag feedback: marching ants round the moving content and the ΔX / ΔY readout in the rulers' unit.

## Requirements

### Requirement: A Move drag shows marching ants and an offset readout

While the Move tool drags a layer, the canvas SHALL outline the moving layer's
content box with marching ants that follow the move, and SHALL show a readout
by the pointer: `ΔX:` over `ΔY:`, the offset so far in the rulers' unit at the
document's resolution. The outline and readout SHALL clear on release.

#### Scenario: Ants and readout follow the drag [move_drag_feedback]

- **WHEN** `tst_move_feedback::dragShowsAntsAndReadout` drags a type layer 36 px right and 18 px down at 72 ppi with the rulers in inches, then releases
- **THEN** mid-drag the ants box is the layer's content box moved by the offset and the readout reads `ΔX: 0.500 in` over `ΔY: 0.250 in`; after release neither shows

#### Scenario: The readout follows the ruler unit [move_drag_units]

- **WHEN** `tst_move_feedback::readoutFollowsTheRulerUnit` reads a 36 × 72 px offset on a 200 × 160 px, 72 ppi document in pixels, centimetres, and percent
- **THEN** it reads `36 px` / `72 px`, `1.27 cm` / `2.54 cm`, and `18.0%` / `45.0%`
