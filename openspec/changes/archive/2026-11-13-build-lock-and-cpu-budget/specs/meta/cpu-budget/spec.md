## Purpose

Defines the CPU budget that local verification gates and the commands they
spawn run under, so concurrent worktrees share one machine-wide slice instead of
each run taking every CPU.

## ADDED Requirements

### Requirement: Local verification CPU budget

The local verification gates SHALL run under a CPU budget of
`max(1, floor(allowed_cpus * percent / 100))`, where `allowed_cpus` is the
number of CPUs available to the process: the count of its affinity mask when
the platform provides one, otherwise the process's usable CPU count
(`os.process_cpu_count()` when available, else `os.cpu_count()`), and `percent`
is 80 by default, overridable through `PICTURA_CPU_BUDGET_PERCENT` in the
process environment. The percent SHALL be a decimal integer from 1 through 100; any
other value SHALL be rejected with a message naming the invalid value and a
non-zero exit. A percent of 100 SHALL mean every allowed CPU, and the budget
SHALL never be less than one.

#### Scenario: The default budget is 80 percent [cb_default]

- **WHEN** the invoking process may run on 16 CPUs and
  `PICTURA_CPU_BUDGET_PERCENT` is unset
- **THEN** the budget is 12

#### Scenario: The budget rounds down and floors at one [cb_rounding]

- **WHEN** the invoking process may run on 3 CPUs at the default percent, or on
  1 CPU
- **THEN** the budget is 2 and 1 respectively

#### Scenario: An invalid percent is rejected [cb_percent_invalid]

- **WHEN** `PICTURA_CPU_BUDGET_PERCENT` is `0`, `101`, or a non-numeric value
- **THEN** the gate prints a message naming the value and exits non-zero before
  running any build or test step

#### Scenario: One hundred percent uses every allowed CPU [cb_percent_full]

- **WHEN** `PICTURA_CPU_BUDGET_PERCENT=100` and the invoking process may run on
  16 CPUs
- **THEN** the budget is 16

### Requirement: One re-exec applies the budget

`scripts/verify-fast.sh`, `scripts/verify-full.sh`, and `scripts/test-report.sh`
SHALL re-exec themselves once through the budget helper before any build-lock
acquisition, so the gate, the gates it invokes, and the commands they spawn run
under one budget. The helper SHALL set `PICTURA_CPU_BUDGET_APPLIED=1` in the
re-executed environment. When that marker is already present, the helper SHALL
NOT change affinity and SHALL NOT override a job variable already present, but
SHALL still fill any missing job variables and exec the given command. The
helper SHALL export `PICTURA_JOBS` equal to the budget. It SHALL export
`CARGO_BUILD_JOBS`, `CMAKE_BUILD_PARALLEL_LEVEL`, `CTEST_PARALLEL_LEVEL`, and
`NEXTEST_TEST_THREADS` equal to the budget, except that any of those four
already present in the process environment SHALL keep its value.
`PICTURA_JOBS` is the helper's output and SHALL NOT be treated as an input
override.

#### Scenario: A gate re-execs once [cb_reexec_once]

- **WHEN** `scripts/verify-fast.sh` starts with `PICTURA_CPU_BUDGET_APPLIED`
  unset
- **THEN** it re-execs once through the helper, the re-executed gate sees the
  marker set, and its children run in the budget's affinity slice

#### Scenario: Nested gates reuse the slice [cb_nested_reuse]

- **WHEN** `scripts/verify-full.sh` invokes `scripts/verify-fast.sh`, which
  invokes `scripts/test-report.sh`
- **THEN** all three see the marker and the same affinity slice, and no gate
  narrows the slice again

#### Scenario: The job variables carry the budget [cb_env_exports]

- **WHEN** a command runs through the helper with no job variable set in the
  process environment
- **THEN** it sees `PICTURA_JOBS`, `CARGO_BUILD_JOBS`,
  `CMAKE_BUILD_PARALLEL_LEVEL`, `CTEST_PARALLEL_LEVEL`, and
  `NEXTEST_TEST_THREADS` all equal to the budget

#### Scenario: An explicit job override wins [cb_override]

- **WHEN** `CARGO_BUILD_JOBS=3` is set in the process environment and the budget
  is 12
- **THEN** the child sees `CARGO_BUILD_JOBS=3`, the other four variables equal
  12, and the affinity slice still holds 12 CPUs

