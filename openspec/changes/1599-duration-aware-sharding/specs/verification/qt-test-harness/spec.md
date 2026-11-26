# Spec Delta

## ADDED Requirements

### Requirement: Shard membership and count come from retained duration evidence

Shard membership and share count SHALL come from the retained timing evidence,
not from a hand-maintained suite list. `scripts/qt_test_timings.py plan` SHALL
write `cmake/qt-test-shards.json` from the timing store: per sharded suite, a
share count, the measured seconds that justify it, and — when per-function
evidence exists — the exact test functions of each share. The count SHALL be the
fewest shares that keep one entry within a quarter of the layer's measured total,
capped by the suite's function count so no share is empty. The algorithm SHALL
sort functions longest-first and fill the emptiest share.

The test build SHALL read that plan: a suite in the plan SHALL register one CTest
entry per share under the names `<suite>_<index>` with
`PICTURA_TEST_SHARD=<index>/<count>`, a measured share SHALL name its functions on
the command line, and a share without named functions SHALL fall back to the
existing declaration-order split in `shardedMain`. A missing plan SHALL warn and
register every suite in one entry rather than fail configuration, and a plan entry
that names no registered suite SHALL fail configuration so a rename cannot
silently drop sharding. The fixed shard scheduler SHALL remain `shardedMain`; no
second scheduler SHALL be added.

`scripts/qt_test_timings.py verify` SHALL reject a plan whose share is empty,
whose function appears in more than one share, or which omits a function the
suite source declares, and SHALL run in the Qt-aware CI job. Regenerating a plan
from a partial store SHALL carry a baseline entry forward instead of silently
un-sharding its suite.

#### Scenario: a measured suite is sharded from its duration [qth_plan_duration]

- **WHEN** the timing store records a suite above a quarter of the layer's measured total
- **THEN** the generated plan gives it more than one share, names the measured seconds as its evidence, and puts each of its functions in exactly one share

#### Scenario: a share is never empty [qth_plan_no_empty_share]

- **WHEN** the duration-derived share count would exceed the suite's function count
- **THEN** the planner caps the count at the function count so no empty share is written, and a plan that declares an empty share is rejected

#### Scenario: an empty or duplicate share is rejected [qth_plan_verify]

- **WHEN** a plan's share is empty, a function repeats across shares, or a declared function is missing
- **THEN** `scripts/qt_test_timings.py verify` exits non-zero and names the suite and the problem

#### Scenario: named functions still run exactly those functions [qth_plan_named_functions]

- **WHEN** a shard entry passes its planned function names on the command line
- **THEN** the suite runs exactly those functions and no others, and the empty-shard detection is unchanged

#### Scenario: a partial store keeps a previous share decision [qth_plan_carry_forward]

- **WHEN** the planner regenerates from a store with no measurement for a suite the existing plan shards
- **THEN** the suite keeps its previous share count and the entry's evidence says it was carried forward

## MODIFIED Requirements

### Requirement: Per-suite state isolation

Each Qt Test suite SHALL isolate persisted state so no suite depends on or
mutates another's preferences or recovery state. Each suite SHALL declare a
`pictura::test::ScopedStateHome` RAII member constructed BEFORE its main window,
so the fixture outlives the window's session save. The fixture's constructor
SHALL point `XDG_STATE_HOME` at a fresh temporary directory, and its destructor
SHALL restore the previous value (including unsetting it only when it was not
set before), rather than blanket-unsetting. Isolation SHALL NOT rely on
`initTestCase()`: the fixture must be alive across window construction and
destruction, which `initTestCase()` cannot guarantee. The frame SHALL read
`XDG_STATE_HOME` and `QApplication` SHALL NOT, so a suite SHALL enter through
`QTEST_MAIN`, or through `pictura::test::shardedMain`, which is `QTEST_MAIN` with
the shard applied: the same application, the same navigation mode, and no global
setup hook. The existing suites with a hand-written main keep it; this
requirement does not add another.

#### Scenario: Suites do not share state

- **WHEN** two suites run in the same `ctest` invocation
- **THEN** each runs in its own process with its own temporary state home, so neither observes state written by the other

#### Scenario: Each run starts clean

- **WHEN** a suite writes preferences and the suite runs again
- **THEN** it starts from a clean temporary state directory

#### Scenario: A pre-existing state home is restored

- **WHEN** `XDG_STATE_HOME` is already set in the environment and the fixture is destroyed
- **THEN** the previous value is restored, not unset

#### Scenario: the sharded main isolates like QTEST_MAIN [qth_sharded_main_isolation]

- **WHEN** a sharded suite runs under any share
- **THEN** it constructs the state-home fixture before its window and enters through `shardedMain`, so it gets the same application and isolation as `QTEST_MAIN`

### Requirement: The Qt Test layer's critical path is not a single suite

