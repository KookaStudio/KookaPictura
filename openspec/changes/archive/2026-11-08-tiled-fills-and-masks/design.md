# Design

## Context

#1213 gave every document plane 256² tiles that are `Empty`, `Uniform(v)` or
`Dense`. `Selection.data` was already a `Plane<u8>`, but every constructor
built it flat. The bucket's flood, the wand, the anti-alias pass and every
selection operation then made dense copies the size of the region or of the
document. In the M5 heap guard (a whole-layer bucket at 4250²) that is about
86 MiB.

Owner decisions (2026-11-07):

- **No new mask struct.** `TiledMask` is a type alias for `Plane<u8>`, tiled at
  the document's width and height. A selection is always stored tiled. A plane
  that is still flat (a test literal, a channel loaded flat) is still accepted,
  because the plane API serves both forms.
- **Port scope.** The paths the issue names are ported: the bucket, the wand,
  Fill, Stroke, Gradient, the selection operations and Quick Mask. Grow,
  Similar, the Magic Eraser and Quick Selection are ported as well.
  - Every other consumer of a dense `&[u8]` selection (adjust, clipboard,
    mask edit, healing, filters through `selection_to_mask`, and about 67
    reads of `.data` in the app) keeps the counted `as_slice` materialization,
    marked `ponytail:`. A follow-up issue tracks those.
- **One pull request.** It closes #1214.
- **Capability paths.** There is no `selection` domain, and
  `openspec/config.yaml` forbids adding one, so the capabilities are
  `tools/tiled-flood` and `tools/tiled-selection-masks`.
- **`fills/budget.rs`** in the issue is `pictura-app/.../helpers/memory_gate.rs`.

## Decisions

### Masks live on the document grid

A flood's output, the bucket's anti-aliased mask and the stroke band are
document-sized `TiledMask`s. They are no longer row-major buffers the size of
their box. Tiles outside the box are `Empty` and cost metadata only: one enum
per tile, 4489 tiles at 17000².
- The mask's tiles line up with the selection's tiles.
- They also line up with a layer's tiles whenever the layer's origin is on the
  256 grid, which holds for a layer grown to the canvas.

### `flood_tiled`

`mask_ops::flood_tiled(w, h, seeds, matches, whole)` is a scanline flood over
linear indices, like `flood`. Its arguments:
- `seeds` is a single pixel or a seed mask (Grow).
- `matches(from, to)` is the same predicate as before.
- `whole(tile)` tells the flood that once any pixel of `tile` is reached,
  every pixel of it is.

How it works:
- **Visited state.** A 1-bit-per-pixel bitmap per tile (1024 `u64`, 8 KiB),
  allocated the first time the flood marks a pixel there, plus a "whole" flag
  per tile.
- **Reaching a whole tile.** When the flood first marks a pixel in a tile for
  which `whole` holds, it sets the flag and pushes the tile's top and bottom
  rows as spans. It then widens out of the tile's left and right edges, row by
  row. Every pixel of the tile is marked, and every neighbour outside it is
  examined, so the reached set is the same as the dense flood's.
- **Why the result matches the dense flood.** The predicates in use depend
  only on the target pixel (bucket, wand), or are symmetric (Grow's
  pixel-to-pixel distance). The reached set is therefore the reachable set of
  an undirected graph, and the order of the visits does not change it.
- **Callers.** The bucket and the wand pass `whole` = "the source tile is
  uniform, its value matches, and the selection tile is uniform and non-zero".
  Grow passes "the source tile is uniform and the selection tile is uniform"
  (a uniform tile is connected under any tolerance).
- **Output.**
  - A tile with the whole flag becomes `Uniform(255)`.
  - A tile with a bitmap becomes dense 0/255 bytes, settled.
  - A tile never reached stays `Empty`.
  - The bitmaps are dropped as their tiles are written out.

A global (non-contiguous) selection evaluates `matches` tile by tile, and
decides a uniform source tile with one test.

The dense `flood` and `flood_from` stay as the reference for the parity tests.
Once every caller has moved, they are compiled only under `#[cfg(test)]`.

### Banded window operations

`mask_ops::map_bands(mask, w, h, pad, uniform, f)` runs a neighbourhood
operation band by band:
1. A band is one row of tiles.
2. Its window is the band's rows grown by `pad`, clipped to the canvas.
3. For each run of tiles in the band that need computing, the window's columns
   are those tiles' columns grown by `pad`, clipped. They are copied with
   `read_rect_into`.
4. `f(window, ww, wh)` computes the window, and its centre is written back to
   the run's tiles.

Two shortcuts:
- A tile whose window, grown by `pad`, lies inside the canvas and covers only
  `Uniform(v)` / `Empty` tiles with the same `v` becomes `uniform(v)` without
  computation.
- Dilate, erode, median and both blurs of a constant are that constant, so
  `uniform` is the identity for all of them.

**Why it is exact.**
- The direct Gaussian sums fixed taps per pixel.
- The box stack's running sums are whole numbers, exact in `f32`, and every
  pass rounds to `u8`.
- The morphology and median kernels read at most `radius` away.
- So a pixel at least `reach` from a window edge that is not a canvas edge
  reads the same samples as in the whole-mask operation. At the canvas edge the
  window's clamp, or its `Border::Zero`, is the canvas's own. `pad` is
  `reach + 1`, as `feather_within` already uses.

Feather, expand, contract and smooth use `map_bands`. Border is built from
`expand` and `contract` per tile. The stroke band uses `map_bands` with
`Border::Clamp` inside its window.

