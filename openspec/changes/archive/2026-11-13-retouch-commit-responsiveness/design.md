# Design

## Windowing the heal

`heal_layer` currently reads every channel of the layer into a dense
`RgbaImage` and copies the result back pixel by pixel. The operation itself
only reads a bounded neighbourhood:

- Spot Healing: the dirty stroke rectangle grown by the margin and `BORDER`;
  Proximity Match additionally reads the source-search reach around the hole.
- Healing Brush / Patch Source: the region and the clamped source it samples.
- Patch Destination: the drag target and the selection it samples back from.
- Content-Aware Move: the selection and its destination.
- Content-Aware Patch: the region plus `BORDER`.
- Red Eye: the clipped box.

Each caller now computes a conservative document rectangle and passes it to
`heal_layer`. The layer is clipped to it, the dense image is built from the
plane's `row_span` (which reads a window of a tiled plane without
materializing it), and the result is written back with `write_rect_from` for
the dirty rectangle only. A transparency lock still skips the alpha plane.

`clamped_shift` returns the pixels a shifted rect reads after clamping at the
layer edge, so a source that runs off the layer stays inside the window. The
Proximity window uses the same reach formula as `best_source`
(`2 × span + 2 × CONTEXT + 2`), so no candidate the full-layer search would
accept is lost.

## Red-black relaxation

The solves are the dominant cost. `relax` keeps the same 96 sweeps and
over-relaxation factors, but updates the grid in two colours per sweep. A cell
of one colour has only the other colour as 4-neighbours, so all cells of a
colour are independent and their rows run in parallel. The reads of the
neighbour colour use the allocation's base pointer; the pass never writes that
colour, so the aliasing is sound. Every cell reads the previous colour's values,
which makes the result deterministic.

The solver result therefore stays within the existing tolerance contracts; the
reference tests compare the red-black result with the former lexicographic
relaxation and accept a small ordering difference.

## Red Eye

`red_eye_layer` passes its clipped box as the window. `red_eye_region` splits
the box's rows into Rayon chunks and applies the existing red-dominance formula
per pixel. Pixels are independent, so the result is identical to the serial
loop.

## Selection bounds

`bounds` scans each row of the document-sized selection mask for its first and
last covered pixel. Rows are independent and the collected spans are folded in
scan order, so the bounding box is the same as the serial scan.

## Not in this change

- The solver keeps its iteration count; a multigrid or adaptive stopping rule
  can follow if the parallel sweep is still too slow on real content.
- Move commits and transform previews keep their existing out-of-core paths.

## Measured

Same host, RelWithDebInfo app with the optimized `dev-app` Rust profile, one
fresh 8000×6000 white document per run, 60 motion events. The release (`up`)
before and after this change:

| Scenario | Before | After |
|---|---:|---:|
| Patch | 6 619 ms | 474 ms |
| Spot Healing | 3 631 ms | 1 328 ms |
| Red Eye | 730 ms | 89 ms |

At 17000×17000 (white document, same build), the after run is Patch 2 755 ms,
Spot Healing 7 871 ms and Red Eye 548 ms; the issue's baseline on its host was
42 916 ms, 18 320 ms and 2 745 ms. Spot Healing remains the largest because its
zigzag stroke's bounding box is much larger than its covered hole; the solve
now touches only the hole, but the window read, margin search and Proximity
search still walk that bounding box.
