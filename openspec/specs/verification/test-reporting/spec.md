# test-reporting Specification

## Purpose
The unified test report: structured C++ self-test output and nextest JUnit output.

## Requirements

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

### Requirement: Structured C++ self-test output

The C++ application self-test SHALL emit machine-readable result tokens on
standard error in the form `pictura self-test: SUITE <suite> <name>`,
`pictura self-test: PASS <suite> <name> [detail]`, `pictura self-test: SKIP
<suite> <name> <reason>`, and `pictura self-test: FAIL <suite> <name> <code>
<message>`. Each run SHALL end with exactly one `pictura self-test: SUMMARY
passed=<n> failed=<n> skipped=<n>` line. The summary SHALL be the final machine
token, and its counts SHALL equal the PASS, FAIL, and SKIP token counts in that
capture.

Every check SHALL declare its name and suite in `selftest_checks.h`. The reporter
SHALL emit a SUITE token when the declared suite changes. Existing checks SHALL
remain in suite `core` so their green token bytes stay stable. A FAIL token SHALL
name the check that owns its allocated code, never an earlier check. A PASS or
SKIP for that check before the FAIL SHALL NOT change the name it carries.

The summary SHALL be emitted on every terminating path, including a direct or
delegated failure, the state-directory failure before `runSelfTest()`, the
profile-open failure before a frame exists, and a successful run. A failing
check's raw code SHALL remain positive and unchanged in its FAIL token. The
process exit status SHALL be non-zero for every failure and zero when no check
fails. The status SHALL equal the raw code when it is in 1..255 and SHALL be 1
otherwise.

The harness SHALL be implemented in `crates/pictura-app/cpp/selftest_report.h`
and `.cpp`, and its output SHALL be writable to a settable sink that defaults to
standard error. `crates/pictura-app/cpp/selftest.cpp` SHALL NOT exceed its
`scripts/file-size-allowlist.txt` ceiling.

#### Scenario: A passing check emits a suite and pass token

- **WHEN** a self-test check succeeds
- **THEN** a SUITE token precedes a PASS token carrying the same suite and the
  check's name

#### Scenario: A skipped check emits a skip token with a reason

- **WHEN** a self-test check is skipped
- **THEN** a SKIP token names the check and gives the reason it was not run

#### Scenario: A failing check emits a fail token and keeps its exit code

- **WHEN** a self-test check fails
- **THEN** a FAIL token carries the check's name, its allocated code, and its
  message, and the process exit status is non-zero

#### Scenario: A failure never exits zero [tr_failure_code_nonzero]

- **WHEN** a check fails with a code the process status cannot carry as-is,
  including a code that is a whole multiple of 256
- **THEN** the FAIL token keeps the raw code and the process exits with 1

#### Scenario: The summary aggregates the tokens

- **WHEN** the self-test finishes
- **THEN** exactly one final SUMMARY reports the PASS, FAIL, and SKIP token
  counts

#### Scenario: A failure before the suite ends still reports a summary [tr_summary_on_failure]

- **WHEN** a check fails and returns through one or more delegated suites
- **THEN** the run emits exactly one final SUMMARY whose failed count includes
  that check, and the process exits non-zero

#### Scenario: A failure before runSelfTest still reports a summary [tr_state_home_summary]

- **WHEN** the self-test cannot create its isolated state directory
- **THEN** the capture contains a FAIL token for `state_home_temp_dir` and one
  final SUMMARY

#### Scenario: A profile-open failure still reports a summary [tr_profile_summary]

- **WHEN** the self-test cannot open its profile before a frame exists
- **THEN** the capture contains a FAIL token for `profile_open` with code 555 and
  one final SUMMARY

#### Scenario: A fail token names its own check [tr_fail_names_its_check]

- **WHEN** a check fails during fixture setup
- **THEN** its FAIL token carries that check's registered name, not the name of
  the preceding check

#### Scenario: A suite transition emits a new suite token [tr_suite_transition]

- **WHEN** consecutive checks declare different suites
- **THEN** the reporter emits a SUITE token for the new suite before that check's
  result token

#### Scenario: The self-test source stays within its size budget

- **WHEN** the self-test instrumentation is complete
- **THEN** `scripts/check-file-size.sh` passes with the `selftest.cpp` allowlist
  entry no larger than the file

### Requirement: Failure codes are allocated centrally and append-only

