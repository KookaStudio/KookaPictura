## MODIFIED Requirements

### Requirement: Smart guide geometry

The smart-guide solver SHALL choose the smallest alignment offset per axis
within the threshold, prefer the earlier reference on ties, then the document,
and report every coincident guide line after snapping.

#### Scenario: Two coincident guides [sg_two_lines]

- **WHEN** `tst_smart_guides::twoLines` snaps a layer so its left edge meets a reference's left edge and its centre meets that reference's right edge
- **THEN** two vertical guide lines are reported at the two aligned positions

#### Scenario: A tie prefers the layer [sg_tie]

- **WHEN** `tst_smart_guides::tiePrefersLayer` places a moving edge equally close to a document target and a reference layer's target
- **THEN** the solver snaps to the reference layer's target, not the document's
