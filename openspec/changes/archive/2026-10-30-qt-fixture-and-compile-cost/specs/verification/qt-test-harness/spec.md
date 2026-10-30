## ADDED Requirements

### Requirement: Qt Test suites wait on conditions, not fixed delays

A Qt Test suite SHALL wait for an observable condition — a widget state, a
signal, a file, a timer having fired — through a bounded condition wait that
returns as soon as the condition holds, rather than through a fixed sleep. A
wait whose only purpose is to deliver events already queued SHALL be an event
flush that does not sleep: a zero-length wait or delivery of the posted events.
A fixed delay SHALL remain only where no observable condition exists, such as
proving that something does *not* happen within a window, and each remaining
delay SHALL carry a comment on the same or the preceding line naming why no
condition can replace it. A repository check SHALL enforce this rule on the
Qt Test sources.

#### Scenario: fixed delays are justified or gone [qth_condition_wait]

- **WHEN** the repository check scans the Qt Test sources
- **THEN** every fixed delay with a non-zero length carries a comment on its line or the line above, and the check fails on an uncommented one

#### Scenario: a negative wait keeps its delay [qth_negative_wait]

- **WHEN** a test asserts that an event does not occur within a time window
- **THEN** it keeps a fixed delay of that window, with the comment that names it

### Requirement: A suite shares one window only when a reset makes its functions independent

A Qt Test suite MAY build one main window for all its test functions instead
of one per function only when its per-function reset restores every piece of
window and persisted state that a test function reads, so each function passes
when run alone and in any shard. A suite that shares a window and is registered
in several CTest entries SHALL keep passing in every entry. A suite whose reset
cannot be shown complete SHALL keep building a window per test function. A
suite executable SHALL run exactly the test functions named on its command
line when any are named, whether or not it is registered in shards.

#### Scenario: naming functions runs only those functions [qth_named_functions]

- **WHEN** a sharded suite executable is started with one test function named on its command line
- **THEN** it runs that function and no other

#### Scenario: a shared-window suite passes function by function [qth_shared_window_isolated]

- **WHEN** each test function of a shared-window suite is run alone by name
- **THEN** every function passes

#### Scenario: a shared-window suite passes in every shard [qth_shared_window_shards]

- **WHEN** a sharded shared-window suite runs under each value of its shard variable
- **THEN** every entry passes and together they run every test function once

## MODIFIED Requirements

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
to the shard registration.

#### Scenario: the critical path names more than one suite [qth_critical_path]

- **WHEN** the CTest run finishes and prints its critical path
- **THEN** the critical path names at least two suites

#### Scenario: no suite is reported as timed out [qth_suite_timeout]

- **WHEN** the CTest run finishes
- **THEN** no suite is reported as timed out

#### Scenario: the longest entry is bounded [qth_longest_entry]

- **WHEN** the Qt Test layer runs at the default parallel level
- **THEN** no entry's time in CTest's report exceeds a quarter of CTest's total real time, or the entry is named next to the shard registration with the reason it cannot be split
