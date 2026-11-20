# Spec Delta

## ADDED Requirements

### Requirement: X/Y slider span

The X and Y fields' slider popups SHALL use a coarse span of about 20,000 px
centred on the typed range's midpoint, so one slider step changes the position
by about 100 px, while the typed fields SHALL still accept the full document
range. A value outside the slider span SHALL park the slider at the nearest end
without changing the typed value.

#### Scenario: A slider step moves about 100 px [tob_xy_slider_span]

- **WHEN** the X slider is moved by one step and then the X field is typed a value outside the slider span
- **THEN** X changes by about 100 px for the step, and the typed value still moves the target beyond the slider span
