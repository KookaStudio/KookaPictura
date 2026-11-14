# Design

## Context

See `proposal.md` for the motivation. The current state that shapes the
approach:

- `TransformSession` (`document_session/transform.rs`) is layer-oriented: it
  holds a path, member rects, a lifted-selection snapshot, and drives a moving
  image through `impl_transform/session.rs`, `preview.rs`, and `group.rs`.
- The gestures, hit-testing, pivot, numeric setters, mode switching, and exact
  rotate/flip commands read only `orig_rect`, the scalars, the quad, and the
  warp — not the layer.
- The selection is a canvas-sized tiled `u8` coverage mask
  (`pictura_select::Selection`). The committed 50 % contour is cached by
  `selection_revision` and exposed through `selection_api`.
- History already snapshots the selection: `Edit::Meta` is the path every
  selection command uses, and `History` restores the mask on undo.
- `pictura-render` owns the tiled resampler (`resample_plane`), the
  src-quad→dst-quad homography solver, and the tiled warp rasterizer
  (`warp_gray`, `warp_tiled`). All are private to
  `document_ops/layer_ops`; `pictura-render` already depends on
  `pictura-select`.
- The warp UI (mesh overlay, presets, custom points, surface drags) is driven
  by session state; only `refresh_warp_preview` and the commit touch the
  layer image.
- C++ starts a transform through `beginTransformPreview`, which requires a
  moving image and a backdrop patch.

## Goals / Non-Goals

**Goals:**

- One transform session with a `Layer` or `Selection` target, so gestures,
  options-bar wiring, mode switching, exact commands, and Warp stay shared.
- A mask commit proportional to the selection area, keeping the result tiled
  and clipped to the canvas.
- A live border preview that follows the gesture without resampling the mask
  on every pointer move.
- Warp on the border with the same mesh, presets, and composition rules as a
  layer warp.
- Byte-exact cancel and undo.

**Non-Goals:**

- Reworking the layer transform preview or its layer-specific commit.
- A vector, re-editable border transform (the mask is raster, as in CS6).
- The Move tool's `Show Transform Controls` path for a selection border.
- The right-click canvas context-menu entry.
- Storing anything new in PSD; a border transform is transient selection
  state.

## Decisions

### D1: Extend the session with a target discriminant

Add `target: TransformTarget { Layer, Selection }` to `TransformSession`.
A selection session has `path = ""`, `members = []`, `lifted = None`,
`untrimmed = []`, and `orig_rect = selection coverage bounds`. It skips
`compute_transform_preview` and never builds a moving image.

- Why: every gesture path already reads only the box/scalars/quad/pivot, and
  `ActiveSession` holds one transform slot. A parallel session type would
  duplicate the gesture, options-bar, and warp surface and complicate the
  slot's one-session invariant.
- Alternative rejected: a temporary transparent layer carrying the mask
  (would show a pixel preview, enter history/PSD, and need cleanup).

### D2: Commit through one composed map

For a non-warp commit, build
`Homography::from_quads(source_corners(orig_rect), session_quad(session))`.
For a warp commit, map each source point by `warp.map` and then by the
effective frame (the box quad, followed by `warp_post` when Warp was left).
`warp_preview_quad(session)` already returns the effective frame's box corners,
so the engine can solve the same frame the preview uses.

- Why: one map for the box, the ant preview, and the commit; a Distort or
  Perspective made before Warp composes exactly.
- The destination bbox is the transformed box bbox (the coverage cannot leave
  its bounds), clipped to the canvas.

### D3: Mask resampling lives in `pictura-render`

New `document_ops/layer_ops/selection_transform.rs` with public
`transform_selection` and `warp_selection`, reusing `solve_homography`,
`resample_plane`, `triangulate`, and `warp_gray`.

- Non-warp: resample the canvas-sized mask directly into the destination bbox;
  the tiled resampler reads only the source tiles the destination touches.
- Warp: crop the selection bounds to a bounds-sized plane, warp it with
  `warp_gray` (outside reads 0), then blit the warped plane at `t.dest`.
- Both return a canvas-sized `Selection` with zero tiles outside the result.

- Why: `pictura-select` cannot depend on `pictura-render` (render already
  depends on select), and the resampler is private there.
