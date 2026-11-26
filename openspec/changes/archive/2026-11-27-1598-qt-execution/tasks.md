# Tasks

## 1. Tests first

- [x] 1.1 Add `scripts/test_report_execution.py` with fake-tool checks for staged ordering, opt-in overlap, one Ninja writer, separate captures, failure identity, stale cleanup, missing reports, timings, signal cleanup, nextest separation, and CI/local adapter wiring. Verify every coverage reference in `test-plan.md` names a function in that file.
- [x] 1.2 Review the fake-tool event protocol so the checks fail on an early CTest start, a second build writer, mixed self-test captures, a stale JUnit input, or a swallowed non-zero stage status. Verify each planted failure has a direct assertion.

## 2. Shared Qt executor

- [x] 2.1 Extend `scripts/test-report.sh` with `--qt-only`, `--configure`, `--build`, and repeatable `--cmake-arg` parsing while preserving managed routing, CPU-budget re-exec, colour selection, and the default local command. Verify `nextest_is_separate` and `local_adapter_uses_shared_executor`.
- [x] 2.2 Add optional configure, `pictura`-first build, all-target build tail, default-sequential self-tests, and `PICTURA_QT_SELFTEST_OVERLAP=1` concurrency. Verify `pictura_links_before_selftests`, `ctest_waits_for_build`, `one_ninja_writer`, `overlap_is_opt_in`, and `selftests_run_concurrently`.
- [x] 2.3 Keep separate self-test captures, preserve deterministic stage exit status, always run the reporter, and terminate/reap children on exit and signals. Verify `failure_identity_and_separate_captures`, `build_failure_stops_later_stages`, and `signal_cleanup_reaps_children`.
- [x] 2.4 Remove stale Qt JUnit before execution, capture CTest registration after configure, collect only current per-suite reports, and retain missing-report validation through `scripts/report_tests.py`. Verify `stale_junit_cleanup` and `missing_report_is_named`.
- [x] 2.5 Print elapsed durations for Configure, `pictura` build, Qt Test build tail, each self-test, CTest, and reporting, with explicit skipped-stage output. Verify `stage_timings_are_recorded`.

## 3. Local and CI adapters

- [x] 3.1 Route `scripts/verify-full.sh` through `scripts/verify-fast.sh --configure --build` and remove its duplicate CMake configure/build while retaining lock ownership, build-tree validation, cache proof, and control checks. Verify `local_adapter_uses_shared_executor` and the existing build-lock structure.
- [x] 3.2 Replace the CI Qt orchestration with one shared-script step, keep TIFF runtime setup before it, preserve no-PCH and sccache options, and use the full runner CPU count unless overridden. Verify `ci_adapter_uses_shared_executor`.
- [x] 3.3 Wire the fake-tool self-check into the local parallel checks and the CI Qt job without adding dependencies. Verify the adapter checks still see exactly one Qt orchestration step.

## 4. Review integration

- [x] 4.1 Inspect the real diff for process lifecycle, capture separation, exit identity, stale cleanup, missing-report naming, timing completeness, and the single-writer invariant. Verify every legitimate review finding is fixed and no build, test, lint, formatter, OpenSpec validation, or repository check command was run.
- [x] 4.2 Record #1570 as the measurement dependency and the likely `ci.yml` conflict in the change artifacts and final handoff. Verify overlap remains disabled by default and no performance claim is made.
