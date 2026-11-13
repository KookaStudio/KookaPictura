# Design

## D1. Source windows

`Planes` (unit `f32` planes, premultiplied when an alpha plane is given) now
holds a window `[x0, x0 + w) × [y0, y0 + h)` of a `sw × sh` source rather than
the whole source. `tap` keeps its source-local integer coordinates: a tap outside
the source reads zero, as before, and one inside reads `(y − y0) · w + (x − x0)`.
Every sample position, snap, floor and weight is computed on the same
source-local doubles as the dense rasterizer, and only the integer index is
shifted, so the sum is byte-identical.

A window is read from the layer's planes with `Plane::read_rect_into` and
converted per plane exactly as the dense path converts the whole plane (8 and
16-bit through `Sample::to_unit`, 32-bit with NaN as zero, the mask's alpha as
ones), then premultiplied in `f32`.

## D2. Blocks and their windows

For each strip of 256 destination rows, the blocks (64 rows × 256 columns) are
rendered in parallel. A block keeps the triangles of its row band (the binning is
unchanged) whose padded x range, the same `floor(min − 0.5)` and
`ceil(max + 0.5)` the rasterizer loops over, meets its columns. The filter
keeps their order, so a fold's later cell still wins, and the block's
rasterization is the dense one restricted to it.

The block's window is the bounding box of every covered pixel's taps. A point
sample reads `floor(u ± ε) − 1 … floor(u ± ε) + 2` (the bicubic reach, with
`ε` covering the snap). A shrinking pixel's bilinear subsamples lie within
`½(|j₀| + |j₂|)` of `u` in x and `½(|j₁| + |j₃|)` in y. The box is clipped to the
source. When its window would exceed `WINDOW_BYTES / (4 · planes)` samples
(8 MiB, the per-thread window the gate already charges), the block's pixels
split along the longer side, recursively, as `transform_tiled` does. A single
pixel's footprint is at most about 20 × 20, since `MAX_SS` caps the step.

## D3. Output

Each pass's outputs are `Samples` planes made with `Plane::filled(dw, dh, 1, 0)`,
uniform zero tiles that hold nothing. A block converts its `f32` result to the
output types (`u8`/`u16::from_unit`, `f32` as is) after a per-pass `post` step:
the mask pass flattens gray over its default by the coverage alpha, as
`warp_gray` did. After a strip's blocks are rendered, they are written with
`write_rect_from` and the planes are settled. Only one strip's typed blocks are
held at a time.

Progress advances by a strip's rows. A cancelled job skips the remaining strips,
and the op's existing `job_cancelled` check discards the result.

## D4. Gate

`warp_strip_bytes(quad) = 5 · 4 · TILE · width(quad)`: up to five output planes of
32-bit samples for one strip. `commit_transform` adds it to a warp commit's
`working`.

## D5. Control and M14

`layer_op` `free_transform` takes optional `warp` (a preset id such as `arc`)
and `bend` (percent, default 50, within [−100, 100]). After the width, height and
angle are set it enters Warp, picks the preset, and sets the bend, as the
options bar does. M14's setup paints Layer 1 with a black-to-white Gradient
across the document. The measured step is `free_transform` at 100 % with an `arc`
warp of bend 50.

**Ceiling (`ponytail:`):** the 16/32-bit path's `write_layer` narrows the
retained store to dense 8-bit channels, as it does for the similarity path.
