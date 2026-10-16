# Proposal: show-transform-controls

## Why

Issue #296 follow-up: the Move tool's `Show Transform Controls` checkbox is
recorded but inert (`options_bar_move.cpp` carries a `ponytail:` note saying so).
CS6 shows the Free Transform bounding box and handles on the active layer while
the box is checked, and dragging a handle transforms the layer directly, so the
user does not need `Ctrl+T` for a quick scale or rotate.

## What Changes

- With the Move tool active, `Show Transform Controls` on, no active selection,
  and a transformable active layer, the application SHALL show the Free
  Transform bounding box and its eight handles on that layer (without the
  reference-point marker, which CS6 reserves for the modal command).
- The box SHALL follow the active layer: a different layer, a content change
  (including undo), a document switch, or the checkbox unchecking SHALL rebuild
  or hide it.
- A handle or rotate-band drag SHALL transform the layer and apply on release as
  one `"Free Transform"` history state, with the box returning afterwards
  (there is no modal commit step); an identity drag records nothing.
- A press inside or outside the box SHALL suspend the box and run the ordinary
  Move drag (translation, Alt-duplicate, cross-document drag), which records its
  `"Move Layer"` state; the box returns when the drag ends.
- The options bar SHALL stay on the Move page while the box is shown; `Ctrl+T`
  SHALL take the box over as the modal Free Transform session.

## Capabilities

### New Capabilities

- `tools/move-transform-controls`: the Show Transform Controls box, its
  gestures, and its lifecycle.

## Impact

- `pictura-app` (ToolController session orchestration, the Move options bar
  checkbox, the frame refresh hook) and a Qt Test suite. No engine or bridge
  contract change beyond one session-rect readout. No new dependency.
- Ceilings (`ponytail:`): an active selection suppresses the box (the lift
  session would mutate the document on show); the box hides during an ordinary
  Move drag rather than staying up; the translation drag inside the box is the
  Move tool's, not the session's.
