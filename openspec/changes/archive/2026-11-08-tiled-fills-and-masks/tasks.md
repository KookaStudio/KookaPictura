# Tasks

## 1. Core

- [x] 1.1 `Plane::set_tile_uniform`; `TiledMask` alias.
- [x] 1.2 `mask_ops::flood_tiled` with per-tile visited bits and whole tiles.
- [x] 1.3 `mask_ops::map_bands`, `mask_ops::coverage_bounds` and uniform-over helpers.
- [x] 1.4 Parity tests: `tiled_flood_matches_dense`, reached tiles, uniform source.

## 2. Select

- [x] 2.1 Tiled constructors (none, all, rect, ellipse, polygon).
- [x] 2.2 Combine, invert, translate and bounds per tile.
- [x] 2.3 Feather, expand, contract, border and smooth in bands.
- [x] 2.4 Wand, Grow and Similar on the tiled flood.
- [x] 2.5 Parity and no-materialization tests.

## 3. Paint

- [x] 3.1 Fill, native fill and gradient: tiled selection and mask, coverage argument.
- [x] 3.2 Uniform fast path in `fill_eight_bit`.
- [x] 3.3 Bucket flood, anti-alias, `fill_within` and stroke band on `TiledMask`.
- [x] 3.4 Magic Eraser on the tiled flood and tiles.

## 4. App

- [x] 4.1 Fills, wand, Quick Selection and Magic Eraser call sites.
- [x] 4.2 Memory gate estimates.
- [x] 4.3 Heap guards: lower M4/M5, add the wand scenario.
- [x] 4.4 `tst_fill_tools::paintBucketFillsAWholeEmptyLayer`.
- [x] 4.5 `ponytail:` note on `Selection` for the remaining dense readers; follow-up issue.

## 5. Verification

- [x] 5.1 fmt, clippy, nextest, CMake build, Qt suites.
- [x] 5.2 17000² profile before and after; results in `design.md`.
- [x] 5.3 `openspec validate --all --strict`; archive.