`selftest_codes.h` SHALL own every active failure-code value and its named
constant. Each value SHALL be positive and unique. Every failure site SHALL use a
named constant from that registry. Retired codes SHALL remain reserved and SHALL
NOT be allocated again. A new failure SHALL append a new value without
renumbering an existing value. When an existing code is shared, the registry SHALL
list every owning check explicitly.

#### Scenario: Duplicate or raw codes are rejected [ca_unique]

- **WHEN** the registry contains a duplicate value or a failure site uses a raw
  integer
- **THEN** guard reports the file and line and exits non-zero

#### Scenario: Retired codes stay retired [ca_retired]

- **WHEN** an active allocation or failure site uses a retired code
- **THEN** the guard reports the violation and exits non-zero

#### Scenario: New startup code is appended [ca_append]

- **WHEN** the profile-open startup failure is added
- **THEN** it receives code 555 and no existing code changes value

#### Scenario: Shared code ownership is explicit [ca_shared]

- **WHEN** an existing code has more than one owning check
- **THEN** the registry lists every owner and the guard rejects any undeclared
  check that uses the code

### Requirement: Checks and suites are declarative

`selftest_checks.h` SHALL own every check name and suite. Every `ST_BEGIN` and
startup `failRun` SHALL name a registered check. The reporter SHALL obtain suite
selection from that registry rather than a hardcoded member value.

#### Scenario: Unknown checks are rejected [dc_registered]

- **WHEN** a source begins or fails a check absent from the registry
- **THEN** the guard reports the file and line and exits non-zero

#### Scenario: Suite metadata drives tokens [dc_suite]

- **WHEN** a registered check declares a suite
- **THEN** its SUITE and result tokens carry that suite, while existing `core`
  registrations keep their token bytes

### Requirement: The self-test has one shared fixture lifecycle

`SelfTestRun` SHALL own the run-scope report guard, non-interactive prompt setup,
document cleanup, and tool restoration. Seeded document checks SHALL reuse the
shared `DocumentFixture` cleanup primitive. A check that returns early SHALL not
leak a document created during the run.

#### Scenario: Early return cleans the run [fx_cleanup]

- **WHEN** a run opens a document and exits early
- **THEN** the run fixture closes documents created after entry and restores the
  previous active tool

#### Scenario: Paint checks reuse document cleanup [fx_paint_reuse]

- **WHEN** a paint fixture exits
- **THEN** it removes its seed PNG and closes its document through the shared
  document fixture

### Requirement: A truncated or inconsistent self-test stream fails the report

The unified report SHALL require every captured self-test stream to contain
exactly one complete SUMMARY as its final machine token. It SHALL reject missing,
partial, multiple, trailing, malformed, and count-mismatched summaries. A summary
that reports failures SHALL fail the run even when no parseable FAIL token exists.

#### Scenario: A stream that dies before its summary fails the run [tr_truncated_selftest_fails]

- **WHEN** a capture has result tokens but no SUMMARY
- **THEN** the report names that capture as truncated and exits non-zero

#### Scenario: A partial summary is rejected [tr_partial_summary]

- **WHEN** a SUMMARY omits one of its three counts
- **THEN** the report names the invalid summary and exits non-zero

#### Scenario: Tokens after the summary are rejected [tr_trailing_tokens]

- **WHEN** a result token follows the SUMMARY
- **THEN** the report names the trailing token and exits non-zero

#### Scenario: Summary counts must match tokens [tr_summary_counts]

- **WHEN** a SUMMARY count differs from the number of corresponding result tokens
- **THEN** the report names both counts and exits non-zero

#### Scenario: A summary-only failure is visible [tr_summary_only_failure]

- **WHEN** a complete SUMMARY reports a failure but the capture has no parseable
  FAIL token
- **THEN** the report names the capture's failed count and exits non-zero

### Requirement: A merged self-test failure stays visible

Merging self-test captures SHALL NOT let a later capture erase an earlier failure.
For a check present in more than one capture, a FAIL in any capture SHALL make the
merged check a failure. The last capture's status SHALL win only among PASS and
SKIP.

#### Scenario: A fail in one capture is not overwritten by a pass in another [tr_merged_fail_not_hidden]

- **WHEN** one capture reports FAIL for a check and another reports PASS
- **THEN** the merged report counts that check as failed and exits non-zero

### Requirement: Each self-test capture's own summary is reported

