# meta/cpu-budget Specification

## Purpose
Defines the CPU budget that local verification gates and the commands they
spawn run under, so concurrent worktrees share one machine-wide slice instead of
each run taking every CPU.

## Requirements

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
slice, and the wrapper for direct heavy commands, including that the wrapper
applies the same budget before the opt-in lock. The command guidance in
`AGENTS.md`, `DEVELOPING.md`, `README.md`, and
`.serena/memories/suggested_commands.md` SHALL NOT present a bare heavy
`cmake`, `cargo` (other than `cargo fmt`), or `ctest` command; each heavy form
SHALL be shown through `scripts/with-build-lock.sh`. `.env.example` SHALL list
the percent knob as a commented entry under its external-build-knobs section,
marked process-environment only. `AGENTS.md` and `DEVELOPING.md` SHALL also name
the shared agent policy, its Claude Code and Codex hooks, the trust steps, the
worktree behavior, and that the refusal names the wrapper from the policy's
checkout.

#### Scenario: The documentation names the budget and the wrapper [cb_docs]

- **WHEN** a reader opens the commands or worktree guidance in `AGENTS.md`,
  `DEVELOPING.md`, `README.md`, or `.serena/memories/suggested_commands.md`
- **THEN** every heavy command form goes through the wrapper, and `AGENTS.md`
  and `DEVELOPING.md` name the default budget, the percent override and its
  valid range, the shared slice across worktrees, the wrapper's budget-and-lock
  behavior, the agent guard's trust and worktree behavior, and the
  policy-checkout wrapper hint

### Requirement: The documented command wrapper applies the CPU budget

Outside a managed slice, `scripts/with-build-lock.sh` SHALL apply the local
CPU budget before it acquires the portable build lock. When
`PICTURA_CPU_BUDGET_APPLIED` is unset, it SHALL re-exec itself exactly once
through `scripts/cpu-budget.py exec` with its original arguments, so the
wrapped command and its children inherit the affinity slice and job counts.
The wrapper SHALL preserve the working directory and command exit status.
When the marker is already present it SHALL use the inherited budget and
require `PICTURA_JOBS`. Inside the managed slice, the wrapper SHALL use the
verified managed queue.

#### Scenario: The wrapper re-execs once into the budget [cb_wrap_budget]

- **WHEN** `bash scripts/with-build-lock.sh -- <command>` starts outside the
  managed slice with `PICTURA_CPU_BUDGET_APPLIED` unset
- **THEN** the budget helper runs before portable lock acquisition and the
  wrapped command inherits the budget

### Requirement: An agent refuses bare heavy commands

The repository SHALL provide one shared policy,
`scripts/heavy_command_policy.py`, that refuses bare shell invocations of
`cargo`, `cmake`, `ctest`, or `ninja` outside an approved runner invocation.
The policy SHALL accept either a `--command <text>` argument or a hook JSON
object on standard input with the command at `tool_input.command`. It SHALL
write the refusal reason to standard error and exit 2, and SHALL exit 0 when
the command is allowed, absent, or the input names an unrelated tool.
The legacy entry points and `cargo fmt` SHALL remain usable. The occurrence of
an entry-point filename in one segment SHALL grant no exemption to a sibling
segment. Shell operators inside quoted text SHALL be treated as argument data
unless passed as a shell command string.
For ordinary legacy calls, the refusal SHALL give a runnable
`scripts/with-build-lock.sh` invocation from the policy's own checkout,
relative when the current directory is that checkout and absolute otherwise.
The OpenCode plugin SHALL delegate to this policy. The policy self-check
SHALL cover both input forms, quoted operators, and the policy-checkout hint
from an unrelated directory. The fast gate SHALL run that self-check.

#### Scenario: A bare heavy command is refused [cb_agent_refuse]

- **WHEN** a harness invokes `cargo nextest run --workspace` without a gate or
  wrapper, or appends a bare heavy command to a wrapped command
- **THEN** the shared policy writes the refusal to stderr and exits 2
- **AND** a properly wrapped command, `cargo fmt`, and a non-heavy command pass

#### Scenario: The refusal names a checkout-relative wrapper [cb_policy_git_root]

- **WHEN** the legacy policy refuses a command from an unrelated current
  directory, a subdirectory, or a linked worktree
- **THEN** the refusal names the wrapper from the policy's own checkout

#### Scenario: Quoted operators are data [cb_policy_quotes]

- **WHEN** an ordinary quoted argument includes `|` or `;` and the command
  contains no unwrapped heavy segment
- **THEN** the policy allows the command
- **AND** it refuses a bare heavy sibling after an unquoted shell separator

### Requirement: A running sccache server is held to the slice

For legacy affinity runs on a platform with process-affinity control and
`/proc`, `scripts/cpu-budget.py` SHALL try to narrow the leader of an
addressable `sccache --server` outside the agent slice. This is best-effort and
SHALL silently skip inaccessible processes. For enrolled runs the launcher
SHALL select a dedicated per-user local sccache socket and disk-cache location
shared across worktrees. When sccache is installed, the host launcher SHALL
start or reuse the dedicated server and verify its membership before the
harness starts. Compiler children and local compilation fallbacks SHALL
inherit the quota. An enrolled build SHALL skip legacy affinity changes to
sccache servers.

#### Scenario: A pre-existing sccache server is narrowed [cb_sccache_server]

