# Tasks

## 1. Tests named by the plan

- [x] 1.1 `pictura-app`: `a_layered_large_move_marks_tiles_instead_of_deferring`.
- [x] 1.2 Qt: `tst_large_document::aLargeMoveCommitLeavesTheTilesStale`, and keep `aLargeMoveCommitLandsOffThread` on an authoritative grayscale document.

## 2. Lazy commit

- [x] 2.1 `should_defer_move` returns false for a non-authoritative document unless the test delay forces the deferred path.
- [x] 2.2 The stale-tile path drops a stored frame before history capture.

## 3. Close-out

- [x] 3.1 `openspec validate --all --strict`, `check-test-plan.py`, one build, `verify-fast`.
- [x] 3.2 Archive the change.
