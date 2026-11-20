# Design

## Context

`NumericField` builds its popup slider over `NumericFieldConfig::minimum..maximum`.
The X/Y fields accept ±1,000,000 px for typed document coordinates, so their
slider covers the same 2,000,000-unit range. With a popup slider around 180 px
wide, one slider pixel is about 11,000 px.

## Goals / Non-Goals

**Goals:**

- About 100 px per slider step for X and Y.
- The typed fields keep the full ±1,000,000 px range.

**Non-Goals:**

- Changing the W/H split slider.
- Changing the typed X/Y step or validation.
- A slider that covers the full typed range at fine resolution.

## Decisions

Add `NumericFieldConfig::sliderSpan` (0 = the typed range). When it is positive,
the slider range is `midpoint ± sliderSpan/2`, its single step is
`sliderSpan/200` (100 px for 20,000), and its page step is `sliderSpan/20`
(1,000 px). The X/Y fields set `sliderSpan = 20000`, giving a ±10,000 slider and
a 100 px step while the typed field remains ±1,000,000.

`syncSlider` still writes the value; the slider clamps a value outside its span
to the nearest end, so the handle reflects that the value is beyond the coarse
range without changing the typed value.

## Risks

- A value outside ±10,000 parks the slider at an end; that is intended for a
  coarse control.
- The mouse movement per pixel is about `sliderSpan / sliderWidth` (~110 px at
  180 px), while the keyboard step is exactly 100 px.