**Memory.** At most one band window is held at a time per run, which is wide
but only `256 + 2·pad` rows tall. The parallelism moves inside the kernels:
every kernel already runs its rows on every core.

### Selections

`Selection::none` is `Plane::filled(w, h, 1, 0)`, and `all` is
`Plane::filled(.., 255)`.
- **Rectangle.** Writes `Uniform(255)` straight into the tiles it covers whole
  (a new `Plane::set_tile_uniform`), and dense edge rows through
  `write_rect_from`.
- **Ellipse and polygon.** Rasterize into a scratch buffer per band of tile
  rows (the polygon keeps its span fill), written with `write_rect_from` and
  then settled.
- **Combine.** Walks tile pairs: a uniform pair gives a uniform result, a dense
  tile is combined in place.
- **Invert.** Maps uniform tiles directly.
- **Translate and bounds.** Use tile states: bounds skips `Empty` tiles and
  takes a `Uniform` tile's rectangle whole.

### Fills

`fill_layer_within` and `fill_planes` take `selection: Option<&TiledMask>` and
a `mask: Option<&TiledMask>` coverage. The source closure becomes
`Fn(x, y, coverage: u8) -> Option<(Rgba, f32)>`. The bucket's closure computes
`coverage / 255 * opacity` exactly as before, so the floats are bit-identical.
Each row of a target tile copies its selection and mask spans (at most 256
bytes) with `read_rect_into`.
- **Uniform fast path.** Before the tile pass, `fill_eight_bit` visits the
  target tiles that lie wholly inside the region. A target tile qualifies when:
  - every one of its planes is uniform;
  - the selection and the mask are each uniform over its document rectangle;
  - the source is position-free (the caller says so: a foreground bucket or
    Edit › Fill, never a pattern or a gradient);
  - the mode is not Dissolve.

  A qualifying tile is blended once. If the result changes the tile, it is
  written with `set_tile_uniform` on each plane, and the remaining tiles go
  through `tiles_mut_at`.
- **16-bit and 32-bit layers** stay on their flat native store (a ponytail
  from #1213), and read the masks row by row in the same way.

### Memory gate

The bucket estimate is the tiles of the box × (8 KiB visited + 64 KiB output)
× 2 (flood and anti-alias), plus the tiles the fill forks, plus the refresh.
It replaces `2 * area`. `estimates_cover_the_measured_peaks` keeps the
estimate above the measured peak.

## Risks

- A document-sized `TiledMask` for a tiny selection on a huge canvas costs its
  tile vector: 4489 entries at 17000², about 100 KiB. That is accepted.
- Callers that still index `selection.data` materialize it. They are counted,
  marked `ponytail:`, and outside the M-scenarios' measured steps.

## M5 scoping (owner decision, 2026-11-07)

The M5 bound is met by the bucket edit and by the layer it leaves. The display
refresh after a whole-canvas edit (a region composite and the view's buffers,
≈ 68 MiB at 4250², ≈ 1.1 GB at 17000²) is out of scope, as #1213 scoped the
refresh out of M2. It belongs to #1216, the viewport tile pyramid.

## Results

Heap guards (`tests_profiles/memory.rs`, 4 rayon threads); master is
`daeed06f`:

| Scenario | Master, 4250² | This change, 4250² | This change, 17000² |
|---|---|---|---|
| M4 bucket in a marquee | 1.7 MiB | 1.7 MiB (the edit: 0.6 MiB) | 13.2 MiB |
| M5 whole-layer bucket | 86.4 MiB | 69.0 MiB (the edit: 86 KiB; refresh: 68 MiB) | 1103.6 MiB (refresh, #1216) |
| M5 layer planes afterwards | not measured | 0 B (uniform tiles) | 0 B |
| Magic Wand on the M12 fixture | n/a | 0.35 MiB | 2.4 MiB |

Every M-scenario keeps 0 materializations.

Timing: `profile-tools.py` at 4000×3000, with master and this change built
`RelWithDebInfo` and run alternately three times each. The table gives the
median `total_ms`.

| Gesture | Master | This change | Ratio |
|---|---|---|---|
| Paint Bucket | 129.4 | 91.5 | 0.71 |
| Magic Wand | 167.9 | 18.3 | 0.11 |
| Magic Eraser | 362.2 | 79.9 | 0.22 |
| Quick Selection | 177.9 | 22.2 | 0.12 |
| Marquee | 36.0 | 30.7 | 0.85 |
| Lasso, feather 20 | 92.1 | 89.4 | 0.97 |
| Gradient | 176.1 | 181.4 | 1.03 |

At 17000² (`--scenarios M4,M5 --peak`):
- **M4** took 0.98 s on master and 0.99 s with this change. Its peak added is
  13 MiB on both.
- **M5** took 3.85–4.05 s on master and 2.38–3.37 s with this change. Its
  peak added went from 1377 MiB to 1101 MiB, all of it the refresh (#1216).

Two fixes came out of the measurements:
- **Selection reads.** On a selection change, the selection's pixel count, its
  coverage probe and the Quick Selection preview read the whole mask as one
  slice. That materialized a copy of the tiled selection, 271 MiB at 17000².
  They now walk the tiles or sample one pixel.
- **Feather windows.** `map_bands` runs its windows side by side on every
  core. It merges neighbouring bands into one window of at most 8 M pixels,
  and computes only the coverage's box grown by the pad, because every kernel
  in use maps zeros to zero.
