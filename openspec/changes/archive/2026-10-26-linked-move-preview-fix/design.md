# Design: A linked move previews and lands at the drop

## Preview origin

`rendered_move_group` crops the rendered group to `clamp(union, canvas)` and
returns that clamped origin. `move_origin_opacity` — used on a preview-cache hit
— returned the unclamped `union.left/top`, so once the group had been dragged
past the canvas edge the cached preview drew the cropped image offset by the
clamp. `move_origin_opacity` now clamps to `max(0)` on both axes, matching the
crop, so a cached preview places the image where a fresh one does and the
committed `translate_move_group` (which shifts the real rects by the delta)
lands the layers under the pointer.

## Smart-guide references

`smartGuideReferences` excluded one path and its descendants. For a
multi-selection that left the other selected layers (and, for a link set, the
unselected partners) as references, so the moving box could snap to itself. The
function now takes the list of moving paths; the Move tool passes
`move_group_paths(view)` — a new free-function bridge over the transform session
— and excludes every one of them. The Type tool passes its single edit path.