- Alternative rejected: exposing a generic plane resampler from render and
  implementing the op in select — extra public surface for no behavioural gain.

### D4: Preview from the cached contour

Add a Rust bridge that maps the cached committed contour through the session
map (homography, or `warp.map` + effective frame) and returns fractional
loops. C++ feeds them to the existing `ImageView::setSelectionPreview` on each
`updateTransformOverlay`. The committed contour is hidden for the duration of
the session and restored on session end.

- Why: a mask resample per pointer move would cost tens of milliseconds on a
  large selection; the contour is already cached and vector.
- The committed mask is re-contoured from the resampled coverage after commit,
  so the final outline is the mask's true 50 % line.

### D5: Overlay-only canvas start

Add `ImageView::beginTransformOverlay()` and branch in
`ToolController::beginTransformImpl` when the session target is a selection:
set the quad, pivot, warp overlay, and ant preview, but no moving image and no
projective image preview. `cancelFreeTransform`/`commitFreeTransform` keep
using `clearTransformPreview`/`endTransformOverlay`.

- Why: `beginTransformPreview` assumes a backdrop patch and a moving image;
  the border session has neither.

### D6: Warp without a moving image

- `refresh_warp_preview` returns true without building an image for a
  selection session; the C++ overlay update refreshes the ant preview.
- `transform_leave_warp` computes the warped bounds analytically
  (`warp_dest_rect`, the triangulation's destination bbox) and moves the box
  with a `warp_post` frame, mirroring the layer flow without rasterising.
- C++ skips `updateTransformPreviewLayer` for a selection session.
- `Edit > Transform > Warp` enters Warp on an open session instead of
  beginning a new layer session.

### D7: History and undo

Commit calls `set_selection(resampled)` then
`edit("Transform Selection", |_| Edit::Meta)`. `Edit::Meta` snapshots the
document and selection; undo restores the mask byte for byte. Identity and
cancel record nothing.

- Alternative rejected: inverse-resampling to undo — round-trip loss at
  fractional edges.

### D8: Modal lock and Transform Again

- The transform lock's blocked provider also disables the Select menu while a
  selection transform is pending (`tools_->transformPending()` plus a
  selection-target readout), so the mask cannot change under the session.
- `RepeatTransform` carries the target kind; a bridge readout lets the C++
  Again command begin a selection session when the last transform targeted the
  border and a selection is active. Projective and warped commits stay
  non-repeatable.

### D9: Catalogue and generated files

Flip `select.transform.selection` to `implemented: true`, add
`SelectTransformSelection` to the const-name table, regenerate
`command_ids.h`, and update the catalogue and const-name goldens.

## Risks / Trade-offs

- **Layer-preview coupling in C++** → the explicit selection branch in
  `beginTransformImpl` and `updateTransformOverlay`; tests assert no moving
  image and no image-preview matrix for a border session.
- **Double marching ants** → clear the committed contour while the session is
  active; the frame's `transformChanged` handler restores it when no session
  remains.
- **Large documents** → resample only the destination bbox, keep the output
  tiled, and gate the commit through the memory governor as the layer commit
  is.
- **Empty/off-canvas result** → commit an empty selection with one undoable
  state; the box's degenerate-quad guard still refuses sub-pixel boxes.
- **Soft-mask preview differs from the final contour** → the live preview maps
  the pre-transform 50 % contour; the commit re-contours the resampled mask.
  Both are within the 1 px box criterion; the 0.5 px contour criterion is
  asserted on the committed mask.
- **Warp box hug is the warped bounds bbox, not a tight content bbox** →
  deliberate ceiling; a tight box would require rasterising the mask on leave.
  Recorded with a `ponytail:` note in the code.
- **`transform_session_path` is empty for a selection session** → add an
  explicit `transform_session_is_selection` readout and audit path consumers;
  tests keep the layer-path contract.
- **Struct literals in transform tests** must add the new target field →
  mechanical update in the existing test helpers.

## Migration Plan

No document-format or saved-state migration. Selections are transient document
state; the command records an ordinary history state and the PSD gains no new
record. Rollback is removing the command wiring; existing layer transforms are
unaffected because the target defaults to `Layer`.