#### Scenario: A preset PICTURA_JOBS is replaced [cb_pictura_jobs_output]

- **WHEN** `PICTURA_JOBS=99` is set in the process environment and the budget is
  12
- **THEN** the child sees `PICTURA_JOBS=12`

#### Scenario: The marker skips only the affinity restriction [cb_marker_skip]

- **WHEN** `PICTURA_CPU_BUDGET_APPLIED=1` is already set and no job variables
  are present
- **THEN** the helper leaves affinity unchanged, fills the five job variables
  to the budget, and runs the given command

### Requirement: The budget is a shared affinity slice

On a platform that provides process-affinity control, the helper SHALL restrict
the re-executed process to the lowest-numbered `budget` CPUs of the current
allowed set, so every child inherits the same mask. On a platform without that
control, it SHALL export the job counts without restricting affinity. If
applying the mask fails, the helper SHALL print a warning, export the job
counts, and continue without a slice. Two
top-level gates started from the same allowed set SHALL select the same slice,
so concurrent worktrees contend inside one slice instead of each taking a
private share of the machine. An explicit job-count override SHALL NOT widen the
slice.

#### Scenario: The slice is the lowest-numbered budget CPUs [cb_slice]

- **WHEN** the allowed set is CPUs 0 through 15 and the budget is 12
- **THEN** the re-executed process and its children may run only on CPUs 0
  through 11

#### Scenario: A restricted allowed set is sliced from itself [cb_slice_subset]

- **WHEN** the allowed set is CPUs 8 through 15 and the budget is 6
- **THEN** the re-executed process and its children may run only on CPUs 8
  through 13

#### Scenario: Concurrent gates choose the same slice [cb_shared_slice]

- **WHEN** two top-level gates start from the same allowed set at the same
  percent
- **THEN** both affinity masks are identical, and the two runs contend inside
  that slice rather than each receiving a private share

#### Scenario: A platform without affinity control still caps job counts [cb_fallback]

- **WHEN** the helper runs on a platform without process-affinity control, or
  applying the mask fails
- **THEN** it warns when the call failed, exports the five job variables, and
  leaves the process's affinity unchanged

### Requirement: The fast gate bounds its check pool

`scripts/verify-fast.sh` SHALL run its independent script checks concurrently
with no more than `PICTURA_JOBS` checks running at once, and SHALL still run
every check before failing and print the captured output of each check that
exited non-zero.

#### Scenario: The check pool stays within the budget [cb_fast_pool]

- **WHEN** `scripts/verify-fast.sh` runs with a budget of 3 and thirteen
  independent checks
- **THEN** no more than three checks run at once, all thirteen run, and a
  failing check still fails the gate after the pool drains

### Requirement: A runnable CPU-budget self-check

The repository SHALL provide `scripts/cpu-budget.py --self-check` that proves
the budget arithmetic, including the rounding floor and the one-CPU case, the
percent validation, the slice selection, the job exports, the explicit
overrides, the wrap-once marker, and the inherited-marker path, and SHALL exit
non-zero on any failed assertion. The mask assertion SHALL be skipped on a
platform without process-affinity control. `scripts/verify-fast.sh` SHALL run
the self-check in its check pool.

#### Scenario: The self-check passes [cb_self_check]

- **WHEN** `python3 scripts/cpu-budget.py --self-check` runs against the
  committed helper
- **THEN** every assertion holds and the script exits 0

#### Scenario: A broken budget fails the self-check [cb_self_check_fail]

- **WHEN** the budget formula, the percent validation, or the wrap-once marker
  is changed so it no longer matches the contract
- **THEN** the self-check reports the failing assertion and exits non-zero

### Requirement: The budget is documented

`AGENTS.md` and `DEVELOPING.md` SHALL state the default 80 percent budget, the
`PICTURA_CPU_BUDGET_PERCENT` range, the always-on gate re-exec and shared
slice, and the wrapper for direct heavy commands. `.env.example` SHALL list the
percent knob as a commented entry under its external-build-knobs section,
marked process-environment only.

#### Scenario: The documentation names the budget and the wrapper [cb_docs]

- **WHEN** a reader opens the commands or worktree guidance in `AGENTS.md` and
  `DEVELOPING.md`
- **THEN** both name the default budget, the percent override and its valid
  range, the shared slice across worktrees, and the wrapper for direct
  `cmake`, `cargo`, and `ctest` commands
