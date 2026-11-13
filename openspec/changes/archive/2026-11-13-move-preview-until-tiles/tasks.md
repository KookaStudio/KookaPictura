# Tasks

## 1. Tests named by the plan

- [x] 1.1 Qt: `tst_large_document::aLargeMoveCommitLeavesTheTilesStale` now asserts the preview survives until `moveCommitted` and then drops.
- [x] 1.2 `pictura-app`: keep `a_layered_large_move_marks_tiles_instead_of_deferring` covering the lazy decision.

## 2. Preview retention

- [x] 2.1 `lazy_move_pending` on the view state and in `move_composite_pending`.
- [x] 2.2 Queue the presented tiles after a lazy commit.
- [x] 2.3 Emit `moveCommitted` when the presented tiles are current, and on a reader settle.

## 3. Close-out

- [x] 3.1 `openspec validate --all --strict`, `check-test-plan.py`, one build, `verify-fast`.
- [x] 3.2 Archive the change.
