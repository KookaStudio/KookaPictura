# Tasks

## 1. Core

- [x] 1.1 `SharedTile`, `Plane::shared_tile`, `Plane::swap_tile`.
- [x] 1.2 `memory::TileCensus`, `HISTORY_SHARE`, `Governor::history_budget`.
- [x] 1.3 Unit tests: swap keeps allocation and generation; census counting; budget share.

## 2. History

- [x] 2.1 Deltas as tile references; one diff path for hinted and unhinted captures.
- [x] 2.2 Swap and apply by reference; damage per tile.
- [x] 2.3 Census bookkeeping on every path that changes what history holds; test recount.
- [x] 2.4 Budget on the census and the governor's history budget.
- [x] 2.5 Tests: `shared_tiles_counted_once`, `snapshot_costs_no_pixels`, `purge_releases_history_tiles`, `undo_redo_identity_seeded`; adapt the 64² tests.

## 3. App

- [x] 3.1 Named snapshot and restore share tiles; restore presents the stored composite.
- [x] 3.2 Remove `detached_copy` and `copy_plane`; memory gate snapshot estimates.
- [x] 3.3 M8 heap scenario mirrors the new restore; `M6x20` memory scenario.
- [x] 3.4 `tst_history_snapshots::snapshotEditRestoreRoundTrip`.

## 4. Verification

- [x] 4.1 fmt, clippy, nextest, CMake build, Qt suites.
- [x] 4.2 4250² heap guards; lower baselines that dropped.
- [x] 4.3 17000² M7/M8 and `M6x20` under 4G (manual); results in `design.md`.
- [x] 4.4 `openspec validate --all --strict`; archive.
