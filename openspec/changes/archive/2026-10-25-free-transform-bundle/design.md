# Design

## Context

See `proposal.md` for motivation and the four delta specs for the behavior
contract. The relevant current state:

- A `TransformSession` holds a similarity (`scale_x`, `scale_y`, `angle`,
  `dx`, `dy`, a `pivot`, and H/V shear) plus, for the projective modes, a live
  `quad` and, for Warp, a source-space mesh. `MapParams` maps source to
  document as `M(p) = P + A·(p − P) + t`.
- `transform_press` / `transform_move` do not carry Ctrl. The only transient
  projective gesture is `transform_begin_free_perspective`, a separate bridge
  call from `tools.cpp` after the press, which sets `base_quad`; the commit
  then routes through the projective quad op.
- The pivot is stored as a document point and displayed at that point, but
  `M(P) = P + t`, so a move or scale leaves the marker behind while later
  rotations turn about `P + t`.
- `transform_leave_warp` keeps `session.warp`; the commit and preview treat
  `warp.is_some()` as holding a mesh. Leaving also moves the box to the warped
  preview's document bounds and stores a source-space `warp_post` that keeps the
  preview pixels in place.
- The transform box is always `session_quad`; in Warp mode the canvas draws the
  mesh grid separately, and the reference point is hidden.
- The live preview is a cached base plus a moving layer; the projective
  preview matrix is identity whenever a warp exists, because the warp preview
  layer already carries the frame and its post-frame.

## Goals / Non-Goals

**Goals:**

- One commit per session, one history state, no document mutation before
  commit.
- Gesture and mode switches preserve the full session state until Apply or
  Escape.
- Keep the existing bridge boundaries: do not widen `transform_press` /
  `transform_move` across their call sites.

**Non-Goals:**

- Switching from a box with a stored warp to `Skew`, `Distort`, or `Perspective`
  via the Transform menu; those commands restart the session today and keep
  doing so.
- Guide snapping in the new Free distort shortcut (the Distort mode's snapping
  is keyed to the session mode name and is not reused).
- Re-warping the preview when only the interpolation changes; commit already
  uses the new method and the next warp or box edit refreshes the preview.
- Warp retention on the Move tool's per-gesture controls session, which commits
  on release; this change targets the modal Free Transform session.
- Puppet Warp, Content-Aware Scale, or split warps.
- Changing the warp resampler or its full-resolution preview cost.

## Decisions

### 1. Transient Ctrl gestures reuse `base_quad`

Add `transform_begin_free_distort` and `transform_begin_free_skew` beside
`transform_begin_free_perspective`, all in `impl_transform/bridge.rs`. The C++
press path decides the modifier precedence:

1. `Ctrl+Alt+Shift` on a corner → existing perspective shortcut.
2. `Ctrl+Shift` on a side handle → free skew.
3. `Ctrl` on a corner → free distort.

Each begin function records a transient flag on the session. Every press clears
the previous transient flags; on release the active flag clears but `base_quad`
stays, so later plain drags use `gesture_scale_base` (scale in the box's
projective frame) and the session commits through the quad op, exactly as the
perspective shortcut does.

To avoid duplicating the gesture math, factor the quad computation out of
`gesture_distort` and `gesture_skew` into pure helpers
(`distort_quad(start, corner, delta)` and
`skew_quad(start, handle, delta, alt, type_layer)`). The free path then:

1. Computes the candidate quad from `start_quad` and the press delta.
2. For distort, carries the reference point with the projective map from
   `start_quad` to the candidate and calls `set_pivot` with the carried
   document point.
3. Pulls the candidate back through the similarity (`unmap_quad`, moved from
   `perspective.rs` to `geometry.rs`) into `base_quad`.

Alternatives considered:

- Widening `transform_press` / `transform_move` with a Ctrl flag touches every
  call site for a value only the press needs. A separate bridge call matches
  the existing perspective shortcut and keeps the ABI stable.
- Setting `session.quad` directly would break the composition: a Free session
  with a `base_quad` maps the base through the similarity; a raw quad bypasses
  that and later plain drags would stop scaling the box.

Type policy: the distort shortcut refuses editable type (falls back to an
ordinary scale), as the perspective shortcut does. The skew shortcut stays
available because its affine quad is re-set as type by the engine.

### 2. The pivot is a source-frame point

Treat `session.pivot` as the reference point in the item's **source** frame.
The document position is `pivot + (dx, dy)` (`M(P)`); no new field is needed.

- Readouts (`transform_pivot_x/y`, the canvas marker, X/Y fields) add the
  session translation. In projective modes `dx = dy = 0`, so the same
  expression is correct there.
- `set_pivot(x, y)` inverse-maps:
  `P' = P + A⁻¹·((x, y) − P − t)`, then
  `t' = t + (P − A·P) − (P' − A·P')`. The map is unchanged and `M'(P')` is the
  requested document position. A singular linear part refuses the set. In a
  projective mode the pivot is set directly.
- `pivot_hit` compares against `pivot + (dx, dy)`.
- `transform_press` captures `start_pivot` as the document position.
- Translation and scale then carry the marker without gesture code: both
  change `t`, and the marker is `P + t`. Rotation already turns about `P + t`
  and leaves it fixed.
- `gesture_scale_base` changes only `base_quad`, so it carries the marker
  explicitly with the projective `carry_pivot(start_quad, new_quad,
  start_pivot)` and `set_pivot`.
