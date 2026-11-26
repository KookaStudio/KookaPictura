# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
