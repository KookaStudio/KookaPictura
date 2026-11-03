# Design

## Damage

Each path reads `damage::influence` before and after its mutation and passes the union
through `structural`. The union matters: a change of alignment can move a same-sized
raster, and a change of font or size can resize it. `influence` already covers effect
reach and the clipped run above the layer.

`influence` ignores visibility, so the rect of a layer hidden by a session is still
its raster. `type_commit_edit` reads `before` while the layer is hidden; the union then
covers the old and the new text, which is everything the history hint must see.

## Previews

`view.preview` replaces `recomposite()` + `changed()`, which also ends the double
`changed` of a colour tick. `apply_edit_with` forces the full path when a pending
preview, a soloed mask, or a live session owns the display, but `preview_edit` does
not. The shared `reach` helper therefore answers `None` in those states, so previews
keep the same rule as commits.

## History hint

Region edits scan history without a hint. The scan compares only bands the edit
unshared, so it stays cheap, and a hint would lose changes outside the refreshed rects:
a dialog's earlier previews, or the raster bytes around a trimmed box.

## Knockouts

`influence` refuses a knockout document because a structural edit can reassign the base.
An in-place pixel rewrite keeps the stack, and knockouts resolve per pixel inside the
region, so `content_influence` skips that refusal. Hiding a layer is not a content edit
(the bottom layer is the base), so session hide and show keep `influence`.

## Undo

Tiles cannot describe a walk across a segment, because the raster changed geometry. The
restore compares the composites instead, skipping bands the states share, and reports
the union of the changed tiles. A mask or document channel a view shows directly must
match. An empty comparison stays a whole refresh, because a reorder of equal layers
still needs its panels redrawn.

## Ink

A box nearly the size of the canvas can hold a few words. `ink_influence` scans its alpha
only at that size, and only when the reach is the raster alone. The raster itself starts
lazily zeroed and only inked rows are copied, so an empty box costs no pages. Type ink that really
covers the canvas stays a whole refresh.

## Measurement

`type_edit_profile` runs edits through both paths on a 16507×16196 document, and
`scripts/profile-tools.py` drives the running app through `type_op`.
