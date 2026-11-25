# Proposal

## Why

Clicking a History panel step 10–15 states back on a large document leaves the
canvas unchanged for seconds, with nothing to say Pictura is working (#1608).

Measured on a 21,000² document with 15 large brush strokes:

- In the headless app, the jump takes 9 ms on the GUI thread, and the GUI is
  free again within 875 ms.
- In a release profile, rebuilding the visible display tiles at fit zoom takes
  2.6 s on a worker (36 tiles at level 4). A larger window has more of them.

So the window is not frozen: the canvas is waiting for its tiles, and until they
land it shows the old pixels or reduced stand-ins. The same wait follows a zoom,
a pan to an unbuilt area, or a large edit.

## What Changes

- A small animated "Rendering…" indicator in a corner of the canvas, shown once
  the display tiles the view asked for have been pending for 250 ms, and hidden
  as soon as none are pending.
- The view reports whether display tiles are pending (`tiles_pending`), and the
  canvas reads it through its level provider.
- No timer runs while nothing is pending.

Making the tiles arrive sooner is tracked separately in #1610.

## Capabilities

### New Capabilities

- `ui/canvas-rendering-indicator`: when the canvas shows that it is still
  rendering, and when it stops.

### Modified Capabilities

None.

## Impact

- `crates/pictura-app/src/cxxqt_object/tiles.rs` and `cxxqt_object.rs`: the
  `tiles_pending` query.
- `crates/pictura-app/cpp/rendering_indicator.{h,cpp}` (new),
  `image_view.{h,cpp}` (hosts it, `LevelProvider::tilesPending`),
  `frame_canvas.cpp` (wires it).
- A new Qt Test suite, `tst_rendering_indicator`.
