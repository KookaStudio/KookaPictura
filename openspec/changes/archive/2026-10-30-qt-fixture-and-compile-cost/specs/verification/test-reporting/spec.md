## MODIFIED Requirements

### Requirement: Unified test report

The system SHALL provide a unified end-of-run test report spanning every test
layer: Rust unit, integration, and oracle tests, the C++
application self-test, and the C++ Qt Test executables. `scripts/test-report.sh`
SHALL run the workspace suite under `cargo nextest run --workspace --no-fail-fast`,
run the self-test both with and without a document fixture, run
the Qt Test layer through `ctest --test-dir build -R '^tst_' --output-on-failure -j
<level>` (CTest registers each executable under
`build/crates/pictura-app/cpp/tests/` and passes
`-o <CMAKE_BINARY_DIR>/qt-test-results/<name>.xml,junitxml`, so each emits a
per-executable JUnit report), capture each layer's output, and invoke
`scripts/report_tests.py`. The CTest parallel level SHALL default to 80% of the
CPUs available to the run, rounded down and at least one, and SHALL be
overridable through `CTEST_PARALLEL_LEVEL`, so the suites use the machine
without launching every offscreen process at once. The CI Qt Test job, which
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
