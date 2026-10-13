## ADDED Requirements

### Requirement: Photo Filter colour picker samples the image

While the Color Picker opened from the Photo Filter dialog's Color swatch is
open, the canvas SHALL show the eyedropper cursor, and a left click on the
image SHALL set the picker's colour to the document composite at that pixel
as it was before the dialog's preview. The click SHALL reach the canvas even
though both dialogs block input to their parent windows, and it MUST NOT run
the active tool or record history. A click outside the document SHALL leave
the colour unchanged. Closing the picker SHALL end the sampling.

#### Scenario: A canvas click samples the base image

- **WHEN** the Photo Filter preview is on and the user clicks a pixel of the
  image while the Color Picker is open
- **THEN** the picker takes that pixel's un-filtered composite colour, and
  accepting it sets the Color swatch and selects Use: Color

#### Scenario: Closing the picker ends sampling

- **WHEN** the Color Picker closes
- **THEN** the canvas sampler is disarmed and a later click reaches the
  active tool again
