## ADDED Requirements

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
