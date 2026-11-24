# Tasks

## 1. Registries and ownership

- [x] 1.1 Add `selftest_codes.h` with one append-only allocation per active code, named constants, code metadata, and the retired-code set.
- [x] 1.2 Add `selftest_checks.h` with one declarative name and suite per check.
- [x] 1.3 Convert every direct failure call and indexed code table to named registry constants without changing raw values or messages.
- [x] 1.4 Reconcile the issue's historical 822-site figure with the current 729 direct failures, 37 delegated propagators, startup failures, and non-failure helper returns.
- [x] 1.5 Move each of the 186 misowned failure sites under its owning `ST_BEGIN`, preserving pass order and formats.
- [x] 1.6 Allocate code 555 to the profile-open startup failure and keep codes 552 through 554 retired.

## 2. Shared lifecycle and reporting

- [x] 2.1 Add `SelfTestRun` with one report guard, shared non-interactive setup, document cleanup, and tool restoration.
- [x] 2.2 Add `DocumentFixture` and reuse it from the paint fixture.
- [x] 2.3 Make `SelfTestReport::finish()` idempotent and keep the settable output sink.
- [x] 2.4 Add dynamic suite selection and suite-transition tokens while keeping existing checks in `core`.
- [x] 2.5 Add `failRun` for state-directory and profile-open failures before `runSelfTest()`.
- [x] 2.6 Normalize process status at the `main.cpp` boundary while preserving raw codes in tokens.
- [x] 2.7 Stop the GPU skip from returning past later checks in `selftest_paint_perf.cpp`.

## 3. Reporter and guard

- [x] 3.1 Require exactly one complete final summary per capture and reject count mismatches, trailing tokens, and malformed result tokens.
- [x] 3.2 Fail a capture whose summary reports a failure without a parseable `FAIL` token.
- [x] 3.3 Keep FAIL-wins merging and per-capture summary reporting.
- [x] 3.4 Extend `report_tests.py --self-check` with partial, trailing, multiple, count-mismatch, and summary-only failure fixtures.
- [x] 3.5 Extend `check-selftest-codes.py` to validate registries, retired and duplicate codes, code/check ownership, dynamic tables, startup failures, and skip truncation.
- [x] 3.6 Include `main.cpp` in the guard scan and add guard self-test fixtures.
- [x] 3.7 Wire the guard into both `verify-fast.sh` paths and the guards workflow.

## 4. Tests and registration

- [x] 4.1 Add Qt Test cases for a real early return, suite transitions, startup failure, registry consistency, and the shared run fixture.
- [x] 4.2 Register `tst_selftest_report`, `selftest_report.cpp`, and `selftest_run.cpp` in the Qt Test CMake target.
- [x] 4.3 Register the new self-test headers and `selftest_run.cpp` in `cmake/sources/selftest.cmake`.
- [x] 4.4 Lower the `selftest.cpp` allowlist ceiling to 6439 without adding entries.
- [x] 4.5 Update `docs/dev/testing-conventions.md` for the registries, fixture, suites, status normalization, and guards.

## 5. Verification

- [x] 5.1 Build the app and Qt Test targets after source edits. The final wrapped build passed; two earlier attempts exposed missing includes in the new fixture and Qt suite.
- [x] 5.2 Run `tst_selftest_report` and `report_tests.py --self-check`. The Qt suite passed 11 cases in 2.17 seconds and the reporter self-check passed.
- [x] 5.3 Run `check-selftest-codes.py --self-test` and its tree scan. All 13 fixtures and the tree scan passed.
- [x] 5.4 Run both headless self-tests and confirm each ends with one complete `SUMMARY`. Baseline and branch each reported `passed=467 failed=0 skipped=0` and exited 0.
- [x] 5.5 Capture a green pre-change token stream and prove byte identity for suite, name, pass detail, and order. The complete streams are byte-identical after normalizing only the pre-existing random `/tmp/pictura-*/control.sock` path; raw stderr differs only in that path.
- [x] 5.6 Run the workspace tests, Qt Test layer, file-size and self-test-budget guards, docs guard with `TASK_ALLOWS_DOCS=1`, test-plan check, and strict OpenSpec validation. The aggregate passed fmt, clippy, all 6,820 Rust tests, both self-test captures, and every post-test guard except the known milestone self-test. Its sole Qt failure, `PresentCacheTest::frameBoundedPaintMatchesCpu(gpu)`, and the milestone self-test assertion both reproduce on clean `origin/master`.
- [x] 5.7 Review `ROADMAP.md` and `PARITY.md`. This test-harness refactor changes no feature-coverage rows, so neither tracker needs an edit.
