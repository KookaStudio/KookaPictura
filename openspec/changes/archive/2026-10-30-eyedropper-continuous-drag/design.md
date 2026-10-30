# Design

## Drag sampling in the controller

`ToolController` already holds one `canvasSampler_`, armed by a dialog. It
sampled only in `handlePressed`. A new `canvasSampling_` flag is set when a
press with an armed sampler samples and is cleared on release and whenever the
sampler is replaced (`setCanvasSampler`). `handleMoved` calls the sampler while
the flag is set, then refreshes the cursor. Qt's implicit mouse grab delivers
the move and release events to the canvas even when the pointer leaves it, so
the flag cannot stick; the replace-time clear covers a dialog that closes
mid-gesture.

## The Eyedropper tool

`EyedropperToolHandler` gets the same shape: `onPress` samples and opens the
gesture, `onMove` samples while it is open, and `onRelease`/`onDeactivate`/
`onSessionCancelled` close it. It skips a sample when the rounded document
point has not changed, so a jitter within one pixel does not recomposite.

ponytail: a scope-aware sample (Sample Size > 1, or a scope other than All
Layers) still runs the compositor per distinct pixel; reading the maintained
composite for every scope is a later optimisation. Qt already coalesces mouse
moves, so this is the only throttle.

## The picker's default sampler

`ColorPickerDialog::setDefaultCanvasSampler(tools, colorAt)` stores a
frame-installed pair. `getColor` uses the explicit pair when the caller passes
one and the default pair otherwise. `sampleCanvas` leaves an already-armed
sampler in place, so a nested picker (Replace Color's Result Color swatch) does
not disarm its parent on close. The frame installs `activeView()->sample_argb`,
the maintained composite, so a sample is O(1). The stored functions capture the
window through a `QPointer`, so a destroyed window leaves them inert and a
picker opened in a test with no window is unaffected.

## Tests

A new `tst_eyedropper` suite drives the canvas with synthetic mouse events: the
Eyedropper tool's drag, and the Color panel swatch's picker (armed by the
default sampler) on click, drag, and release.
