# Spec Delta

## MODIFIED Requirements

### Requirement: Unified test report

The system SHALL provide a unified end-of-run test report spanning every test
layer: Rust unit, integration, and oracle tests, the C++
application self-test, and the C++ Qt Test executables. `scripts/test-report.sh`
SHALL run the workspace suite under `cargo nextest run --workspace --no-fail-fast`,
run the self-test once with the `two_layers.psd` document fixture, run
the Qt Test layer through `ctest --test-dir build -R '^tst_' --output-on-failure -j
<level>` (CTest registers each executable under
`build/crates/pictura-app/cpp/tests/` and passes
`-o <CMAKE_BINARY_DIR>/qt-test-results/<name>.xml,junitxml`, so each emits a
per-executable JUnit report), capture each layer's output, and invoke
`scripts/report_tests.py`. The CTest parallel level SHALL default to the local
CPU budget (`PICTURA_JOBS`, itself 80% of the allowed CPUs by default, at least
one), and SHALL be overridable through `CTEST_PARALLEL_LEVEL`, so the suites use
the machine without launching every offscreen process at once and without
taking 80% of an already-reduced slice. The CI Qt Test job, which
has its runner to itself, SHALL default to the runner's CPU count, keeping the
same override. The script SHALL
clear `build/qt-test-results/*.xml` before the CTest run so a removed, renamed,
or crashed suite cannot be counted from a stale file. The reporter
SHALL fold the per-executable JUnit from `build/qt-test-results/*.xml` and SHALL
parse it with the same JUnit parser it already uses for nextest. The reporter
SHALL print, for each suite, a line with its passed and skipped counts, a
subtotal per layer, a TOTAL line, and, when there are failures, a FAILURES
section listing every failed test with its message. The reporter SHALL use only
the python3 standard library, SHALL exit non-zero when any layer fails, and SHALL
accept `--color auto|always|never`. `scripts/verify-fast.sh` and
`scripts/verify-full.sh` SHALL both route through this same capture-and-report
flow, and nextest's own progress output SHALL remain visible while the run
executes.

#### Scenario: Every layer is summarised

- **WHEN** all Rust tests, self-test checks, and Qt Test cases pass
- **THEN** the report lists a line per Rust suite with its pass count, an
  app-self-test subtotal, a Qt Test subtotal, and a TOTAL with
  the aggregate passed, skipped, and failed counts, and the reporter exits 0

#### Scenario: A failure is listed and fails the run

- **WHEN** any test layer reports a failure
- **THEN** the reporter prints a FAILURES section naming each failed test with
  its message and exits non-zero

#### Scenario: Colour output is selectable

- **WHEN** the reporter is invoked with `--color never`
- **THEN** the report contains no ANSI escape sequences, and with `--color
  always` it contains them even when stdout is not a terminal

#### Scenario: The Qt Test layer is folded in

- **WHEN** `scripts/test-report.sh` runs with built Qt Test executables under
  the build tree at `build/crates/pictura-app/cpp/tests/tst_*`
- **THEN** it runs `ctest -R '^tst_'` with the configured parallel level, CTest
  emits per-executable JUnit under `build/qt-test-results/`, the reporter parses
  that JUnit with the same parser it uses for nextest, and the layer's passed
  and skipped counts appear in the report

#### Scenario: Qt Test suites run concurrently [tr_qt_parallel]

- **WHEN** `scripts/test-report.sh` runs with the default parallel level on a machine with 16 available CPUs
- **THEN** CTest runs up to twelve suites at once, each suite still writes its own JUnit file, and a failure in any suite fails the report

#### Scenario: the parallel level stays overridable [tr_qt_parallel_override]

- **WHEN** `scripts/test-report.sh` runs with `CTEST_PARALLEL_LEVEL=2`
- **THEN** CTest runs up to two suites at once regardless of the CPU count

#### Scenario: a one-CPU machine still runs the layer [tr_qt_parallel_floor]

- **WHEN** `scripts/test-report.sh` runs on a machine with one available CPU and no override
- **THEN** CTest runs with a parallel level of one

#### Scenario: stale Qt Test results are not counted [tr_stale_junit]

- **WHEN** a Qt Test executable from an earlier run no longer exists and the report runs again
- **THEN** its old JUnit file was removed before CTest started and it does not appear in the report

### Requirement: The test layers run concurrently

The unified report SHALL run the test layers that do not share the workspace
build directory at the same time: the application self-test invocation SHALL
run concurrently with the Qt Test layer, so the report's wall time is not the
sum of those layers' durations. The layers SHALL still each produce their own
capture, and a failure in any layer SHALL still fail the report. This change
SHALL NOT add nextest to that concurrent group.

#### Scenario: the self-test layer overlaps the Qt Test layer [tr_layers_concurrent]

- **WHEN** `scripts/test-report.sh` runs with a built application and built Qt Test executables
- **THEN** the single combined self-test runs while the Qt Test layer runs, and the report's wall time is below the sum of those two layers' individual durations

## ADDED Requirements

### Requirement: One in-process self-test capture preserves the unique check set

The unified report SHALL launch one application self-test process with the
`two_layers.psd` fixture. The process SHALL start on the scratch document and
run `fresh_white` while one mutable boolean is false. It SHALL
then open the fixture in-process, close the scratch document after a successful
open, update the active view and image, and set that same boolean from the open
result. The 36 document-only checks SHALL run only when the boolean is true.
The 466 shared checks SHALL run once. The combined
capture SHALL contain the same unique `(suite, name)` check keys as the merged
legacy bare and document captures, currently 503.

#### Scenario: One process produces one summary [tr_single_document_capture]

- **WHEN** `scripts/test-report.sh` runs with a built application
- **THEN** it starts one self-test process with `two_layers.psd`, passes one capture to the reporter, and that capture ends with exactly one SUMMARY

#### Scenario: The combined capture keeps the legacy unique checks [tr_unique_checks_preserved]

- **WHEN** the 466 shared checks, `fresh_white`, and 36 document-only checks pass
- **THEN** the report contains 503 unique check entries, with no duplicate shared checks

#### Scenario: Mode-specific checks stay mutually exclusive [tr_mode_conditional_checks]

- **WHEN** the self-test starts and the document has not opened
- **THEN** it runs `fresh_white` and does not run the document-only block, and after a successful in-process open it runs the document-only block without running `fresh_white` again

### Requirement: A single self-test capture keeps its summary and unique entries

The reporter SHALL use a single complete capture's SUMMARY as the layer total
and SHALL build the displayed check entries from that capture's unique
`(suite, name)` keys. Multi-capture merging SHALL keep its existing FAIL-wins
behavior and recomputed unique totals.

#### Scenario: One summary remains authoritative [tr_single_summary_authoritative]

- **WHEN** one complete self-test capture contains unique checks and a matching SUMMARY
- **THEN** the app-self-test subtotal equals that SUMMARY and the listed entries equal the capture's unique check keys