The unified report SHALL show each capture's SUMMARY counts beside the merged
total. A capture's counts SHALL come from its own SUMMARY, not from deduplication
across captures.

#### Scenario: Two captures report their own totals [tr_per_capture_summary]

- **WHEN** two complete captures have different SUMMARY counts
- **THEN** the report shows both sets of counts and the merged subtotal

### Requirement: The self-test failure invariants are pinned by a guard

The repository SHALL provide a self-testing guard that validates both registries
and every failure source, including `main.cpp`. It SHALL reject raw, duplicate,
unregistered, retired, non-positive, or check-mismatched codes, a failure before
its owning `ST_BEGIN`, and a `ST_SKIP` followed by a return that leaves later
checks in the same function. It SHALL run in both fast gate paths and the guards
CI job.

#### Scenario: Invalid code allocation is rejected [sc_code_nonzero]

- **WHEN** a fixture uses a raw, duplicate, retired, or unknown code
- **THEN** the guard reports the violation and exits non-zero

#### Scenario: A fail site outside its own check block is rejected [sc_fail_names_check]

- **WHEN** a failure code belongs to a later check but appears before that
  check's `ST_BEGIN`
- **THEN** the guard reports the mismatch, while a failure after its own PASS is
  accepted

#### Scenario: A skip that truncates its suite is rejected [sc_skip_truncation]

- **WHEN** a check emits `ST_SKIP` and returns while later `ST_BEGIN` calls remain
  in the same function
- **THEN** the guard reports the file and line

#### Scenario: Indexed code and name tables are checked [sc_code_table]

- **WHEN** a loop uses `names[i]` and `codes[i]`
- **THEN** the guard checks each code against its corresponding registered name

#### Scenario: The guard is exercised by its own fixture [sc_guard_self_test]

- **WHEN** the guard runs with `--self-test`
- **THEN** it reports every planted violation and accepts the clean fixtures

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

The two application self-test invocations SHALL run concurrently with each
other and SHALL retain separate captures. When
`PICTURA_QT_SELFTEST_OVERLAP=1`, they SHALL also run while the remaining Qt Test
targets build after `pictura` has linked. The overlap flag SHALL default to
disabled, and CI and local adapters SHALL NOT enable it until #1570 records a
net wall-time improvement on four vCPUs without new flakes or CPU-contention
regressions. CTest SHALL wait for the full Qt Test build and both self-test runs
to finish. A failure in any layer SHALL still fail the report.

#### Scenario: the self-test layer overlaps the Qt Test layer [tr_layers_concurrent]

- **WHEN** the shared Qt executor runs with a built application and built Qt Test executables
- **THEN** both self-test invocations run concurrently with each other, each keeps its own capture, and a failure in either fails the report

#### Scenario: build-tail overlap is opt-in [qe_overlap_opt_in]

- **WHEN** `PICTURA_QT_SELFTEST_OVERLAP=1` and the Qt Test build tail is still running after `pictura` links
- **THEN** both self-test invocations start before the build tail finishes, and CTest starts only after the build tail and both self-test runs finish

#### Scenario: overlap stays disabled before measurement [qe_overlap_default]

- **WHEN** the shared Qt executor runs without `PICTURA_QT_SELFTEST_OVERLAP=1`
- **THEN** it waits for both self-test runs before starting the remaining Qt Test build

### Requirement: One shared Qt execution implementation

CI and local verification SHALL use `scripts/test-report.sh` as the single Qt
execution implementation. The script SHALL own optional configuration, staged
builds, self-test captures, stale-result cleanup, CTest selection, missing-report
validation, and reporter invocation. Workflow-only setup SHALL remain in the CI
adapter. Rust nextest SHALL remain a separate stage and SHALL NOT overlap the Qt
build, self-test, or CTest stages in this change.

#### Scenario: CI delegates Qt execution [qe_ci_shared]

- **WHEN** the CI Qt job configures, builds, self-tests, runs CTest, and reports Qt results
- **THEN** it invokes the same `scripts/test-report.sh` Qt mode used by local verification and does not duplicate that ordering in YAML

#### Scenario: the local full gate delegates Qt execution [qe_local_adapter]

- **WHEN** `scripts/verify-full.sh` runs
- **THEN** it reaches the shared Qt executor through `scripts/verify-fast.sh` with configure and build enabled, and performs no separate CMake build

