# Tasks

## 1. Builder
- [ ] 1.1 Move `build_tile` and `from_children` to `pictura-render/src/tile_build.rs`
- [ ] 1.2 `TilePyramid::snapshot` and `any_finer_under`
- [ ] 1.3 `build_folded`, with tests for `ctf_fold_cached`, `ctf_partial`, `ctf_same_job`, `ctf_stale`

## 2. App
- [ ] 2.1 `TileJob` carries a snapshot, builds finer first, lands intermediate tiles
- [ ] 2.2 `build_now` uses the same builder
- [ ] 2.3 Test `ctf_worker`

## 3. Cheaper passes
- [ ] 3.1 Interior fast path in `Region::halved`, test `ctf_halve`

## 4. Close
- [ ] 4.1 Measure M12 before and after, record in STATE.md
- [ ] 4.2 Archive the change
