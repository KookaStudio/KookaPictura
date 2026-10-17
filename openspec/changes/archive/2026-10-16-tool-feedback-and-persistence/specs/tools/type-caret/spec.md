## ADDED Requirements

### Requirement: The caret contrasts with the image

The caret, the baseline guides, and the placement point SHALL draw white when
the image under the caret is dark (mean luminance below half, transparency read
as light) and black otherwise, re-evaluated on every edit and caret move.

#### Scenario: White over a dark image, black over a light one [caret_contrast]

- **WHEN** `tst_type_tools::caretContrastsWithTheImage` opens a Horizontal Type session on a black image, then on a white image
- **THEN** the caret draws white on the black image and black on the white one
