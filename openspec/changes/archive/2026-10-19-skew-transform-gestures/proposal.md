# Skew transform gestures

## Why

`Edit > Transform > Skew` does not skew the way Photoshop does (#747). A corner
handle currently does nothing, and an edge handle follows the pointer in both
axes, so the box drifts off its axis instead of sliding. The owner's reference
implementation in photocraft has the intended Skew gesture math.

## What Changes

- Port photocraft's Skew gesture algorithm:
  - a corner moves alone along whichever of its two adjacent edges the drag
    follows most;
  - an edge slides its two endpoints only along the edge's own axis;
  - Alt moves the opposite corner or edge by the same displacement in the
    opposite direction;
  - Shift has no effect in Skew;
  - a corner drag stops where the quad would become concave or fold;
  - an editable type layer keeps a parallelogram by sliding the followed edge.
- Thread the existing Alt modifier through the transform dispatch; it was
  already delivered by the C++ shell but ignored by Rust.
- Keep the options bar's H/V readouts and numeric setters unchanged.

## Impact

- Capability: `document/free-transform` (MODIFIED requirement).
- Code: `crates/pictura-app/src/cxxqt_object/impl_transform/{geometry,dispatch,session}.rs`,
  `crates/pictura-app/src/cxxqt_object/state.rs`.
- Tests: Rust unit tests for the gesture math, a new `tst_transform_skew` Qt
  Test suite, and updated C++ self-test expectations for the axis constraint.
