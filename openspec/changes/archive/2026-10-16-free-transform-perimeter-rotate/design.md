# Design: free-transform-perimeter-rotate

## Context

`hit_test_quad` (`crates/pictura-app/src/cxxqt_object/impl_transform/geometry.rs`)
classifies a press as handle (6 px), inside-move, or rotate (20 px around a
corner), else `-1`. The band loop tests only `dist2(p, corner)`. The press
routing (`tools.cpp::handlePressed`) already returns without committing on a
`-1` hit for the modal session, so the remaining defect is the missing band
coverage; the no-commit contract needs a regression test rather than a code
change.

## Goals / Non-Goals

**Goals:**
- Rotate from just outside any side, at any zoom, in `Free` mode.
- Pin the inert far-press behavior.
- Keep projective modes handle-only.

**Non-Goals:**
- Changing the cursor glyph or its orientation rule (already radial from the
  quad centre, which faces the grabbed side/corner).
- The Move tool's controls-box inside/outside press routing, which keeps its own
  suspend-and-move contract (`tools/move-transform-controls`).
- Crop/Slice rotate zones, which have their own bounded-ring math.

## Decisions

- **Segment distance, not a second corner loop.** Replace the corner loop with
  the minimum squared distance from the point to the four quad edges
  (`point_segment_dist2`, clamped projection). This subsumes the corner case
  (a corner is an endpoint of two segments) and gives the same 20 px band around
  the whole perimeter with one helper. The quad is always convex in `Free`
  mode (an affine image of the source rect), so outside points have a unique
  nearest edge and no inside/outside ambiguity.
- **Inside is still move; the band is strictly outside.** `point_in_quad` runs
  before the band, so a point on or inside an edge moves; only exterior points
  reach the band. That preserves every existing hit-test expectation.
- **Projective modes return before the band.** Distort/Perspective/Skew keep
  handle-only hit testing, matching their existing spec and CS6 (no rotate
  affordance outside a distort box).
- **The inert press is test-only.** `handlePressed` already returns for a
  modal-session `-1`; the new Qt Test presses 40 px beyond the band and asserts
  the session is still active with no history state, so a future routing change
  cannot silently turn it into a commit.

## Risks / Trade-offs

- [A band on the side competes with the Move tool's outside-press suspend when
  the controls box is shown] → The controls box uses the same `hit_test_quad`;
  a press within the band is a rotate gesture, outside it is the Move drag. The
  controls-box spec already names "in the rotate band" as a transform gesture.
- [A 20 px band around a tiny box overlaps the whole box] → Handles (6 px) and
  inside-move are tested first, so a small box stays usable; the band is screen
  pixels, so it never grows with zoom.

## Migration Plan

None: pure hit-test behavior, no persisted state.

## Open Questions

None.
