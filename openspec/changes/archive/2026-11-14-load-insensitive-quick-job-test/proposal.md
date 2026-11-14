# Proposal

## Why

Issue #1313. `tst_busy_job::quickJobShowsOnlyTheWaitCursor` fails intermittently
under host load. It starts its 60 ms quick-job clock in the `start` callback,
which `runCancellableJob` invokes before it constructs the `QProgressDialog`.
When dialog and style initialization takes longer than 60 ms, the first
`running()` poll already reports the job finished, so
`QApplication::setOverrideCursor` is never reached and `sawWait` stays false.
The gate saw one failure in 8608 passes, and the issue reproduced 18 of 20
runs under the 80% CPU budget. The job runner's cursor logic is correct; the
test times the wrong interval.

## What Changes

- Start the quick-job clock on the first `running()` poll instead of in
  `start()`. That matches `runCancellableJob`'s own 250 ms clock, which starts
  after the dialog exists, and excludes dialog construction from the job's
  60 ms.
- Latch the wait-cursor witness from a zero-delay `QTimer::singleShot`
  scheduled on that first poll, so the assertion does not depend on the 16 ms
  poll timer being delivered inside the 60 ms window on a loaded host.
- Keep every existing assertion, the 60 ms quick-job deadline, and the other
  three `tst_busy_job` cases unchanged.
- No production code change. `document/long-operation-progress` scenario
  `[lop_quick_job]` already states the behavior under test and is not modified.

## Capabilities

No spec delta: this is a test-timing correction with no requirement or
behavior change (`skip_specs: true`).

### New Capabilities

None.

### Modified Capabilities

None.

## Impact

- One test file: `crates/pictura-app/cpp/tests/tst_busy_job.cpp`.
- No APIs, production behavior, dependencies, build files, docs, or roadmap
  entries.
- Verification follows the issue's repro: build `tst_busy_job` and repeat
  `quickJobShowsOnlyTheWaitCursor` under `scripts/cpu-budget.py exec` on a
  loaded host, plus the full suite once.