- **WHEN** an addressable `sccache --server` runs outside the agent slice and
  the legacy budget helper applies a narrower mask
- **THEN** the helper narrows the server's leader and does not restart it

#### Scenario: The enrolled launcher isolates the cache endpoint [acq_cache]

- **WHEN** an enrolled launch inherits a default sccache configuration or a
  private Qt runtime directory
- **THEN** it selects the same dedicated socket and cache directory as another
  enrolled worktree for that user, independent of the ambient cache endpoint

#### Scenario: The cache server is prepared before entering the sandbox [acq_cache_start]

- **WHEN** sccache is installed and the host launcher prepares an enrolled
  command
- **THEN** it starts a dedicated server when necessary and checks that its
  process belongs to the same quota slice before running the command

#### Scenario: Enrolled verification leaves legacy cache affinity alone [acq_cache_affinity]

- **WHEN** the verification budget helper runs inside the agent slice
- **THEN** it applies its job defaults and skips the legacy sccache
  affinity changes

#### Scenario: Cache misses compile inside the quota [acq_cache_worker]

- **WHEN** an enrolled cache client uses the dedicated server for a
  cache miss while a default server can also exist
- **THEN** the dedicated server and compiler are descendants of the quota
  slice and the default server remains outside

### Requirement: Claude Code and Codex run the shared policy

The repository SHALL commit `.claude/settings.json` and `.codex/hooks.json` with
a `PreToolUse` hook that matches `Bash` and runs
`scripts/heavy_command_policy.py` with a timeout of at most 30 seconds. The
Claude Code hook SHALL resolve the policy from `${CLAUDE_PROJECT_DIR}` and set
`onFailure` to `block`. The Codex hook SHALL resolve the policy from the
current checkout using `git rev-parse --show-toplevel`, SHALL fall back to the
main checkout of a linked worktree using
`git rev-parse --path-format=absolute --git-common-dir`, and SHALL exit 0 when
neither has the policy file so a stale checkout fails open instead of blocking
every shell command. `scripts/verify-fast.sh` SHALL run a check that both files
parse, match `Bash`, reference the policy, set a timeout, and use the correct
worktree-relative resolution. `AGENTS.md` and `DEVELOPING.md` SHALL name the
trust steps (`/hooks` for Codex, the workspace trust dialog for Claude Code) and
the worktree behavior.

#### Scenario: Both harnesses are wired [cb_harness_hooks]

- **WHEN** the committed hook files and the guard check are read
- **THEN** each harness has a `PreToolUse` / `Bash` hook that runs
  `scripts/heavy_command_policy.py` with a timeout, and the check fails when a
  hook, its matcher, or its policy reference is removed

#### Scenario: The harness hooks follow worktrees [cb_harness_worktree]

- **WHEN** a linked worktree runs Claude Code or Codex
- **THEN** the Claude hook runs the policy from `${CLAUDE_PROJECT_DIR}`, the
  Codex hook runs the checkout's policy and falls back to the main checkout
  through `--git-common-dir`, and a missing policy fails open

### Requirement: Enrolled coding agents share one user CPU quota

The repository SHALL provide an explicit Linux systemd user-slice setup for
Codex, OpenCode, and Claude Code. Setup SHALL default to 80% of the online
logical CPUs and SHALL accept a percentage from 1 to 100. There SHALL be one
quota on the parent slice across all enrolled sessions and worktrees. No user
unit SHALL be installed or enabled by routine repository use. Processes
outside the slice SHALL keep their existing CPU access.

#### Scenario: Setup generates one configurable slice unit [acq_unit]

- **WHEN** a 16-logical-CPU host requests the unit definition at the default
  percentage or at 50%
- **THEN** it specifies `CPUQuota=1280%` or `CPUQuota=800%` respectively, writes
  the definition to standard output, and leaves installation to the user

#### Scenario: Two sessions consume the same CPU quota [acq_aggregate]

- **WHEN** enrolled commands from separate worktrees run concurrent shell,
  JavaScript, or Python child processes under a low test quota
- **THEN** the children share the quota ancestor and the parent's CPU
  accounting records their combined usage and throttling

### Requirement: Launch verifies the quota before running work

The launcher SHALL use a systemd user scope and SHALL keep the working
directory, environment, terminal, arguments, and command status usable by the
harness. It SHALL check actual cgroup membership and a finite parent quota
before executing the target, including when already in the enrolled slice.
It SHALL give a nonzero result when it cannot establish that enrollment.
A status command SHALL expose the effective slice path and configured quota
from a process running inside it.

#### Scenario: The launcher uses the shared slice and preserves arguments [acq_launch]

- **WHEN** an opted-in launcher starts a supported CLI or another command
- **THEN** the invocation targets the one named user slice and delivers the
  command's arguments without shell interpretation or variable expansion

#### Scenario: Missing or ineffective enrollment is refused [acq_refuse]

- **WHEN** the slice is missing from the command's ancestry, its quota is
  missing or unlimited, or systemd cannot put the command in the slice
- **THEN** the launcher reports failure before running the target

#### Scenario: Each harness executes inside the quota [acq_harnesses]

- **WHEN** OpenCode, Claude Code, and an enrolled Codex CLI or app server execute
  local shell commands from their normal entry points
- **THEN** the actual shell reports the same quota ancestor with the status
  command, including the regular and approved local Codex command paths
