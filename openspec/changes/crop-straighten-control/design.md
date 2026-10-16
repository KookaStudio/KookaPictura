# Design

## Box fit

The content is the document canvas rotated by the session angle about the
pivot (the box centre when the angle was set). Each box corner is
`centre + t * d`; mapped back through the inverse rotation it is
`centre' + t * R(-θ) d`, linear in `t`. The fit scale is the minimum over the
four corners and both axes of the `t` that keeps the point inside the canvas,
clamped to 1 (`fitBoxInRotated` in `crop_grip.h`). Scaling about the centre
keeps the ratio and the pivot, so the preview does not jump.

ponytail: CS6 may pick a larger off-centre box; centre-anchored scaling is the
approximation, with no oracle for Adobe's exact choice.

## Ctrl invocation

`ToolHandler::onModifiers` already sees every modifier change on the canvas.
The crop handler arms through `CropOptions::setStraightenMode`, so the button,
the cursor refresh, and the handler stay on the one existing notification path;
a `ctrlArmed_` flag remembers that Ctrl, not the button, armed it.
