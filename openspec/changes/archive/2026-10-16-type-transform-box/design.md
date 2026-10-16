# Design

## `TypeFrame` (type_frame.{h,cpp})

The session's placement moved out of `TypeToolHandler` into a value class. It
holds the origin, the `TySh` linear matrix, paragraph type's box, point type's
Ctrl box, and the reference point, plus the hit tests and the four gestures.
The handler keeps the text-field behaviour and wires pointer events to it. That
also keeps `tool_type.cpp` under its size target.

- **Resize** maps the pointer back through the matrix at the gesture's start
  and moves the grabbed edges (at least 1 px a side), as before.
- **Scale** multiplies the matrix columns by the grabbed edges' stretch (at
  least 1%) and shifts the origin so the opposite edge or corner stays put.
- **Rotate** left-multiplies the matrix by the pointer's angle about the
  reference point and turns the origin about it.
- **Pivot** moves the reference point. It defaults to the box centre, and a
  text move carries a placed pivot along.

The rotate band is outside the box, off its handles, within 24 screen pixels
of an edge. Beyond it the pointer is the earlier text move. The handles take 5
screen pixels and the reference point 6.

## Point type's Ctrl box

`type_local_bounds` gives point type's bounds in its own frame
(`type_placement(spec).bounds`, before the matrix). The handler tracks Ctrl
from `onModifiers` (canvas key events) and from the live modifiers on every
hover, so the box appears whichever widget has focus.

## Readouts

`ImageView::measureReadout` formats two labelled lengths in the rulers' unit,
and `offsetReadout` now uses it. The angle is the matrix's own rotation
(`atan2(xy, xx)`), clockwise positive on screen as CS6 shows it.

ponytail: no Shift constraints (proportional scale, 15° steps), no skew, and
cursors are not turned with the box. The committed layer's Free Transform has
all three.
