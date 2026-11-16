# Design

## Zoom model

The canvas stores `zoom_` as screen pixels per **document** pixel, so the
field's percentage matches the picture, not the proxy image. A value of 0 is
the fit view: `zoom()` resolves it through `fitZoom()`, which scales the
document aspect into the widget's margin-adjusted room. The opening view is
therefore fit, as before, and a resize re-fits it.

The placement is derived from a focus fraction rather than a pixel offset:
`imageRect()` centres the document point `focus_` on the room's centre. This
keeps a resize meaningful and lets `zoomAbout` preserve the point under the
wheel.

## Anchoring

`zoomAbout(next, anchor)` reads the document fraction under `anchor` from the
current `imageRect()`, sets the zoom, then solves `focus_` so that fraction
lands under `anchor` again. `setZoom` anchors at the room's centre; the wheel
anchors at the pointer.

## Field and shortcuts

The dialog's options bar gains `−`, `+`, and an editable `QComboBox` at its
left (the bar is already the workspace's bottom row). The combo shows the live
percentage, lists the preset levels plus Fit in View / Actual Pixels, and
parses a typed percentage on `editingFinished`. The existing
`addZoomShortcuts` helper wires `Ctrl++` / `Ctrl+-` to the two buttons. A
`zoomChanged` signal keeps the field in step with wheel and resize changes
without writing over a field being edited.

## Commit

Pin positions are fractions and blur amounts are document pixels, so the zoom
never enters `values()`; OK commits exactly what it did before.
