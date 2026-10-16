## ADDED Requirements

### Requirement: Background Eraser tool

The Background Eraser SHALL erase to transparency, at each dab, the pixels
within the tip that match a reference colour within the Tolerance, recording
exactly one "Background Eraser" history state per stroke that changed pixels.
Sampling SHALL take the reference from under the crosshair at every dab
(Continuous, skipping an already cleared pixel), from the first dab (Once), or
from the background colour (Background Swatch). Limits SHALL erase every match
in the tip (Discontiguous), only matches connected to the crosshair
(Contiguous), or connected matches not across a strong edge (Find Edges). With
Protect Foreground Color, pixels matching the foreground colour SHALL be kept.
It SHALL override Lock Transparency, SHALL turn the Background into a layer,
and SHALL refuse a layer whose pixels are locked.

#### Scenario: Cutting a subject out of its background

- **WHEN** the `background_eraser_tool` self-test drags down the sky of an opened image 4 px from a yellow subject
- **THEN** one "Background Eraser" state is recorded, the sky under the tip is transparent, and the subject and the far sky are kept

#### Scenario: Protecting the foreground colour

- **WHEN** a dab samples the background swatch while that colour is also the protected foreground
- **THEN** nothing is erased