- `repeatable_transform` records `pivot − centre(orig_rect)`. Because `pivot`
  is now a source point, the automatic carry is not stored as an offset; the
  Transform Again session places the new pivot at the new source centre plus
  the user's original offset.

Alternatives considered:

- Keep the pivot in document space and re-base it after every gesture. That
  needs a full press-time map snapshot (`start` currently stores only the
  scalar and translation values), and rotation would still mix the stored
  point with `P + t`. The source-frame reading is the smaller root-cause fix.
- Add a second `pivot_source` field next to a document getter. That is two
  copies of one value with no owner.

### 3. Warp stays in the session; a flag says who owns the UI

Add `warp_editing: bool` to `TransformSession`.

- `warp.is_some()` means the session holds a mesh: the commit applies it and
  the preview drawing treats the moving layer as already transformed.
- `warp_editing` means the mesh overlay, warp gestures, and warp options page
  are active. `transform_warp_active` returns this flag; a new
  `transform_warp_stored` exposes `warp.is_some()` for the C++ preview choice.
- `transform_leave_warp` keeps the mesh, clears the drag state, sets
  `warp_editing = false`, moves the box to the warped preview's document bounds,
  and stores `warp_post = new_frame⁻¹ · old_effective_frame`. The preview pixels
  do not change; only the box and the post-frame move.
- `transform_enter_warp` sets `warp_editing = true`; it creates the identity
  mesh only when `warp` is `None` (and clears `warp_post`), otherwise it reuses
  the mesh, style, bend, distortion, and post-frame.
- `warp_post` is applied after the mesh, before the box frame. The preview and
  commit pass `warp_preview_quad = (box_frame · post)(source box)` to
  `transform_layer_warp`; the grid, hit-testing, and drag math apply the same
  post-frame (and its inverse for a drag delta).
- Dispatch gates warp gestures on `warp_editing`. After any box gesture that
  changes the box and has `warp.is_some()`, it calls `refresh_warp_preview`
  (which warps through the current `session_quad` and `warp_post`).
- The numeric setters and exact rotate/flip commands re-warp after they change
  the session when `warp.is_some()`; otherwise the preview matrix would stay
  identity and the preview would be stale.
- `transform_quad_extents` always returns `session_quad`; the box does not jump
  to the mesh anchors in Warp mode. The canvas draws the grid from the mesh and
  post-frame separately.
- The C++ `updateTransformOverlay` uses `transform_warp_stored` to choose the
  whole-preview path and `transform_warp_active` for the grid and the hidden
  pivot; the options bar keeps using `transform_warp_active` for its warp page.
- `transform_repeat` stays disabled when any warp is stored; `can_set_mode` and
  the transient begin functions gate on `warp_editing` rather than
  `warp.is_some()`.
- `transform_apply` keeps the projective (live-quad) branch whenever the mode is
  projective, warp stored or not; otherwise it edits the similarity. When a warp
  is stored it re-warps the preview afterwards.
- `ToolController::resetTransformUi` clears the canvas transform preview, smart
  guides, and controls box, and `PictureViewRust::clear_transform_preview` drops
  the prepared preview fields. `onSessionCancelled` calls it so Undo/Redo and
  document swaps cannot leave a stale box or warp grid over a restored layer.

Cost: a box drag with a stored warp re-warps the full layer on every move. This
is the existing Warp-mode preview cost, now also paid in box mode; it is marked
with a `ponytail:` comment and is the first candidate for a downsampled
preview.

### 4. Land order

1. Pivot representation (#903): the model change, with its unit and Qt tests.
2. Ctrl gestures (#901): they consume the carried pivot for the distort path.
3. Warp retention (#902): independent of the other two and the riskiest for
   preview behavior.

One OpenSpec change, three commits, one pull request. Each commit carries its
issue number.

## Risks / Trade-offs

- **Full-resolution re-warp per box move** → Reuse the existing preview path
  and mark the ceiling; downsample only if a real lag report appears.
- **Box versus warped bounds** → The box now follows the warped preview's
  document bounds. A source-space post-frame keeps the rendered pixels fixed;
  the mesh and preset style stay in place. Re-fitting the mesh to the box would
  be a smaller diff but would visibly move the image on every toggle.
- **Modifier precedence** → `Ctrl+Alt+Shift` wins over `Ctrl+Shift` and `Ctrl`;
  `Ctrl+Shift` on a side wins over `Ctrl`; `Ctrl` on a side without Shift keeps
  scaling; `Ctrl+Shift` on a corner stays distort. Tests exercise each row of
  that table.
- **No guide snap in the free distort** → The Distort mode's snap is keyed to
  the session mode string. Adding a transient-state accessor to the C++ press
  path is possible but not required by #901; record it as a known gap.
- **Pivot inverse on a singular map** → The setter refuses instead of moving
  the marker to an undefined preimage.
- **Free skew on editable type** → The commit relies on the engine's affine
  quad path to re-set the type. A projective quad would rasterize it; the
  shortcut is affine by construction.
- **Existing warp tests encode the old drop-on-leave rule** → Update the
  toggle/carry assertions and add the retention cases rather than weakening
  them.

## Migration Plan

No document or file-format change. The change is code plus tests; the OpenSpec
deltas are archived with the implementation. Rollback is a revert of the three
commits. The behavioral change is limited to sessions started after the fix;
no saved state is reinterpreted.

## Open Questions

None. The Ctrl mapping, type policy, box representation, and mutation-path
coverage were settled before planning.
