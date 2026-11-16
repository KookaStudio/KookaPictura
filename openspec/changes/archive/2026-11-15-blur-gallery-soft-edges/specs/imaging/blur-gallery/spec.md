# Spec Delta — imaging/blur-gallery

## ADDED Requirements

### Requirement: Blur Effects render order and soft Light Range

The Blur Effects' Light Bokeh and Bokeh Color SHALL be applied to the source
colour **before** the variable blur, gated by the same per-pixel field weight
the blur uses, so a blurred high-contrast edge keeps the unlit blur's shape
instead of gaining a bright rim. A pixel the field leaves sharp SHALL NOT
change. The Noise pass SHALL remain after the blur, gated by the same field
weight. The Light Range SHALL be a soft tonal window that reaches zero at both
bounds, so raising Light Bokeh across a tonal ramp SHALL NOT add a step at a
range bound.

#### Scenario: A blurred edge keeps its shape [blur_bokeh_no_rim]

- **WHEN** Light Bokeh is raised on a dark midtone subject against a bright
  background with a single blur pin
- **THEN** the blurred edge is not brightened relative to the unlit blur more
  than the subject's interior is, so no bright rim appears

#### Scenario: The Light Range bound is soft [blur_light_range_soft]

- **WHEN** a tonal ramp crosses the Light Range's upper bound while Light Bokeh
  is raised
- **THEN** the output has no tonal step at the bound
