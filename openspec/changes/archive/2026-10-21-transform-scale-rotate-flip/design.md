# Design

## Session reuse

Every command in the group uses the existing Free Transform session rather than
a second transform pipeline. Beginning a session already handles the hard parts:
trimming the layer to its content, lifting a selection onto a floating layer,
building the preview, and committing one resampled op. The exact commands begin
a `Free` session, set the requested similarity parameters, commit immediately,
and record a command-specific history label.

## Constrained modes

`TransformMode` gains `Scale` and `Rotate`; both remain similarity modes (not
projective). The mode only changes hit testing and gesture dispatch:

- `Scale`: the eight handles call `gesture_scale`; the rotate band returns no
  hit, so a press outside the box grabs nothing. Moving inside still translates.
- `Rotate`: every handle and the rotate band return the rotate handle, so a
  handle drag calls `gesture_rotate`; moving inside still translates.

Choosing Scale or Rotate while a similarity session is open changes the mode in
place, keeping the live box. Projective and Warp sessions refuse the switch.

## Exact commands

`transform_apply` maps the command to the session state:

- non-projective session: add ±90°/180° to `angle`, or negate `scale_x` /
  `scale_y` for a flip;
- projective session: rotate or reflect the live quad about the reference point;
- Warp mode: the box's similarity frame rotates/flips, the mesh stays in source
  space.

With no session, the C++ handler begins one, applies the op, sets the history
label, and commits. With a session open, it applies the op and refreshes the
overlay without committing.

## Transform Again

A successful similarity commit stores `RepeatTransform`: scale, angle, shear,
translation, and the reference point's offset from the source rect's centre.
`transform_repeat` recreates that state on the new session, anchored to the new
target's centre. Projective and Warp commits clear the memory, so Again is
disabled after them. The memory is per view and cleared when a document is
loaded or created.

## History labels

`TransformSession` carries a `history_label` (default `Free Transform`). The
immediate commands set `Rotate 180°`, `Rotate 90° Clockwise`, `Rotate 90°
Counter Clockwise`, `Flip Horizontal`, `Flip Vertical`, or `Transform Again`
before committing. Interactive sessions keep `Free Transform`.

## Menu

Frozen command ids replace the path-derived leaves. The command table adds the
screenshot's separators after Again, after Warp, and after the quarter turns.
Labels use the screenshot's full words (`Rotate 90° Clockwise`) rather than the
abbreviated `CW` form.
