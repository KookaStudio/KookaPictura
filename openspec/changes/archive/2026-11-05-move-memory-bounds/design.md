# Design

## Moving image

`compute_move_preview` builds a single pixel layer's image over
`content_rect ∩ layer.rect` with `layer_image_within`, which reads each plane
through `Plane::row` (a banded plane never materializes) and copies only the
box. `move_layer_offset` records the box's offset inside the layer, so the
cache-hit path, which places the image at the layer's moved rect, adds it.
Shape layers and move groups keep their paths and a zero offset.

## Damage

`move_reach` returns `content_rect` for a pixel layer with no effects that is
not a group or an adjustment; with no visible pixels it falls back to the rect,
so an empty layer still moves. The commit's region composite, the display patch
and the history hint all follow the smaller rect, and the level-0 frame is
patched rather than replaced.

## Gate

The fill family's gate moves to `cxxqt_object::memory_gate` and its bridge is
renamed `edit_refused_for_memory` / `set_available_memory_for_test`. Estimates:

| Edit | Peak |
|---|---|
| Move press | 16 × mark area (planar copy, RGBA bytes, image, canvas copy) |
| Move commit | 12 × damage area (composite, sRGB copy, display patch) |
| Transform commit | 4 × output box + 12 × (output box ∪ source rect) |

A refused commit translates the layers back, or drops the transform session,
which untrims and restores a lifted document.

## History cap

`History::enforce_byte_cap` runs after the depth limit in every capture. It
counts the allocations of every retained document's tracked planes (each
counted once, those the live document shares excluded) plus the stored delta
tiles, and drops the oldest state while that exceeds `total_memory() / 4`,
stopping at two states. No cap when the host's memory cannot be read.
