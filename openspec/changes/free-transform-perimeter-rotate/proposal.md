# Proposal: free-transform-perimeter-rotate

## Why

Issue #310: outside the Free Transform bounding box there is a thick margin that
is neither the move zone nor the rotate zone. The rotate hit-test only accepts a
20 screen-pixel band around the four **corners**, so hovering just outside an
edge midpoint shows the default arrow and the canvas looks clickable; CS6 shows
the curved rotate cursor around the whole perimeter and rotates from any side.
A stray press beyond the band must stay inert (the session commits on
Enter/Apply and cancels on Escape).

## What Changes

- The rotate band SHALL follow the whole bounding quad perimeter: a press
  outside the quad within 20 screen pixels of **any** side rotates about the
  reference point, and the rotate cursor is oriented to the grabbed side or
  corner.
- A press beyond the band SHALL be inert: it must not move, rotate, commit, or
  cancel the session (the current early return is pinned by a regression test).

## Capabilities

### Modified Capabilities

- `document/free-transform`: the bounding-box hit-testing requirement's rotate
  band grows from corners-only to the whole perimeter, and the beyond-band press
  is specified as inert.

## Impact

- `crates/pictura-app/src/cxxqt_object/impl_transform/geometry.rs` (hit test and
  its unit tests), `crates/pictura-app/cpp/tests/tst_transform_options.cpp`
  (edge rotate + inert far press). No new dependency.
- The cursor orientation already follows the pointer from the quad centre, so no
  cursor change is needed; only the hit zone grows.
