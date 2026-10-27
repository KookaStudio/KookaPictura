# Design: A moved link set recomputes its move preview

`rendered_move_group` composites the members into a scratch document at their
current positions and crops to the canvas-clamped union rect. Both the composite
and the clip depend on the positions, so the image is only valid for the
positions it was built at.

`move_cache_valid` previously reused the prepared image whenever the content
revision and the group key matched, and a move changes neither (the move commit
deliberately does not bump `content_revision`, and the group is the same). It now
also compares the group's union rect — `move_group_union(doc, paths)` — with the
rect stored when the preview was prepared (`move_prepared_union`, `None` for a
single layer, whose own image is position-independent). A moved group therefore
recomputes; an unmoved group still reuses the prepared image, so the `warm()`
path keeps its value.

`compute_move_preview` stores the union rect beside the group key, and the
existing cache clears also clear it.
