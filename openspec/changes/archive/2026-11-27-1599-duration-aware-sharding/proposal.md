# Proposal

## Why

The Qt Test layer takes 389 s median and 393 s in warm run #1592. Sharding is a
hand-maintained list split in declaration order, and only a small subset of
suites is sharded: `tst_quick_mask` has a recorded 112 s run and stays unsharded.
`scripts/report_tests.py` reads each JUnit report but discards its `time`
attributes, so nothing retains the evidence a shard decision needs.

The fixed-wait policy has the same split: `qt_test_waits_are_justified` lives in
the Rust module `crates/pictura-app/src/build_config.rs`. A C++-only pull request
skips that test module, so the policy does not run where it is needed. Timing
evidence and the sharding policy live apart; the wait policy lives in the wrong
execution path.

## What Changes

- Retain per-suite and per-function duration evidence. `scripts/report_tests.py`
  keeps each Qt JUnit suite's `time` and the reporter shows it; the new
  `scripts/qt_test_timings.py` collects every report plus CTest's per-entry
  wall time into one store, and CI uploads the raw JUnit/CTest XML and the
  store as an artifact.
- Choose shard membership and count from the retained evidence. The plan lives
  in `cmake/qt-test-shards.json` (written by `qt_test_timings.py plan`) and is
  read by the test CMake registry: a measured share names its exact functions
  and falls back to the existing declaration-order split otherwise. The plan
  reuses `shardedMain`; no second scheduler exists. `tst_quick_mask` enters the
  plan with two shares, and `tst_document_navigate` stays out because its 21 s
  is below the quarter-of-the-layer threshold the planner uses.
- Keep explicit test-function selection and empty-shard detection unchanged:
  `shardedMain` still runs named functions as given and fails a share that holds
  no functions. `qt_test_timings.py verify` refuses a plan with an empty,
  duplicate, or missing share, and `--self-check` proves it.
- Interpret the "one executable per `tst_*.cpp`" constraint as one process and
  one CTest entry per suite, building on #1566's registry-driver model: the
  constraint protects suite isolation (a crash names one suite, JUnit is
  per-suite, the timeout is per-suite), which the driver model preserves while
  the shard plan only changes entry membership.
- Move the fixed-wait policy from the Rust scanner into
  `scripts/check-qt-test-waits.py`, a repository guard run by `scripts/guard.sh`
  (so local verification and the guards CI workflow run it) and by the Qt-aware
  CI job. Delete `qt_test_waits_are_justified` only after the guard is wired
  into all three paths.
- Report the longest entry against the quarter-of-the-layer budget from the
  retained timings, so the `qt-test-harness` longest-entry rule is checked,
  not assumed.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `verification/test-reporting`: the Qt layer retains duration evidence, the
  timing store is written, and the longest entry is reported against the
  quarter budget.
- `verification/qt-test-harness`: shard membership and count come from retained
  duration evidence through the plan, and the fixed-wait requirement names its
  repository guard instead of a Rust test.
- `verification/verification-harness`: the non-goal guard gains the fixed-wait
  check, so it runs on C++-only changes.

## Impact

- Scripts: new `scripts/qt_test_timings.py` and `scripts/check-qt-test-waits.py`;
  `scripts/report_tests.py` keeps Qt `time` evidence; `scripts/test-report.sh`
  writes the timing store; `scripts/guard.sh` runs the wait check.
- CMake and data: `crates/pictura-app/cpp/tests/CMakeLists.txt` reads
  `cmake/qt-test-shards.json`; that file and `.ci/qt-test-timings.json` are
  tracked.
- CI: `.github/workflows/ci.yml` uploads the Qt timing artifacts, writes and
  reports the store, and verifies the shard plan and waits.
- Rust: `qt_test_waits_are_justified` is removed from
  `crates/pictura-app/src/build_config.rs` after the guard is wired.
- No dependency is added. Suite assertions, per-suite state isolation, timeouts,
  failure identity, and JUnit output are unchanged.
