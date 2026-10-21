# Spec Delta

## ADDED Requirements

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
