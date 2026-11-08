# Tasks

## 1. Tests

- [x] 1.1 `pictura-core` `tiled_plane/tests.rs`: the seeded `Vec` oracle, one-pixel fork,
  per-channel collapse, empty default tiles, identity, generations, channel
  restart, uniform writes, `build_tiled`.
- [x] 1.2 Rewrite the plane and access tests from bands to tiles.
- [x] 1.3 Paint: tile-fork tests for strokes, fills, gradients and buckets,
  and the M6 width-independence test.
- [x] 1.4 History: tile tests, `untouched_tile_keeps_its_arc`, the hinted
  capture sharing fully hinted tiles.
- [x] 1.5 `fork_bytes_counts_tiles`; M1, M2 and M3 plane-cost guards.
- [x] 1.6 Codec `tests/tiled_planes.rs`: byte-identical tiled write, tiled read.

## 2. Core

- [x] 2.1 `TiledPlane<T>` with the `Empty` / `Uniform` / `Dense` tile states,
  generations and `settle`.
- [x] 2.2 Swap `Plane`'s insides to `Flat` | `Tiled`; delete the bands and the
  band API.
- [x] 2.3 Tile access: `for_each_tile`, `tiles_mut`, `tiles_mut_at`,
  `for_each_tile_mut`, `row` (Cow), `read_row_into`, `write_row_from`,
  `all_eq`, `allocations`, `detached`, `filled`, `build_tiled`.
- [x] 2.4 `layer_bounds`: `padded_plane`, `cropped_plane`; grow reserves tiles.

## 3. Callers

- [x] 3.1 Render: compositing, GPU assembly, composite store, Move, clipboard,
  via, transform.
- [x] 3.2 Paint: the cell raster, tiled coverage buffers, fills, saved tiles.
- [x] 3.3 App: history, budget, `fork_bytes`, New Document, level 0, RGBA
  conversions.
- [x] 3.4 Select: the polygon's local task size.
- [x] 3.5 Codec: streamed PSD write (`ScanRows`), tiled read.

## 4. Verification

- [x] 4.1 `cargo fmt`, `clippy -D warnings`, `nextest` for the workspace.
- [x] 4.2 Lower `BASELINE`; `STREAMED` lists M1–M12.
- [x] 4.3 Timing against master and the brush scenarios
  (`profile-tools.py`), recorded in design.md and the PR.
