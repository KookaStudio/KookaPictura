# Design

## Context

Before this change a document plane was one base allocation cut into 64-row
**full-width** bands (#1212 added streaming access over them). A one-pixel
write forked a whole band, so an edit cost grew with the width of the
document. A band could not say "one value", so a white Background cost its
whole area.

The #1212 API also assumed contiguous full-width rows: `row()` returned
`&[T]`, and `write_bands` handed out disjoint full-width `&mut [T]`. A square
tile cannot lend such a row, and an `Empty` or `Uniform` tile has no storage at
all. This was the escalation the issue asked for (rule 8). Owner decisions
(2026-11-06):

- **Fold the rest of #1212 into this change.** The paint, select, app and codec
  migrations ship here, and #1212 closes with it.
- **The API is tile-native, plus row copies.** The primary path is
  `for_each_tile` / `tiles_mut` / `for_each_tile_mut`. Rows are copied:
  `row()` now returns `Cow<[T]>` (borrowed from a flat plane, copied from a
  tiled one), alongside `read_row_into` and `write_row_from`. The full-width
  band API is deleted.
- **Acceptance scoping:**
  - **M2:** the new layer plus its history state ≤ 1 MiB. Refresh outputs are
    out of scope.
  - **M3/M4:** the bound is the square rounded out to the 256 grid × channels
    written × 1.2, measured on the layer's planes.
  - **M6:** the stroke's coverage buffers become tiled planes that start
    `Empty`.
  - **`as_slice` on a tiled plane** materializes, is counted, and is allowed
    only on `ponytail:` paths. There is no fourth "window" tile state.
- **The native 16/32-bit `Samples` stores stay flat** (`ponytail:`).

## Representation (`pictura-core`)

```rust
pub const TILE: usize = 256;                       // ponytail: fixed side
enum Store<T> { Flat(Arc<[T]>), Tiled(TiledPlane<T>) }
enum Tile<T> { Empty, Uniform(T), Dense(Arc<[T]>) } // Dense = clipped tile
TiledPlane { width, height, channels, tiles, gens: Vec<u64>, dirty: Vec<bool> }
```

- **Flat vs tiled.** A plane stays **flat** until `tile(width, height)` or
  `Plane::filled` / `Plane::build_tiled`. Filter and adjust buffers, metadata
  and native stores never tile, so their slices stay free.
- **Stacked channels.** `tile` takes the channel height, so a buffer that
  stacks channels (the composite, the level-0 frame) restarts the grid at each
  channel. Rows stay global (`c * height + y`). `tile` copies a flat plane
  once, and the uniform tiles of that copy hold nothing.
- **Writes and collapse.** A write forks only the tiles it touches and marks
  them dirty. A row or rect write that would leave a uniform tile unchanged
  forks nothing. `settle()` folds the dirty tiles that hold one value into
  `Uniform`, or `Empty` at `T::default()`. It compares bit for bit
  (`TileSample::same`, so `-0.0` and `0.0` stay apart). The settle points:
  - `for_each_tile_mut`, `build_tiled` and `padded_plane`;
  - the end of a fill or gradient;
  - the composite store;
  - history's `tile_planes` on every capture;
  - undo and redo swaps.

  A dab never settles, so dab latency pays nothing for it.
- **Generations.** Each tile has a `u64` from `fresh_stamp()`. A clone keeps
  it, and any `&mut` access to the tile replaces it. `shares_tile(i, other)`
  compares generations, and `share_tile_from` adopts another plane's tile and
  generation. The whole-plane `stamp` and `ident` are cleared by every `&mut`
  accessor, as before. Collapse keeps all three.
- **Counted copies.** `as_slice` on a tiled plane builds a counted mirror, and
  `as_mut_slice` flattens the plane. `detached()` copies dense tiles in
  parallel with fresh generations; it replaces the history's band copy.

## Caller migration

| Area | Change |
|---|---|
| core | `layer_bounds`: grow uses `padded_plane` (empty tiles plus strip copies); shrink uses `cropped_plane` (`build_tiled`); the governor reservation counts only the tiles the old content lands on |
| render | Compositing rows, GPU assembly, mask feather, damage, effects matte: `row()` copies. Region copy and the opacity probe: `read_rect_into`. The composite store tiles the composite and writes rects, then settles. Move grow/trim use the core helpers; the merge writes spans and settles. Inertness checks: `all_eq`. Transform: `read_rect_into` |
| paint | The dab raster runs on **cells** (zipped `tiles_mut` of the layer planes and the tiled `scratch`/`applied`). Parallel over cells; Dissolve walks the cells in raster order, so the rolls match the serial raster. Fill, gradient and bucket go through `fill_tiles` (whole-layer fills too), then settle. Saved-tile save, restore and grow read and write rects |
| select | `polygon` keeps a local 64-row task size |
| app | History: `tile_planes`; the diff compares 64² deltas inside unshared tiles (`regions` / `changed_region`); fully hinted tiles are shared via `share_tile_from`; swaps run one tile per core (`tiles_mut_at`); `copy_plane` = `detached()`. Budget and retained bytes use `allocations()`. `fork_bytes` = planes × tile-rounded hit area × sample bytes. New Document builds `Plane::filled` planes; level 0 tiles the composite and reuses its tiles when it is the same allocation. RGBA conversions read runs with `read_into` |
| codec | `ScanRows` / `PlaneBytes`: RLE, raw and ZIP read an 8-bit plane row by row (`native_plane_rows`), so the bytes are identical. Read tiles each decoded layer channel and mask |

## Shortcuts (`ponytail:`)

- **Tile size.** The 256² tile side is fixed; the CS6 "Cache Tile Size"
  preference does not drive it.
- **Contiguous fallbacks.** Filters, warp, whole-document mode/depth/resize
  ops, the non-RGB codec writes and the native stores still use contiguous
  slices. They materialize a tiled plane (counted).
- **PSD read.** Each channel is decoded whole and then tiled. A row-sink
  decoder would avoid holding one channel contiguously.
- **ZIP write.** The planes are gathered into one payload before deflating.
  RLE, the default, streams row by row.

## Measurements

Heap added by each guard's measured step at 4250² (`tests_profiles/memory.rs`),
before → after; every scenario reports 0 materializations and `STREAMED` lists
M1–M12:

| Scenario | Before | After |
|---|---|---|
| M1 new_document | 172.3 MiB | 103.4 MiB (the Background's planes: 0 B) |
| M2 add_layer | 86.1 MiB | 0.2 MiB (layer and history: 0 B) |
| M3 edit_fill | 14.4 MiB | 3.0 MiB (layer planes added: 768 KiB ≤ 1.2 MiB bound) |
| M4 bucket_in_selection | 15.5 MiB | 1.7 MiB |
| M5 bucket_whole_layer | 208.3 MiB | 86.4 MiB |
| M6 brush_stroke | 105.7 MiB | 2.3 MiB |
| M7 undo_redo | 252.4 MiB | 137.7 MiB |
| M8 snapshot_restore | 460.3 MiB | 93.6 MiB |
| M9 move_layer | 5.5 MiB | 1.9 MiB |
| M10 free_transform | 34.5 MiB | 3.5 MiB |
| M11 save_psd | 277.7 MiB | 122.7 MiB |
| M12 open_psd | 497.5 MiB | 357.4 MiB |

Process benchmark against master (`a331d752`) built on the same machine,
`profile-tools.py --scenarios memory --size 4250x4250 --peak --memory-max
1700M`: process peaks fall 1.2–2.4× (M8 653 → 278 MiB, M7 800 → 454 MiB, M11
503 → 214 MiB). Times stay within ±5 % except M7 at +2 % on a three-run median;
M11 is 11 % faster.

Brush scenarios (`brush-40/500/1940`, `pencil-1940`, `eraser-1940`), medians of
three alternating runs against master: total time −8 % to −17 % with the GPU
off and −8 % to −11 % with it on; per-move p95 within ±4 %. Stroke end is
faster for the large brushes; for 40 px it is +2–3 ms (+11–14 %) on a 17–25 ms
step.

Getting there took four fixes, each found by comparing against master:
- **Pyramid shrink reads only its rect.** The view pyramid reads only the
  level-0 columns of the rectangle it updates (`Planes::span`), not whole
  rows.
- **Raster tasks follow the dab.** The dab raster splits only the 64-row
  strips the dab reaches, and runs a single task inline.
- **Tile forks are cheap.** Tile forks are serial below five tiles, and a new
  tile is one allocation.
- **Uniform checks compare blocks.** The integer uniform checks (`settle`, and
  a write onto a uniform tile) compare 64-sample blocks as slices instead of
  calling `same` per sample, which had made a region patch 8× slower.

## Risks

- **Row copies.** A tiled plane's `row()` copies, which costs one `memcpy` per
  layer row in compositing. The time budget (≤ 10 %; dabs ≤ 5 %) is checked
  with `profile-tools.py`.
- **Re-tiling after a flatten.** A `ponytail:` path that flattens a tiled
  plane is re-tiled (one copy) at the next history capture.
