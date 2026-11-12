# Tasks

## 1. Tests named by the plan

- [x] 1.1 `pictura-core`: rework `second_chance_and_hot_holds`; add `hot_tiles_are_evicted_last`, `a_closed_hold_keeps_only_what_it_saw`, `a_hard_limit_evicts_at_the_ceiling`, `an_allocation_burst_is_caught_between_passes`.
- [x] 1.2 `pictura-app`: `the_view_holds_only_the_tiles_it_built`.
- [x] 1.3 Qt: `tst_control_modal_guard::aJobProgressWindowRunsToTheEnd`.

## 2. Core

- [x] 2.1 Open and closed hot holds; `hot_stamp` and the `HotSet` snapshot.
- [x] 2.2 Pass order with hot tiles last.
- [x] 2.3 Hard-limit threshold and the allocation estimate.

## 3. App

- [x] 3.1 View holds taken on the tile builds, closed, and kept per view.
- [x] 3.2 `ControlScope` leaves `jobProgress` alone.

## 4. Close-out

- [x] 4.1 Re-run the 17000² `MemoryMax=2G` cases (`te_cgroup_commit`).
- [x] 4.2 Docs: `docs/dev/STATE.md`; ROADMAP unchanged (no roadmap item for #1290).
- [x] 4.3 `openspec validate --all --strict`, `scripts/check-test-plan.py`, one build, `verify-fast`.
- [x] 4.4 Archive the change in the PR.
