# Tasks

## 1. Checks and evidence first

- [x] 1.1 Add `scripts/qt_test_timings.py` with `collect`, `plan`, `verify`, and
      `report` commands plus a standard-library `--self-check` covering JUnit and
      CTest parsing, duration-derived shard counts, a balanced longest-first
      split, rejection of an empty/duplicate/missing share, carry-forward from a
      partial store, the committed plan's completeness, and the CI/guard wiring.
- [x] 1.2 Add `scripts/check-qt-test-waits.py` carrying the fixed-wait policy
      with a `--self-check` that accepts a trailing or above comment and a
      zero-length wait, and fails a bare non-zero wait; keep `--changed` scoping.
- [x] 1.3 Retain Qt `time` evidence in `scripts/report_tests.py`: a `junit_timings`
      reader, the Qt `Layer.times`, and the suite seconds in the rendered output;
      extend the `--self-check` with the duration and render assertions.
- [x] 1.4 Commit the timing seed `.ci/qt-test-timings.json` and the generated
      plan `cmake/qt-test-shards.json`.

## 2. Shard membership from evidence

- [x] 2.1 Read `cmake/qt-test-shards.json` in
      `crates/pictura-app/cpp/tests/CMakeLists.txt` with `string(JSON ...)`:
      derive the sharded suite list, each suite's share count, and any per-share
      function names; register each share as `<suite>_<index>` with
      `PICTURA_TEST_SHARD` and pass the named functions on the command line.
- [x] 2.2 Keep `shardedMain` as the only scheduler: named functions run as given,
      a share with no functions falls back to the declaration-order split, and an
      empty share still fails. Add `tst_quick_mask` to the plan with two shares;
      leave `tst_document_navigate` unsharded (21 s is under the quarter budget).
- [x] 2.3 Interpret one executable per `tst_*.cpp` as one process and one CTest
      entry per suite on #1566's registry-driver model: the plan changes entry
      membership only; per-suite state isolation, JUnit path, timeout, and
      failure identity are untouched.

## 3. Wait policy in the guard path

- [x] 3.1 Run `scripts/check-qt-test-waits.py` (self-test then the scoped scan)
      from `scripts/guard.sh`, so `scripts/verify-fast.sh` and the guards CI
      workflow run it on a C++-only change.
- [x] 3.2 Run it in the Qt-aware CI job beside the shard-plan verification.
- [x] 3.3 Delete `qt_test_waits_are_justified` from
      `crates/pictura-app/src/build_config.rs` and drop it from that module's
      doc comment, only after 3.1 and 3.2 are wired.

## 4. CI retention

- [x] 4.1 Write the timing store from `scripts/test-report.sh` after the Qt layer
      and from the Qt-aware CI job, and print the longest-entry report.
- [x] 4.2 Upload `build/qt-test-results/*.xml`,
      `build/ctest-junit.xml`, and `build/qt-test-timings.json` as the
      `qt-test-timings` artifact on every run.
- [x] 4.3 Add the shard-plan verification step so an incomplete plan fails CI.

## 5. Review integration

- [x] 5.1 Inspect the diff: no suite assertion, state-isolation fixture, timeout,
      or JUnit name changed; the plan only selects existing functions.
- [x] 5.2 Record that no build, test, lint, formatter, validator, or measurement
      was run and that the first CI run is what proves the CMake JSON parse.
