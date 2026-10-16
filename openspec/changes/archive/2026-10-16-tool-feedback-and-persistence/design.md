# Design

## Persistence (`tool_options_json.{h,cpp}`)

Each options struct lists its persisted fields once in a `fields(T&, F&&)`
template. A `Writer` visitor turns the list into a JSON object and a `Reader`
visitor fills a copy of the group's current value, so a missing key keeps its
default. Restoring uses `ToolOption::update(clampOptions(v))`, which clamps
like `set` but does not notify, as `CropOptions::apply` already does at
startup. `PaintOptions` and `SelectionOptions` are classes with setters. A
small field struct mirrors their getters, and restore goes back through the
clamping setters.

Groups are addressed through `ToolController::optionsFor<T>(owning tool)`, so
the controller does not grow. Blur, Sharpen, and Smudge (and Dodge, Burn, and
Sponge) each keep their own group and their own key.

The session is saved on close and quit (existing), plus on `activeToolChanged`
and the Move / Type / Shape option signals, so a crash keeps the last switch.

## Caret contrast

`TypeToolHandler::darkUnder` samples the document composite (`sample_argb`)
at five points along the caret segment. Transparency reads as the light
checkerboard (composited over white). The text being typed is not in the
composite, and a reopened layer is hidden while edited, so the sample is
always the image behind the text.

## Forced-lock badge

`LockBadgeRole` is `LockRole` without bits 0x03 for `type` rows and shape
rows. The row delegate, its hit test, and the badge test hooks read it. The
Lock strip, the filter, and the full-lock checks keep `LockRole`, so the locks
themselves are unchanged.

## Move feedback

- `pixelsPerRulerUnit` / `formatRulerLength` (canvas_ruler) are the rulers'
  own conversion, now shared. `CanvasScrollBars` forwards its unit and ppi
  provider to its `ImageView`, holding them for a canvas set later.
- `ImageView::offsetReadout` formats ΔX / ΔY (Percent against the document
  width / height). The existing drag-size hint now sizes to multi-line text.
- `ImageView::setMoveOutline` draws the content box with the selection's
  marching-ants pens and phase, translated by the move delta. The ants timer
  runs while an outline exists. `endMovePreview` clears it.
- The Move handler takes the content box from the warm layer image's alpha,
  cached by `QImage::cacheKey`, so a warm press does not rescan.

ponytail: the outline is the box round the content, not its contour (CS6 also
draws a box for the move bounds).

## The idle controls box

The controls box used to be a Free Transform session, begun (and the layer
trimmed to its content) whenever the box showed. With the box on by default,
that trimmed every layer the Move tool selected, so a filter run with the box
up could not spread past the content (the tilt-shift test caught it).

Now `syncTransformControls` only draws: `controls_box_extents` returns
`pictura_render::content_rect` (what `trim_to_content` would trim to,
computed without changing anything), and the canvas draws it through the same
Free Transform painter (`ImageView::setControlsBox`). The rect is cached per
(layer, rect, history step), so a refresh does not rescan. Hover cursors and
presses hit-test the cached rect (`controls_box_hit_test`, the session's own
`hit_test_quad`). A press on a handle starts the session for that gesture
(`beginControlsGesture`), and the release commits it and re-syncs to an idle
box. Any other press is the ordinary Move drag, with the box hidden until the
release. `untrim_to` still pads a layer back when a session that trimmed it
ends without a transform (an identity gesture, Esc on Ctrl+T).

Two side effects the new default exposed are fixed alongside it: the session no
longer takes keyboard focus unless it is the modal command, and Space pans
over the box.
