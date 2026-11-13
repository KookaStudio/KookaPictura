# Tasks

## 1. Stabilise the quick-job case

- [x] 1.1 In `BusyJobTest::quickJobShowsOnlyTheWaitCursor`, move the clock start from the `start` lambda to the first `running()` poll, guarded by a `started` flag, and keep the 60 ms deadline. Arm `QTimer::singleShot(0, &probeContext, ...)` on that first poll using a local `QObject probeContext`, latch `sawWait`, keep the existing per-poll cursor sample, and add the short comment that records why the clock starts on the poll. Verify the diff touches only `crates/pictura-app/cpp/tests/tst_busy_job.cpp`.
- [x] 1.2 Build the test: `scripts/cpu-budget.py exec -- cmake --build build --target tst_busy_job` (configure `build/` first only when absent).
- [x] 1.3 Run `ctest --test-dir build -R '^tst_busy_job$' --output-on-failure`; verify all four cases pass, including the unchanged slow, Esc, and busy-bar cases.
- [x] 1.4 Run `QT_QPA_PLATFORM=offscreen scripts/cpu-budget.py exec -- build/crates/pictura-app/cpp/tests/tst_busy_job quickJobShowsOnlyTheWaitCursor` 20 times; verify 20 of 20 pass, then repeat a few times with `PICTURA_CPU_BUDGET_PERCENT=5` and verify they pass.

## 2. Ship

- [x] 2.1 Run `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`; verify both pass.
- [x] 2.2 Run `openspec archive load-insensitive-quick-job-test` first, then `bash scripts/verify-fast.sh` (the fast gate runs `check-openspec-archived.sh` and fails on a non-archived change); verify the change lands under `openspec/changes/archive/` and the gate passes.
- [ ] 2.3 Commit the test fix as `test(app): time the quick-job case from its first poll (#1313)` and the archive move as `chore(openspec): archive load-insensitive-quick-job-test (#1313)`, open the pull request titled `test(app): make the quick-job wait-cursor test load-insensitive (#1313)` with the test plan's manual rows listed as a reviewer checklist, and verify the commits pass the conventional-commit check.