The Qt Test layer SHALL be split so that the CTest run's critical path is not
one suite: the suites that determine the layer's wall time SHALL number at
least two, so the layer's duration is set by the number of suites and the
parallel level rather than by one slow suite. A suite that is inherently long
SHALL be split into additional registered entries, and the CTest parallel
level SHALL remain overridable through `CTEST_PARALLEL_LEVEL`. At the default
parallel level, no single registered entry SHALL take more than a quarter of
the layer's total wall time, both as CTest reports them, unless the entry's
test functions share state; such an entry SHALL be named, with its reason, next
to the shard registration. Shard membership and share count SHALL be derived
from the retained timing evidence and reported against that quarter budget.

#### Scenario: the critical path names more than one suite [qth_critical_path]

- **WHEN** the CTest run finishes and prints its critical path
- **THEN** the critical path names at least two suites

#### Scenario: no suite is reported as timed out [qth_suite_timeout]

- **WHEN** the CTest run finishes
- **THEN** no suite is reported as timed out

#### Scenario: the longest entry is bounded [qth_longest_entry]

- **WHEN** the Qt Test layer runs at the default parallel level
- **THEN** no entry's time in CTest's report exceeds a quarter of CTest's total real time, or the entry is named next to the shard registration with the reason it cannot be split

### Requirement: Qt Test suites wait on conditions, not fixed delays

A Qt Test suite SHALL wait for an observable condition — a widget state, a
signal, a file, a timer having fired — through a bounded condition wait that
returns as soon as the condition holds, rather than through a fixed sleep. A
wait whose only purpose is to deliver events already queued SHALL be an event
flush that does not sleep: a zero-length wait or delivery of the posted events.
A fixed delay SHALL remain only where no observable condition exists, such as
proving that something does *not* happen within a window, and each remaining
delay SHALL carry a comment on the same or the preceding line naming why no
condition can replace it. A repository guard run by local verification, the
guards CI workflow, and the Qt-aware CI job SHALL enforce this rule on the Qt
Test sources, so it runs on a C++-only change.

#### Scenario: fixed delays are justified or gone [qth_condition_wait]

- **WHEN** the repository guard scans the Qt Test sources
- **THEN** every fixed delay with a non-zero length carries a comment on its line or the line above, and the guard fails on an uncommented one

#### Scenario: a negative wait keeps its delay [qth_negative_wait]

- **WHEN** a test asserts that an event does not occur within a time window
- **THEN** it keeps a fixed delay of that window, with the comment that names it

### Requirement: Qt Test and CTest harness

The build SHALL call `include(CTest)` gated on `BUILD_TESTING` and SHALL add a
`crates/pictura-app/cpp/tests/` directory with its own `CMakeLists.txt`. That
file SHALL register one CTest entry per suite, running that suite in its own
process, and SHALL register each entry with `add_test` under
`ENVIRONMENT QT_QPA_PLATFORM=offscreen` and a per-suite `TIMEOUT`. A sharded
suite SHALL register one entry per share, named `<suite>_<index>`, and every
entry SHALL keep the per-suite environment and timeout. Each `add_test` SHALL
pass `-o <CMAKE_BINARY_DIR>/qt-test-results/<name>.xml,junitxml` so the process
emits a per-entry JUnit report. The harness SHALL be gated on `BUILD_TESTING`
with no effect when it is off. CTest SHALL own only the Qt Test entries and
SHALL NOT wrap or replace `cargo nextest`; the Rust suite stays with
`scripts/test-report.sh`.

"One executable per `tst_*.cpp`" is a suite-isolation rule, not a file-build
rule: it requires one process and one CTest entry per suite, so a crash names
one suite, each suite owns its JUnit report, and each suite keeps its timeout.
A registry-driver build that runs one suite per CTest entry in one process per
invocation satisfies it, and shard entries only change which functions that
process runs.

#### Scenario: Configure builds the suites

- **WHEN** CMake configures with `BUILD_TESTING` on
- **THEN** the tests subdirectory is added and each suite is registered with a process of its own linked against `pictura_shell` and `Qt6::Test`

#### Scenario: CTest runs each suite offscreen

- **WHEN** `ctest` runs a registered Qt test entry
- **THEN** it runs with `QT_QPA_PLATFORM=offscreen` and its suite's `TIMEOUT`

#### Scenario: CTest emits per-entry JUnit

- **WHEN** `ctest` runs a registered Qt test entry
- **THEN** the process writes a JUnit report to `build/qt-test-results/<entry>.xml`

#### Scenario: one process and one entry per suite [qth_suite_process]

- **WHEN** the layer is registered
- **THEN** every suite runs in its own process and owns exactly one CTest entry per share, with no suite grouped into another suite's process

#### Scenario: Tests are optional

- **WHEN** CMake configures with `BUILD_TESTING` off
- **THEN** no test entry or test registration is emitted

#### Scenario: CTest does not wrap nextest

- **WHEN** the harness is built
- **THEN** no CTest test invokes `cargo nextest`
