## ADDED Requirements

### Requirement: Color Range coverage

Color Range SHALL produce a graded coverage mask for a sampled colour (within
Fuzziness 0–200), a hue band (Reds … Magentas, excluding greys), or a tonal band
(Highlights, Midtones, Shadows); raising Fuzziness SHALL never shrink it, and
Invert SHALL yield `255 − coverage`.

#### Scenario: Bands and fuzziness behave

- **WHEN** the `color_range` unit tests match red against red and blue, raise Fuzziness on a near miss and across a grey ramp, pick Reds / Greens against red, green, and grey, and pick Highlights / Shadows against white and black
- **THEN** the exact colour is fully selected and a far one not (reversed with Invert), more fuzziness takes more and partly, grey belongs to no colour band, and the tonal bands split light from dark

### Requirement: Color Range dialog

Select > Color Range… SHALL open a non-modal dialog, unavailable for 32-bpc
documents, whose preview is the mask OK will select; while its eyedropper is
down a canvas click SHALL sample the composite instead of reaching the active
tool. OK SHALL make a new selection, or narrow a live selection to its matching
subset, as one "Color Range" state; Cancel SHALL change nothing.

#### Scenario: Sample, preview, select, refine

- **WHEN** the `tst_color_range` test opens the dialog on a red square over white, clicks the canvas on the square, switches to Highlights and back, presses OK, cancels a second dialog, and runs it again on a live rectangle selection
- **THEN** the click samples red without a history state, the preview is white over the square and black elsewhere (Highlights the reverse, with the eyedropper off), OK records one "Color Range" state selecting only the square, Cancel records nothing, and the refine keeps only the red inside the rectangle
