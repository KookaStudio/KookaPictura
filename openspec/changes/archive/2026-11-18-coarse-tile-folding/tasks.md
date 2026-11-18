# Tasks

## 1. Builder
- [x] 1.1 Move `build_tile` and `from_children` to `pictura-render/src/tile_build.rs`
- [x] 1.2 `TilePyramid::snapshot` and `any_finer_under`
- [x] 1.3 `build_folded`, with tests for `ctf_fold_cached`, `ctf_partial`, `ctf_same_job`, `ctf_stale`

## 2. App
- [x] 2.1 `TileJob` carries a snapshot, builds finer first, lands intermediate tiles
- [x] 2.2 `build_now` uses the same builder
- [x] 2.3 Test `ctf_worker`

## 3. Cheaper passes
- [x] 3.1 Interior fast path in `Region::halved`, test `ctf_halve`
- [x] 3.2 Strip builds of fresh tile rows, test `ctf_strip`
- [x] 3.3 Merged preview decodes a band's channels side by side

## 4. Close
- [x] 4.1 Measure M12 before and after, record in STATE.md
- [x] 4.2 Archive the change
