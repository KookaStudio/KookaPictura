## ADDED Requirements

### Requirement: Horizontal Type Mask tool

The Horizontal Type Mask tool SHALL open the Horizontal Type tool's session but
SHALL preview it as a red tint over the canvas with the letters cut out. Its
commit SHALL add no layer: it SHALL merge the type's coverage into the
selection — replacing it, or adding with Shift and subtracting with Alt held at
the click — and SHALL record exactly one "Horizontal Type Mask" state. Blank
text SHALL record nothing.

#### Scenario: Type becomes a selection

- **WHEN** the `tst_type_tools` mask test clicks, types "MASK", and presses Ctrl+Enter with the Horizontal Type Mask tool
- **THEN** one "Horizontal Type Mask" state is recorded, the layer count is unchanged, and the selection covers the letters but not the canvas corner

#### Scenario: Coverage is clipped and hard without anti-aliasing

- **WHEN** `type_layer::tests::type_mask_is_coverage_clipped_to_the_canvas` renders type running off the canvas, then with anti-aliasing off
- **THEN** the mask stops at the canvas edge, and without anti-aliasing every value is 0 or 255
