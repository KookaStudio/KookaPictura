# Design

## The contract under test

`PanelColumn::moveFloat` clamps a top-level float to the available geometry of
the screen under the target point, falling back to the window's screen and then
to the primary screen, and never to the main window's rect
(`crates/pictura-app/cpp/panels/panel_column_float.cpp`). The check has to
observe that clamp without depending on anything the platform decides.

## Why the band precondition had to go

The check asked for a band between the main window and the screen edge so that
a float clamped to the screen could also be outside the window. The main
window's minimum size (787-803 x 839-841 px) exceeds the 800x800 offscreen
screen in the vertical direction on every platform, so:

- a bottom band can never exist, whatever the window's size;
- a right band exists only when the platform's font metrics keep the window
  narrower than the screen, which is the 7 px difference between Ubuntu (793 px)
  and Fedora (803 px).

No resize makes the window smaller than its minimum, so the precondition could
not be repaired by placing the window differently either. It was a property of
the platform's fonts, and the check's result flipped with it.

The "the float sits outside the window rect" assertion inherits the same
problem: with the window covering the screen, no position inside the screen is
outside the window.

## The replacement assertion

The check moves the float past the screen's bottom-right corner and looks at
where it lands:

```cpp
const QRect screen = column->floatHostRectForTest();
onScreen = column->floatClampedForTest(index, screen.bottomRight() + QPoint(400, 400));
const QRect after = column->floatGeometryForTest(index);
onScreenEdge = after.bottom() == screen.bottom() && after.right() == screen.right();
```

Two properties, both frame-independent:

- `onScreen` — the float came back inside the screen's available geometry. It
  fails when the clamp is removed (the float stays where it was asked to go,
  off-screen) and when the float is larger than the screen.
- `onScreenEdge` — the float rests on the screen's bottom-right edge, which is
  where the clamp puts it. A window-relative clamp puts the float on the
  window's edge instead, and the two differ whatever the window's size or
  position, because the window is never exactly the screen.

The request point is the screen's bottom-right corner plus 400 px in both
directions, the same shape the sibling `m42_float_overlay` check (136) uses, so
the two checks agree on where "past the screen" is. Off the offscreen screen
`QGuiApplication::screenAt` returns null and `moveFloat` falls back to the
window's screen, which is the screen the check measures.

The window stays at the screen origin for the duration of the check and its
previous position is restored afterwards, so the check's setup is explicit and
the rest of the self-test sees the window where it was.

## What is deliberately not covered

That a float may sit outside the main window's rect cannot be observed on the
offscreen screen, whose height is below the window's minimum height. The
requirement stays in `ui/panel-column` as a "may"; the offscreen self-test
covers the half of the contract that is observable.
