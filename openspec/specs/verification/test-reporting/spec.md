# test-reporting Specification

## Purpose
The unified test report: structured C++ self-test output and nextest JUnit output.

## Requirements

### Requirement: Unified test report

The system SHALL provide a unified end-of-run test report spanning every test
layer: Rust unit, integration, and oracle tests, Rust doctests, the C++
application self-test, and the C++ Qt Test executables. `scripts/test-report.sh`
SHALL run the workspace suite under `cargo nextest run --workspace --no-fail-fast`,
run the doctests, run the self-test both with and without a document fixture, run
the Qt Test layer through `ctest --test-dir build -R '^tst_' --output-on-failure -j
<level>` (CTest registers each executable under
`build/crates/pictura-app/cpp/tests/` and passes
`-o <CMAKE_BINARY_DIR>/qt-test-results/<name>.xml,junitxml`, so each emits a
per-executable JUnit report), capture each layer's output, and invoke
`scripts/report_tests.py`. The CTest parallel level SHALL default to four and
SHALL be overridable through `CTEST_PARALLEL_LEVEL`, so the suites run
concurrently without launching every offscreen process at once. The script SHALL
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

- **WHEN** all Rust tests, doctests, self-test checks, and Qt Test cases pass
- **THEN** the report lists a line per Rust suite with its pass count, an
  app-self-test subtotal, a Qt Test subtotal, a doctest line, and a TOTAL with
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

- **WHEN** `scripts/test-report.sh` runs with the default parallel level
- **THEN** CTest runs up to four suites at once, each suite still writes its own JUnit file, and a failure in any suite fails the report

#### Scenario: stale Qt Test results are not counted [tr_stale_junit]

- **WHEN** a Qt Test executable from an earlier run no longer exists and the report runs again
- **THEN** its old JUnit file was removed before CTest started and it does not appear in the report

### Requirement: Structured C++ self-test output

The C++ application self-test SHALL emit machine-readable result tokens on
standard error, one per check, in the form `pictura self-test: SUITE <suite> <name>`,
`pictura self-test: PASS <suite> <name> [detail]`, `pictura self-test: SKIP
<suite> <name> <reason>`, and `pictura self-test: FAIL <suite> <name> <code>
<message>`, followed by a single `pictura self-test: SUMMARY passed=<n>
failed=<n> skipped=<n>` line. Every check SHALL carry an explicit suite and name;
the suite SHALL be the milestone prefix already present in the check name (for
example `m47_float_close` belongs to suite `m47`), and checks without a milestone
prefix SHALL belong to `core` or `misc`. A failing check's `<code>` SHALL be the
check's existing process exit code, and the self-test's exit-code contract SHALL
remain unchanged. The harness SHALL be implemented in
`crates/pictura-app/cpp/selftest_report.h` and `.cpp`. Instrumentation SHALL
replace the existing progress and failure output in place rather than add to it,
and `crates/pictura-app/cpp/selftest.cpp` SHALL NOT exceed its
`scripts/file-size-allowlist.txt` ceiling; the allowlist entry SHALL be lowered
to the resulting size.

#### Scenario: A passing check emits a suite and pass token

- **WHEN** a self-test check succeeds
- **THEN** its suite token precedes a PASS token carrying the same suite and the
  check's name

#### Scenario: A skipped check emits a skip token with a reason

- **WHEN** a self-test check is skipped
- **THEN** a SKIP token names the check and gives the reason it was not run

#### Scenario: A failing check emits a fail token and keeps its exit code

- **WHEN** a self-test check fails
- **THEN** a FAIL token carries the check's name, its exit code, and its message,
  and the process returns that same exit code

#### Scenario: The summary aggregates the tokens

- **WHEN** the self-test finishes
- **THEN** the SUMMARY line's passed, failed, and skipped counts equal the counts
  of the PASS, FAIL, and SKIP tokens emitted

#### Scenario: The self-test source stays within its size budget

- **WHEN** the self-test instrumentation is complete
- **THEN** `scripts/check-file-size.sh` passes with an allowlist entry for
  `selftest.cpp` no larger than the file's new size

### Requirement: nextest configuration and JUnit output

The repository SHALL provide `.config/nextest.toml` that configures the nextest
status levels so per-test progress streams during the run, disables fail-fast so
all failures are reported, and writes a JUnit report to
`target/nextest/default/junit.xml`. The workspace test invocation SHALL emit that
JUnit file, and CI SHALL upload it as a workflow artifact.

#### Scenario: A nextest run writes JUnit XML

- **WHEN** the workspace suite runs under nextest with the repository config
- **THEN** a JUnit XML file exists at `target/nextest/default/junit.xml`

#### Scenario: CI uploads the JUnit artifact

- **WHEN** the CI test job completes
- **THEN** the workflow uploads the JUnit XML as an artifact

#### Scenario: All failures are reported

- **WHEN** the workspace suite runs through `scripts/test-report.sh`
- **THEN** nextest runs with `--no-fail-fast` so the report includes every
  failing test, not only the first

### Requirement: A registered suite that produced no report fails the run

Each Qt Test suite writes its own JUnit report from inside the test
executable, so a suite that never started — because its executable is missing
from a stale build tree, or because it crashed before writing — leaves no
report behind. The unified report SHALL compare the suites CTest registered
against the reports that were produced, and SHALL fail the run, naming every
registered suite that produced none. A suite that produced no report SHALL NOT
be counted as passing, skipped, or absent.

#### Scenario: a missing suite executable fails the report [tr_missing_suite_fails]

- **WHEN** a CTest-registered suite has no executable in the build tree
- **THEN** the report names that suite as having produced no report and the run exits non-zero

#### Scenario: a crashed suite fails the report [tr_crashed_suite_fails]

- **WHEN** a suite starts but dies before writing its JUnit report
- **THEN** the report names that suite as having produced no report and the run exits non-zero

### Requirement: The test layers run concurrently

The unified report SHALL run the test layers that do not share the workspace
build directory at the same time: the two application self-test invocations
SHALL run concurrently with each other and with the Qt Test layer, so the
report's wall time is not the sum of the layers' durations. The layers SHALL
still each produce their own capture, and a failure in any layer SHALL still
fail the report.

#### Scenario: the self-test layer overlaps the Qt Test layer [tr_layers_concurrent]

- **WHEN** `scripts/test-report.sh` runs with a built application and built Qt Test executables
- **THEN** both self-test invocations run while the Qt Test layer runs, and the report's wall time is below the sum of the layers' individual durations
