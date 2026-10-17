# Design

## Blink timer lives in the canvas

`ImageView` owns a 750 ms `QTimer`. While a type overlay is active, each tick
flips `typeCaretShown_` and repaints. `setTypeOverlay` runs on every edit,
caret move, and click, so it resets the phase to *shown* and restarts the
timer. The caret then never disappears under a keystroke. `clearTypeOverlay`
stops the timer. The tool handler does not track the phase, so every Type tool
and the mask tools get the same blink.

Only the vertical line blinks. The placement point and the baseline guides are
persistent overlay geometry, matching the requested behaviour (and CS6, whose
baseline marker does not blink).

## Geometry from the engine

`type_baselines(spec)` returns one `[x0, y0, x1, y1]` per laid-out line in
document pixels after `spec.matrix`. Horizontal type gets the baseline from the
line's first caret stop to its last. Vertical type gets the column's centre axis
from its first cell to its last. It walks the same line set as
`type_caret_stops`, so a guide always ends at the caret stop it is drawn under.
A blank line has no guide.

The placement point is `spec.origin`, which the handler already holds. It is
drawn as a 3×3 device-pixel square (the line is one device pixel), so it stays
the same size at any zoom.

## Tests

The canvas exposes `typeCaretShownForTest()`, `typeCaretBlinkIntervalForTest()`,
`typeOverlayPlacementForTest()`, and `typeOverlayBaselinesForTest()`. The Qt
test waits on the real timer with `QTRY_VERIFY` (bounded, offscreen-safe).
