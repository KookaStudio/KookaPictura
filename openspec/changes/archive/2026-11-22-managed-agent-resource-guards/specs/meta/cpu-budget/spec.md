## MODIFIED Requirements

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
