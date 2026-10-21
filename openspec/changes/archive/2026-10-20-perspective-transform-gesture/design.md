# Design — Perspective transform gesture

## Gesture

`gesture_perspective` (new `impl_transform/perspective.rs`) works on the
press-time quad `S` and the press-to-pointer delta `d`:

- Corner `c` (0..=3): its horizontal partner `h` and vertical partner `v` are
  the other ends of its two edges (the same pairing Skew uses). For each partner
  `n`, `u_n = (S[c] − S[n]) / |S[c] − S[n]|` and `t_n = d · u_n`. The partner
  with the larger `|t_n|` wins; `S[c] += t·u` and `S[n] −= t·u`. Re-evaluated
  from `S` on every move, so the picked edge can switch mid-drag and the other
  edge snaps back.
- Edge (4..=7): the Skew edge slide without Alt.
- No convex clamp; `store_quad` refuses only a degenerate quad (non-finite,
  < 1 px², three consecutive collinear corners), keeping the last quad. A bowtie
  is a valid homography target; the engine and preview already draw only its
  front side (#748).

## Reference point

The recording's centre marker always sits at the corner average, including after
asymmetric edge slides, which matches a bilinear carry (the bilinear centre is
the corner average) and not the homography carry Distort uses. The pivot's
`(u, v)` in the press-time quad is found by a few Newton steps of the inverse
bilinear map, falling back to leaving the pivot when they do not converge, and
re-evaluated in the new quad.

## Free Transform shortcut

`TransformSession` gains `base_quad: Option<[(f64, f64); 4]>`, a source-space
quad after which the similarity `M` (scale, rotate, skew, move about the pivot)
applies. `session_quad` in `Free` returns `M(base)` when a base exists. Because
`M` is affine, `M ∘ H(rect → base)` is still a homography fixed by those four
corners.

- C++ calls a new bridge `transform_begin_free_perspective` right after a corner
  press when Ctrl+Alt+Shift are held. It refuses for type, non-`Free` sessions,
  and non-corner handles. On success it sets `free_perspective` for this drag
  and seeds `base_quad` with the source corners when absent.
- A perspective drag in `Free` runs the same gesture on the live quad, then sets
  `base = M⁻¹(quad)`; the pivot is carried in document space with `set_pivot`,
  which compensates the translation so the quad does not jump.
- A scale drag on a box with a base runs `gesture_scale_base`: the pointer is
  pulled back through the press-time homography to source space, the source
  rect scales about the opposite handle there, and the result maps forward
  again, so the opposite edge stays exactly in place (a similarity scale would
  slide a slanted edge). The similarity scalars are untouched; the base takes
  the new box.
- Commit, the preview matrix, and the local preview matrix take the projective
  path when `mode.is_projective() || base_quad.is_some()`.

## Readout

`transform_drag_angle_degrees(view)` returns NaN without a perspective drag.
The gesture records which corner moved, its unmoved neighbour across the picked
edge's side, and whether the edge was horizontal; the angle is the signed change
of the side direction `atan2` from `start_quad` to the live quad, wrapped to
(−180°, 180°], negated for top/bottom edges. C++ formats it as `∠: %.1f°` (the
recording's symbol is a skew glyph; a plain angle sign stands in) through the
existing `setDragSizeHint`, and clears it on release.

## Crossed (bowtie) box

A homography onto a self-intersecting quad puts the horizon through the source,
so its image wraps round the outside of the quad (the owner's screenshot). The
CS6 recording instead fills the quad's own outline: two straight-edged
triangles meeting at the crossing point. A bilinear map was tried first; it
only matches a symmetric bowtie, since for a lopsided one the row segments
sweep a curved (hyperbolic) waist (the owner's second screenshot).

- `crossed_quad::Hourglass` works in a frame where the side edges cross (a quad
  whose top and bottom edges cross is transposed into it). With the crossing
  point `X` at fractions `s` and `t` along the two side edges, the source splits
  at row `split = (s + t) / 2`. Rows above it fill triangle `(TL, TR, X)`:
  `p = (1 − λ)·lerp(TL, TR, u) + λ·X`, `λ = v / split`; rows below fill
  `(X, BR, BL)` the same way. Each triangle is filled exactly, so the edges are
  straight; the inverse reads `λ` and `u` off the barycentric weights.
- Engine: `transform_layer_quad_with` routes a crossed quad (side edges or
  top/bottom edges properly intersecting) to `crossed_quad::CrossedMap`.
  Convex and concave quads keep the homography.
- Preview: `warpTransformPreview` recovers the target corners from the
  homography it is given and uses the same map for a crossed quad, bounded by
  the corners. It flags the result `crossed` so an empty image never falls
  back to the projective `QTransform` draw.
- Inferred from a flat-colour recording: the shape matches, the pixel layout
  inside the triangles is not verified.
- The preview's ordinary projective path now divides by the inverse map's
  homogeneous divisor itself: `QTransform::map` clamps a negative divisor to a
  tiny positive one, so a front side behind `w < 0` (a concave target, the
  engine's horizon fixture) was never drawn. `tst_transform_distort`'s horizon
  check moved from a matrix whose projected corners cross (now an hourglass) to
  that concave fixture.

## Pending-transform lock and tool-switch prompt

Both are owner requests on #750 and apply to every modal transform, not just
Perspective.

- `ToolController::transformPending()` is a modal session that is not the
  Move tool's controls box (that box commits each gesture on release).
- `CommandRegistry::setBlockedProvider` is a gate over every command, applied
  in `refresh()` and in `dispatch()`. `frame_transform_lock.cpp` blocks the
  commands whose menu path starts with `Layer` while pending, and disables the
  Layers panel widget. Children keep their own enabled state, so re-enabling
  restores them. The lock is recomputed on `transformChanged` and in the
  frame's `refresh()`, which covers a session dropped by a document switch.
- `ToolController::setActiveTool` calls `resolvePendingTransform()` first.
  Every tool-switch route (tool strip, flyout, keyboard shortcut, control
  server) ends there. The prompt is a hook the frame fills with
  `askApplyTransform`. On Cancel the controller re-emits `activeToolChanged`
  with the current tool so the tool strip drops the click.
- `askApplyTransform` follows `askUnsaved`'s non-interactive pattern, but its
  default is non-interactive on the offscreen platform, so headless runs and
  Qt tests (which inject their own answers) never block on a modal. The
  control server turns it off explicitly. Non-interactive answers Don't Apply,
  which is today's behavior.
