# Design

## D1. Strip builder in core

`Plane::build_tiled_strips(width, height, channels, f)` calls
`f(first_row, rows, strip)` once per strip of up to 256 rows, then writes the
strip into a tiled plane and settles it, so uniform tiles drop their
allocation. `build_tiled` (per row) is rebuilt on top of it. Each strip's dense
tiles are new `Body` allocations, registered with the evictor like any other
live tile.

## D2. Block-window resampler (`transform_tiled.rs`)

The resampler is generic over `T: Sample + TileSample`, with a `pad`
argument: 2 for 8-bit planes (transparent edge anti-aliasing) and 0 for deep
planes (clamp to the edge, as today).

For each strip, it processes the destination in 256-column blocks in parallel.
A block first stores the inverse-mapped source coordinate of every pixel (in
padded-plane coordinates, `None` for a point at or beyond infinity, or for one
outside the padded plane). The kernel taps of a sample at `lx` span
`floor(lx−0.5)−1 … floor(lx−0.5)+2`. The block's window is the bounding box of
its samples' taps, clipped to the padded plane. When the window exceeds
`WINDOW_PIXELS` (4 Mpx), the block splits along its longer side, recursively
down to single pixels. An extreme downscale therefore reads small, sparse
windows instead of the whole source.

The window is filled with zeros, and the source rows it covers are read with
`read_rect_into`. A sample is `kernel::sample(window, ww, wh, lx − wx0,
ly − wy0)`.

**Why this is byte-identical.** `lx` is a multiple of its ulp (≤ 1), so
`lx − wx0` is exact, and the fractional part and every weight are unchanged.
Every tap lies inside the window, except at a padded-plane edge, where the
window edge is the padded edge too, so clamping behaves the same. The dense
resampler is kept as the `#[cfg(test)]` oracle.

**Progress and cancellation.** `advance(rows)` is called per strip and plane,
and a cancelled job skips its remaining strips. The strip buffer comes from
`memory::try_buffer`, so the phase-2 failpoint contract still holds.

## D3. Tiled banded composite

`composite_banded_with_plan` keeps the flat `Plane::build` for regions below
`TILED_REGION_PIXELS` (16 Mpx), so small edits keep their fast flat buffers. A
larger region is built as `Plane::filled(rw, rh, 4, 0)`. For each group of four
bands (256 rows), the bands are composited in parallel into a group buffer, then
written with `write_rect_from` and settled. The band function is unchanged, so
the output is byte-identical.

`patch_buffer_region` reads the region row-block by row-block with
`read_rect_into` instead of slicing `region.data`, so a tiled region is never
materialized.

`RegionComposite::srgb` becomes lazy. The deferred whole-document landing on a
large document invalidates the display tiles and never reads it, so a
document with a profile skips its whole-image display conversion.

**Ceiling (`ponytail:`):** a stack that cannot band (an effect pass that reads
the pattern library) still composites densely. A display conversion of a
non-whole region of a profiled document is still dense.

## D4. Gate

`commit_bytes` becomes `CommitCost { working, spillable }`:

- `working = 3 · 4 · TILE · max(out_w, union_w) + threads · WINDOW_PIXELS`.
  This is the strip and group buffers, plus one window per worker thread.
- `spillable = 4 · area(out) + 4 · area(union)`: the new layer tiles and the
  tiled composite. A whole-pixel move has no new layer tiles.

`memory_gate::admit_transform(cost)` admits `working` through the governor (a
memory refusal), then requires `spillable ≤ room + scratch available`. When
that fails it records a scratch-full refusal (`SCRATCH_FULL`), which also
counts as a memory refusal, so every existing caller still reports it.
`transformMemoryRefusalMessage()` then names the scratch disks.
`scratch::sizes().available` already sums the usable volumes' free space. A
test seam pins it.
