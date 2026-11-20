# Design

## Context

The transform options page has a 3×3 locator (`optionsPivot0..8`) and X/Y/W/H
`NumericField`s. `setTransformPivot` deliberately moves the reference point
while keeping the rendered quad fixed, which is correct for the locator and the
canvas pivot drag but wrong for the X/Y fields: Photoshop's X/Y fields are the
reference point's document position and moving them translates the object. The
locator cells are created with a fixed icon and never refreshed. The numeric
field popup slider is linear over `NumericFieldConfig::minimum..maximum`, and the
W/H fields use 0.1..10000.

## Goals / Non-Goals

**Goals:**

- Highlight the locator cell that holds the reference point.
- X/Y translate the target so the reference point lands at the typed position,
  with the preview live before Apply; relative mode uses a fixed session origin.
- A split W/H slider: midpoint = 100%, maximum = 1000%.

**Non-Goals:**

- Changing the locator/pivot-drag behavior (the quad stays fixed when the
  reference point moves).
- Changing W/H handle scaling or the session's scale range beyond the typed
  1000% ceiling.
- Reworking the other numeric fields' sliders.

## Decisions

### Pivot anchor readout

`geometry::pivot_preset` compares the live document pivot with
`quad_anchors(&session_quad(session))` using a tolerance scaled to the source
rect diagonal. It returns 0..8 or -1. The bridge exposes it read-only; the
options page sets each cell's `pivotActive` property and re-draws its icon with
the filled centre for the active cell. A pivot between anchors highlights none.

### X/Y translation

`geometry::translate_to` computes `delta = target − pivot_point(session)`. For a
stored-quad session (`session.quad` is `Some`, as in Distort/Perspective) it
translates every quad corner and the pivot. Otherwise it adds the delta to
`dx/dy`, which also translates a base-quad session's mapped quad. The bridge
exposes `transform_set_position`; the X/Y fields call it instead of the
pivot-only setter.

Relative positioning uses `pivot_origin` directly as the fixed session origin
(not `pivot_origin + dx/dy`), so typing a relative value translates the object
by that offset and the readout is the object's offset from the origin. The
locator and pivot drag still use `set_pivot` and keep the quad fixed.

### Split slider

`NumericFieldConfig` gains `splitSlider` and `sliderMidpoint`. A split slider
uses the integer range 0..1000: position 0..500 maps linearly to
`minimum..sliderMidpoint`, and 500..1000 maps linearly to
`sliderMidpoint..maximum`. The field converts both ways in `syncSlider` and the
slider's `valueChanged` handler. W/H configure `maximum = 1000`,
`splitSlider = true`, `sliderMidpoint = 100`. Typed values clamp at 1000; the
slider's midpoint is exactly 100%.

## Risks

- The X/Y change alters the relative-positioning readout after a translation;
  the existing relative test is updated to the fixed-origin semantics.
- Projective-mode translation must move both the stored quad and the pivot, or
  the reference point would detach from the object.
- The slider mapping must round-trip so `setValue` and a slider drag agree.
