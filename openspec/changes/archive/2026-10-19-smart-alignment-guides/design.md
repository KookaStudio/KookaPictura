# Design: smart-alignment-guides

## Data flow

1. **Reference boxes.** `pictura_render::layer_content_bounds(doc, path)` returns
   the document-space box round a layer's non-transparent pixels (the same
   computation `Layer > Align` uses). A bridge call exposes it to C++ as
   `transform_api::contentRect`. The tools collect every visible layer's box once
   at drag start (excluding the moving path and its descendants) and keep it for
   the drag; per-move work is then pure geometry.
2. **Moving box.** The Move tool already computes `contentBounds(warmLayer_)` —
   the moving layer's ink box in layer-local coordinates. The Type tool computes
   its ink box once when the move begins, from the preview buffer's alpha, and
   translates it by the origin delta.
3. **Solver.** `smart_guides::snap(moving, references, document, zoom)` returns
   the smallest `(dx, dy)` within `8 / zoom` document pixels that aligns a
   moving edge/centre with a target edge/centre, plus the active guide lines and
   gap measurements. Each axis is solved independently.
4. **Apply.** The Move tool keeps the pointer's raw delta and uses the snapped
   delta for the preview and the commit, so a snap is sticky until the pointer
   leaves the threshold. The Type tool adds the snap to its frame origin.
5. **Draw.** `ImageView` stores the guide lines and gap labels; `paintEvent`
   draws them after the existing guides, in the view frame, 1 px magenta
   (`#FF4AFF`), with a small dark label for the gap.

## Solver

For the x axis, candidate moving positions are `left`, `(left+right)/2`, `right`.
Targets are the document's `0`, `width/2`, `width` and each reference's three
positions. The smallest `|target - candidate| <= threshold` wins; ties prefer the
earlier reference, then the document. The y axis is symmetric. After snapping,
every target that coincides with a snapped moving position within an epsilon
adds a guide line, so a same-width layer can show two lines.

The gap for a snapped axis is the distance between the moving and reference boxes
along that axis (0 when they overlap), with an anchor at the midpoint between the
facing edges.

## Exclusions

- The moving layer and its descendants are never references.
- Groups and adjustment layers have no ink box (the existing Align rule).
- Hidden layers are skipped.
- Ruler guides, grid, and slices are not targets (out of scope).
