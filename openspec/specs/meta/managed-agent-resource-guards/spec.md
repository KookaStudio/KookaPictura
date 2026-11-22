# meta/managed-agent-resource-guards Specification

## Purpose
Keep local agent workloads within a shared Linux resource budget and
serialize heavyweight verification across worktrees. Define the host setup
and OpenCode admission behavior that support these controls.

## Requirements

### Requirement: An enrolled agent has finite resource limits

The Linux launcher SHALL verify finite aggregate CPU, memory and swap limits in the shared slice before executing an agent. It SHALL keep the dedicated compiler cache in that slice and report effective configured limits through its status command.

#### Scenario: The launcher supplies and verifies resource boundaries [mag_limits]

- **WHEN** a user generates a slice and then launches an agent or checks its status
- **THEN** the unit defines CPU quota, memory pressure and hard ceilings, and a swap ceiling, and the launcher refuses incomplete or invalid limits before starting the agent

#### Scenario: Local executor and cache use the same slice [mag_host]

- **WHEN** the host owner enrolls OpenCode executors in two worktrees, including any dedicated compiler cache daemon
- **THEN** the shell workers and cache daemon are in the same slice with finite resource limits

### Requirement: Managed verification shares one host queue

A managed heavy verification entry point SHALL use the shared host queue regardless of caller-provided lock markers or overrides. The runner SHALL admit one job across worktrees, reuse verified job membership for nested calls, and retain the slot until the job unit stops. A wait timeout SHALL identify the holder and fail. Cancellation SHALL stop the job unit before handing over the slot.

#### Scenario: Managed invocations queue and nested invocations reuse the slot [mag_queue]

- **WHEN** a second worktree attempts a managed job with lock disabling or lock ownership environment overrides
- **THEN** it waits or times out with holder details, and a nested invocation inside the current job can proceed

#### Scenario: Cancellation clears the current job before handover [mag_cancel]

- **WHEN** the current job is cancelled or its runner disappears while its job unit is active
- **THEN** an active previous job unit prevents the next job from starting and cancellation requests stop the whole unit

#### Scenario: Real user units serialize and clean up across worktrees [mag_host_queue]

- **WHEN** two enrolled worktrees submit harmless commands and the active command is cancelled
- **THEN** the waiting command starts only after systemd stops all processes of the active job

### Requirement: OpenCode checks the canonical policy

The managed OpenCode adapter SHALL use a host-selected policy path and refuse shell execution when that policy is unavailable or returns an unexpected result. The policy SHALL direct heavy invocations to the approved runner and SHALL evaluate sibling commands separately from that runner invocation.

#### Scenario: Policy failure stops managed OpenCode execution [mag_policy_failure]

- **WHEN** the managed policy is absent or its execution fails
- **THEN** the OpenCode hook refuses the command and names the policy failure

#### Scenario: An approved invocation grants no exemption to other segments [mag_policy_segment]

- **WHEN** a shell command names the approved runner and also contains a separate heavyweight invocation
- **THEN** the guard rejects the separate command, including common environment, path, or shell prefixes

#### Scenario: A stale worktree receives the canonical guard [mag_host_opencode]

- **WHEN** an enrolled OpenCode server with the global adapter services a worktree on an old revision
- **THEN** shell admission still uses the canonical policy

### Requirement: OpenCode can enroll its Pictura project shell

On Linux, the repository OpenCode adapter SHALL support project shell enrollment after an explicit Pictura user opt-in. It SHALL select the shell entry point from the canonical checkout shared by linked worktrees, preserve the original command presented to OpenCode's permission flow, and leave unrelated projects, opted-out sessions, and explicit full-executor launches on their existing shell.

#### Scenario: Plain OpenCode configures a managed shell in a Pictura worktree [mag_project_shell]

- **WHEN** the user opts in and starts plain OpenCode in a linked Pictura worktree
- **THEN** the plugin selects the canonical managed shell without rewriting tool arguments

#### Scenario: Scope and opt-in preserve existing shells [mag_project_isolation]

- **WHEN** an unrelated project loads the plugin, a Pictura user has not opted in, or an executor already uses the full launcher
- **THEN** the plugin leaves its existing shell selection intact

#### Scenario: Incomplete enrollment stops shell execution [mag_project_fail_closed]

- **WHEN** project enrollment is enabled but the managed shell is missing, overridden or unable to establish the configured slice
- **THEN** OpenCode reports the setup failure before running the shell command

### Requirement: Enrolled project commands use the existing resource controls

The managed OpenCode shell entry point SHALL use the existing systemd launcher for all invocations and SHALL route recognized heavyweight commands through the managed verification queue. It SHALL keep the caller's directory and Bash arguments.

#### Scenario: Heavy project commands enter the shared queue [mag_project_queue]

- **WHEN** OpenCode launches an ordinary shell command, a recognized heavyweight command, or an invocation of the approved runner
- **THEN** each command uses the shared slice and heavyweight work enters the queue once

#### Scenario: Plain OpenCode from two worktrees shares the host budget [mag_project_host]

- **WHEN** plain OpenCode starts from two opted-in Pictura worktrees and runs harmless shell and queue probes
- **THEN** their shell workers report the same finite slice limits and the second queued probe waits for the first
