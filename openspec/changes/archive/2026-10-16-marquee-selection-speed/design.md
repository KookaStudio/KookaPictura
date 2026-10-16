# Design

## Measurements

`marquee_selection_profile` (`tests_profiles.rs`, `--ignored --nocapture`,
release, 8000×6000, 5000×3000 shape at (1000, 1000)):

| step                     | before            | after            |
|--------------------------|-------------------|------------------|
| build rect / ellipse     | 96 / 204 ms       | ~10 / ~10 ms     |
| `feather(0)`             | ~130 ms clone     | 0 (moved through)|
| combine into no selection| ~30 ms            | 0 (shape used)   |
| contour rect / ellipse   | 486 / 274 ms (×2) | ~2 / ~8 ms (×2)  |
| history record           | ~50 ms            | ~50 ms           |
| drag move: translate     | 189 ms            | ~20 ms           |
| drag move: retrace       | 486 ms            | 0 (offset ants)  |

The "before" column was measured with the same shapes through the old
primitives. The history record (two selection clones plus the plane diff) is
unchanged and is the largest remaining commit cost.

## Outline tracer

Each row becomes a sorted list of half-open inside runs. For row boundary `y`,
top edges are `below \ above` and bottom edges `above \ below` (maximal run
differences); each run contributes a left and a right unit edge. Edges are
directed with the inside on one side, so every lattice vertex has equal in- and
out-degree and a walk from any unused edge closes on its start. Horizontal runs
are maximal, so no vertex lies inside another edge. Collinear vertices are
dropped after the walk. Identical consecutive rows are detected with a slice
compare and reuse the previous runs, so a rectangle costs one memcmp per row.

## Offset ants

`ImageView::setSelectionContourOffset` translates the painter for the
committed ants only. The drag still calls `preview_selection_move` (the mask
moves live, as the self-test requires); only the retrace is skipped. Release
and cancel reset the offset before the commit retrace, so a refused move shows
the origin outline.

Ceiling (`ponytail:` in `image_view.cpp`): a dragged outline past the canvas
edge is cut off by the document clip rather than closed along the edge; the
release retrace draws the clipped mask.
