# Design

## Context

See `proposal.md` — Why. The current state that shapes the approach:

- `build_transform_preview` builds a backdrop with the target (or linked set,
  or clipping group) hidden. That backdrop already contains the layers above the
  target, so the transformed layer is painted over them.
- The Move preview solved the same problem for a drag (#1250): `above_patch`
  composites the layers above the moving group over transparency, and the
  canvas draws that image back over the moving image clipped to it.
- The transform session already reuses the Move preview's base/layer fields
  (`move_base`, `move_layer`, `move_above`, …) and the canvas stores the above
  image in `moveAbove_`.

## Goals / Non-Goals

**Goals:**

- Keep the document's stacking order visible for every transform gesture.
- Reuse the existing above-layer image and bridge, with no new preview path.
- Build the above image once at session start; the layers above do not change
  during a transform.

**Non-Goals:**

- Changing what the commit writes.
- Fixing the Move preview's existing semi-transparent double-blend caveat.
- Changing layer order or selection.

## Decisions

### D1. Build the above image with the transform's own hidden set

The transform hides a target, a linked set, or a target plus its clipping run.
Pass exactly that hidden set to `above_patch`, so the layers above the clipped
run are drawn back and the clipped members keep moving with the target.

Alternative considered: pass only the session path. Rejected — a clipped layer
above the target would be drawn back on top of the moving clipping group
instead of moving with it.

### D2. Clip the above image to the transformed footprint

The above image is drawn over the transformed layer only, using the same
similarity rectangle, projective quad, or warped-image bounds that the preview
paints. Outside the moving content the base or live canvas already shows the
above layers, so a wider clip would blend them twice.

Alternative considered: draw the above image over the whole presented rectangle.
Rejected — semi-transparent above layers would darken over themselves.

### D3. Reuse the Move preview's above fields and bridge

The transform preview populates `move_above`, `move_above_origin`, and
`move_above_scale`; the canvas gets them through the existing
`move_preview_above*` bridge. This keeps one implementation and one invalidation
story instead of a parallel transform-only channel.

## Risks / Trade-offs

- **A transform may extend beyond the presented rectangle.** The above image is
  built over the presented rectangle when the canvas has one, else the whole
  canvas, matching the Move preview. Content outside the presented rectangle is
  not on screen.
- **A reduced transform preview with a full-resolution above image.** The above
  image follows the presented level, as the Move preview does, so it stays at
  the canvas's sampling resolution.
