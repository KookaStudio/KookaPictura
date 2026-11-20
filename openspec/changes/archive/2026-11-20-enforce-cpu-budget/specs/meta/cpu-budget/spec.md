# meta/cpu-budget Specification

## ADDED Requirements

### Requirement: The documented command wrapper applies the CPU budget

`scripts/with-build-lock.sh` SHALL apply the local CPU budget before it sources
`scripts/build-lock.sh` or acquires the lock. When
`PICTURA_CPU_BUDGET_APPLIED` is unset, it SHALL re-exec itself exactly once
through `scripts/cpu-budget.py exec` with its original arguments, so the wrapped
command and its children run in the budget's affinity slice with the budget's
job counts. The wrapper SHALL keep the caller's working directory and propagate
the wrapped command's exit code. When the marker is already present it SHALL NOT
re-exec, and it SHALL require `PICTURA_JOBS` before running the command.

#### Scenario: The wrapper re-execs once into the budget [cb_wrap_budget]

- **WHEN** `bash scripts/with-build-lock.sh -- <command>` starts with
  `PICTURA_CPU_BUDGET_APPLIED` unset
- **THEN** it re-execs once through the budget helper before sourcing the lock,
  and the wrapped command's affinity mask is the budget's slice

### Requirement: An agent refuses bare heavy commands

The committed opencode plugin SHALL refuse, before execution, a shell command
whose segment starts `cargo`, `cmake`, `ctest`, or `ninja` outside a gate or
wrapper, and the refusal message SHALL name
`bash scripts/with-build-lock.sh -- <command>` for a simple command or
`bash scripts/with-build-lock.sh -- bash -c '<command>'` for a compound one. A
command whose text names `scripts/cpu-budget.py`, `scripts/with-build-lock.sh`,
`scripts/verify-fast.sh`, `scripts/verify-full.sh`, or
`scripts/test-report.sh` SHALL pass, as SHALL a command with no heavy segment.
The matcher SHALL be a pure module with a `node --test` suite, run by
`scripts/verify-fast.sh`.

#### Scenario: A bare heavy command is refused [cb_agent_refuse]

- **WHEN** the agent invokes `cargo nextest run --workspace` without a gate or
  wrapper
- **THEN** the plugin refuses the call before it runs and the message names the
  wrapped form
- **AND** a command that names a gate or the wrapper, and a non-heavy command,
  pass

### Requirement: A running sccache server is held to the slice

On a platform with process-affinity control and `/proc`,
`scripts/cpu-budget.py` SHALL narrow a running `sccache --server` process it can
address to the applied affinity slice, best-effort and without restarting it, so
a server started outside the budget cannot compile wider than the slice. It
SHALL fail silently when a process cannot be addressed, and SHALL skip the step
on a platform without `/proc` or process-affinity control.

#### Scenario: A pre-existing sccache server is narrowed [cb_sccache_server]

- **WHEN** a wide `sccache --server` is running and the budget helper applies a
  narrower slice
- **THEN** the server's affinity mask is the slice and the server is not
  restarted

## MODIFIED Requirements

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
marked process-environment only.

#### Scenario: The documentation names the budget and the wrapper [cb_docs]

- **WHEN** a reader opens the commands or worktree guidance in `AGENTS.md`,
  `DEVELOPING.md`, `README.md`, or `.serena/memories/suggested_commands.md`
- **THEN** every heavy command form goes through the wrapper, and `AGENTS.md`
  and `DEVELOPING.md` name the default budget, the percent override and its
  valid range, the shared slice across worktrees, and the wrapper's
  budget-and-lock behavior
