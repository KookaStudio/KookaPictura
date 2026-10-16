## ADDED Requirements

### Requirement: Red Eye engine

`pictura_paint::healing::red_eye_layer` SHALL neutralise red-eye in one pixel
layer inside a document rectangle clipped to the canvas. A pixel SHALL be
treated as red only when its red channel is at least `1.8 − 0.75 × Pupil
Size / 100` times the larger of its green and blue; such a pixel's red SHALL
become the mean of its green and blue, and all three channels SHALL then be
scaled by `1 − 0.6 × Darken Amount / 100`, keeping alpha. Every other pixel
SHALL be unchanged. A box with no red pixel or outside the canvas SHALL be a
no-op, and a pixel-locked or non-raster layer SHALL be refused.

#### Scenario: A red pupil is neutralised and skin is kept

- **WHEN** `red_eye_layer` runs at Pupil Size 50 / Darken Amount 50 over a red pupil on skin tone
- **THEN** the pupil's red falls below 80 and the skin pixels in the box are unchanged

#### Scenario: Darken Amount darkens the result

- **WHEN** the same pupil is fixed at Darken Amount 0 and at 100
- **THEN** the pupil is darker at 100

#### Scenario: Pupil Size widens the match

- **WHEN** a pixel whose red leads green and blue by 1.3× is fixed at Pupil Size 0 and at 100
- **THEN** it is left alone at 0 and changed at 100

### Requirement: Red Eye tool

The Red Eye tool SHALL fix the active pixel layer inside the box dragged over
an eye, and a click (a box under 2 px) SHALL use a 24 px box centred on the
click. Each fix that changes pixels SHALL record exactly one "Red Eye Tool"
history state; a box with no red SHALL record nothing. The options bar SHALL
offer Pupil Size and Darken Amount (1–100 %, default 50 each).

#### Scenario: Drag a box over the eye

- **WHEN** the `red_eye_tool` self-test drags a box over a red pupil on skin
- **THEN** one "Red Eye Tool" state is recorded, the pupil is no longer red, and the skin is unchanged

#### Scenario: A click on skin records nothing

- **WHEN** the tool is clicked where the 24 px box holds no red
- **THEN** no history state is recorded