#### Scenario: nextest stays separate [qe_nextest_separate]

- **WHEN** the shared script runs its default local mode
- **THEN** nextest finishes as its own stage before Qt configuration, builds, self-tests, or CTest begin

### Requirement: Qt builds are staged with one Ninja writer

When building is requested, the shared Qt executor SHALL configure first, build
and link `pictura` before starting either self-test invocation, and then build
all remaining targets. It SHALL run at most one `cmake --build` process against
the build tree at a time. CTest SHALL start only after the full build succeeds
and every required Qt Test executable exists.

#### Scenario: pictura links before self-tests [qe_pictura_first]

- **WHEN** the shared Qt executor performs a staged build
- **THEN** the `pictura` target build completes before either self-test invocation starts

#### Scenario: CTest waits for every Qt executable [qe_ctest_after_build]

- **WHEN** the remaining Qt Test targets are still compiling or linking
- **THEN** CTest has not started, and after the full build succeeds CTest runs only then

#### Scenario: one Ninja writer owns the tree [qe_single_ninja]

- **WHEN** self-tests overlap an enabled Qt Test build tail
- **THEN** no second `cmake --build` or Ninja process runs against the same build tree

### Requirement: Qt execution preserves failure identity and process cleanup

The shared Qt executor SHALL keep each self-test capture separate, run the
reporter even after a failed stage, and return the first non-zero stage status
in deterministic order. For concurrent self-tests, the bare invocation's status
SHALL precede the document invocation's status. A failed `pictura` build SHALL
prevent self-tests and CTest. A failed Qt Test build SHALL prevent CTest. On
interrupt or termination, the executor SHALL terminate and reap its child
processes before exiting.

#### Scenario: self-test failure identity survives aggregation [qe_failure_identity]

- **WHEN** the bare self-test exits 7 and the document self-test exits 9
- **THEN** the reporter receives both separate captures and the executor exits 7

#### Scenario: a failed build stops later stages [qe_build_failure]

- **WHEN** the `pictura` build or the remaining Qt Test build fails
- **THEN** the executor skips the stages that depend on it, still reports available captures and missing reports, and returns the build's exit status

#### Scenario: termination reaps children [qe_signal_cleanup]

- **WHEN** the executor receives SIGINT or SIGTERM while a build or self-test child is running
- **THEN** it terminates and reaps those children, removes temporary captures, and exits with the signal's conventional non-zero status

### Requirement: Qt report inputs stay fresh and complete

Before any stage can produce a Qt result, the shared Qt executor SHALL remove
old `build/qt-test-results/*.xml` files. After CTest, it SHALL pass the current
per-suite JUnit files and the CTest registration JSON to the reporter. A
registered suite with no current report SHALL fail the run and be named, even
when a stale file for that suite existed before the run.

#### Scenario: stale JUnit files are removed [qe_stale_cleanup]

- **WHEN** a previous run left JUnit files for suites that no longer produce reports
- **THEN** those files are removed before execution and do not appear in the new report

#### Scenario: missing reports are named [qe_missing_report]

- **WHEN** a CTest-registered suite exits without writing its JUnit report
- **THEN** the reporter names that suite as having produced no report and the executor exits non-zero

### Requirement: Qt execution records stage timings

The shared Qt executor SHALL print elapsed durations for Configure, the
`pictura` build, the Qt Test build tail, each self-test invocation, CTest, and
reporting. A stage that is skipped SHALL say so rather than report a fabricated
zero duration. The timing output SHALL be sufficient for #1570 to compare the
sequential and overlapped candidates.

#### Scenario: every executed Qt stage is timed [qe_timings]

- **WHEN** the shared Qt executor completes a staged run
- **THEN** its output records all six stage durations, including separate durations for the two self-test invocations

### Requirement: Overlap retention depends on measurement

The build-tail overlap SHALL remain disabled in CI and local adapters until
#1570 measures it on a four-vCPU runner and records a net wall-time improvement
without new flakes or CPU-contention regressions. The executor SHALL retain an
explicit disable switch so a measured regression can be reverted without
changing code.

#### Scenario: four-vCPU measurement decides retention [qe_overlap_measurement]

- **WHEN** #1570 compares sequential and overlapped staged runs on four vCPUs
- **THEN** the adapters enable overlap only if the measured net result is positive, and otherwise leave it disabled
