# Design — Skew transform gestures

## Context

The Free Transform session already stores a live target quad for `Skew`,
`Distort`, and `Perspective` (`TransformSession::quad`, anchored at press in
`start_quad`). Hit-testing returns corner handles `0..=3` and edge midpoints
`4..=7`. The Skew dispatch previously handled only edges and projected the raw
pointer delta onto both axes; corners fell through to `false`. The C++ shell
already passes Shift and Alt into `transform_move`, but the Rust side ignored
Alt.

## Gesture math

Port the Skew branch from photocraft's `apply_mode_drag`:

- **Corner**: take the two adjacent edge vectors `corner − neighbor` (top/bottom
  and left/right). Project the press-to-pointer delta onto each unit vector and
  choose the larger projection. Move the corner by that projected vector. Alt
  moves the opposite corner by its negation.
- **Edge**: project the delta onto the edge vector itself and translate both
  endpoints by that projected vector. Alt moves the opposite edge by its
  negation. This keeps top/bottom edges horizontal and left/right edges
  vertical, regardless of Shift.
- **Type layer**: move the corner's whole followed edge (corner and its neighbor
  along the chosen axis); Alt moves the opposite edge the other way. This keeps
  the quad a parallelogram so `affine_quad` can fold it into `TySh` and the text
  stays editable.
- **Convex clamp**: for corner drags, interpolate from the start quad to the
  target and keep the furthest convex result, matching photocraft's
  `keep_convex`. Degenerate results still go through the existing
  `store_quad` refusal.

The session gains `type_layer: bool`, set from `Layer::is_type()` when the
session begins (after any selection lift, so a lifted floating layer is not
treated as type).

## What stays the same

- Distort and Perspective gestures.
- Hit-testing (handle-only in projective modes).
- The reference point is not carried with the quad: handle drags leave it in
  place, as the reference-point spec requires.
- The options bar's derived H/V readouts and numeric shear setters.
- Commit/cancel and history behavior.

## Verification

Rust unit tests cover the gesture vectors, Alt mirroring, the type parallelogram,
and the convex clamp. `tst_transform_skew` exercises the same behavior through
the `PictureView` press/move bridge (including Alt plumbing). The existing C++
self-test's Skew check is updated from the old diagonal-edge expectation to the
axis-constrained one.
