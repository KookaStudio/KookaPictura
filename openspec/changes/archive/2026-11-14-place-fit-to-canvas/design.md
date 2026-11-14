# Design

## Context

A placed smart object is rendered from its embedded source scaled into the
layer's rect (`compositing/smart-object-rendering`). The layer rect is the
transform the renderer understands; it can represent a uniform or per-axis
scale and a translation, but not a rotation, skew, or warp. The Free Transform
engine's raster path (`transform_layer`) resamples the layer and, for a
channel-less smart object, materializes and drops the object. That is correct
for rotation/skew/warp but wrong for the plain canvas fit and for a scale or
translation, where the source can stay untouched.

## Fit after the smart object exists

`pictura-render` gains:

- `fit_smart_object_to_canvas(doc, path) -> bool` — resolves a smart-object
  layer, takes its current rect as the source-native frame, computes
  `s = min(1, canvas_w / w, canvas_h / h)`, and sets a centred rect of
  `round(w*s) × round(h*s)` (at least 1 px per axis). It changes the layer rect
  only; the payload and `smart_object` are untouched. A source that already
  fits keeps its native rect (no upscale).
- `can_transform_smart_object_rect(doc, path) -> bool` and
  `transform_smart_object_rect(doc, path, rect) -> bool` — the non-destructive
  path used by the scale/translation commit. Eligibility: a non-group,
  non-adjustment, non-Background, position-unlocked, channel-less embedded smart
  object with a decodable source and no raster mask. Setting the rect does not
  touch the payload or the preserved blocks.

The Place bridges call the fit inside the same `edit("Place", …)` as the
placement, so the fit belongs to the one `"Place"` state. The PSD/PSB bridge
uses the channel-less layer created by `place_smart_object`. The raster bridge
converts the native-size raster, clears the raster proxy (the conversion left a
proxy so rendering was unchanged), then fits; the embedded source is the
full-resolution raster.

The engine `place_smart_object` keeps its documented primitive behaviour: it
appends at the source's native rect. The application-level fit is the transform
that follows it.

## Free Transform scale/translation

`commit_transform` already snapshots the session's `MapParams`, quad, warp, and
members. Before the resampling/deferred path, it detects the non-destructive
case:

- exactly one member, equal to the session target;
- `warp` is `None`, not projective/quad-driven;
- angle is zero, both skews are zero, both scales are positive;
- `can_transform_smart_object_rect` succeeds.

The destination rect is the axis-aligned integer bounding box of the quad
(`floor`/`ceil`, as the commit cost does). The commit calls
`transform_smart_object_rect` instead of `group_apply`, records one
`"Free Transform"` state through `commit_transform_edit`, and recomposites.
The smart object stays; the renderer scales the embedded source into the new
rect. The path is cheap, so it bypasses the deferred pixel-transform branch.

Rotation, skew, warp, negative scales, masks, linked sets, and every other
target keep the existing rasterizing path, including the channel-less
materialization rule. This is the agreed scope: the source retention is
guaranteed for the fit and for scale/translation, not for the raster modes.

## Preview and history

The Place fit is applied before the session begins, so the session's preview
and box start from the fitted rect. The fit is part of the placement edit; an
identity commit adds no history. A changed scale/translation commit adds one
`"Free Transform"` state, and a reset/replace still reads the original source.

## Tests

- `pictura-render` unit tests for the fit (landscape, portrait, no upscale) and
  for `transform_smart_object_rect` preserving the payload.
- `tst_place_fit` (Qt Test): PSD fit and session start, no upscale, scale-up
  keeps the smart object and the native source, raster fit, and the fit's Place
  state.
- Existing `tst_place_sequence` coverage remains for the Place sequencing and
  cancel rules. The canvas-drop fit row is manual: the drop router uses the same
  bridge and the self-test already covers the drop sequencing.

## Risks

- The smart-object renderer is nearest-neighbour into the rect; the fit is as
  sharp as the existing placed-source render. A future bilinear/transform
  renderer can replace the rect model without changing this contract.
- The scale/translation path only works when the layer has no raster mask; a
  masked placed object falls back to the destructive transform, which is the
  current behaviour.
