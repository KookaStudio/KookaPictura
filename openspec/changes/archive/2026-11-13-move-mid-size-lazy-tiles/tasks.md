# Tasks

## 1. Tests named by the plan

- [x] 1.1 `pictura-app`: `a_mid_sized_layered_move_still_marks_tiles`.
- [x] 1.2 Qt: `tst_large_document::aMidSizedMoveCommitWaitsForTheTiles`.
- [x] 1.3 Qt: `tst_large_document::aSecondMoveInheritsThePendingWait`.
- [x] 1.4 `pictura-app`: `a_lazy_move_paints_its_preview_into_the_stale_tiles`; `pictura-render`: `a_stand_in_paints_only_stale_tiles`.
- [x] 1.5 Qt: `tst_large_document::aLayeredTransformShowsItsResultAtOnce`; `pictura-app`: `a_reduced_stand_in_shows_the_document_in_the_stale_tiles`.

## 2. Decision

- [x] 2.1 Drop the dirty-area floor from `should_leave_tiles`; keep it for `should_defer_move`.
- [x] 2.2 Clear a lazy flag with nothing to wait for without emitting `moveCommitted`.
- [x] 2.3 A new Move commit takes over a pending lazy wait without reporting it.
- [x] 2.4 Paint a lazy view-preview Move's result into the stale presented tiles as their stand-in.
- [x] 2.5 Paint a reduced composite into the stale tiles after a lazy transform commit and report it at once.

## 3. Close-out

- [x] 3.1 `openspec validate --all --strict`, `check-test-plan.py`, one build, tests.
- [x] 3.2 Archive the change.
