# meta/cpu-budget Specification

## MODIFIED Requirements

### Requirement: An agent refuses bare heavy commands

The repository SHALL provide one shared policy,
`scripts/heavy_command_policy.py`, that refuses, before execution, a shell
command whose segment starts `cargo`, `cmake`, `ctest`, or `ninja` outside a gate
or wrapper. The policy SHALL accept either a `--command <text>` argument or a
hook JSON object on standard input with the command at `tool_input.command`,
SHALL write the refusal reason to standard error and exit 2, and SHALL exit 0
when the command is allowed, absent, or the input names a non-`Bash` tool. A
command whose text names `scripts/cpu-budget.py`,
`scripts/with-build-lock.sh`, `scripts/verify-fast.sh`,
`scripts/verify-full.sh`, or `scripts/test-report.sh` SHALL pass, as SHALL a
command with no heavy segment and a `cargo fmt` segment, which runs no compiler.
Segments SHALL be split on `;`, `&&`, `||`, `|`, and newline only outside single
and double quotes, so a quoted operator is data rather than a second command.
The refusal SHALL name
`bash <wrapper> -- <command>` for a simple command or
`bash <wrapper> -- bash -c '<command>'` for a compound one, where `<wrapper>` is
the `scripts/with-build-lock.sh` of the checkout that contains the policy
script: relative when the cwd is that checkout, absolute otherwise. The opencode
plugin SHALL delegate to this policy instead of carrying its own matcher. A
`--self-check` SHALL prove the allow and refuse cases, both input forms, the
quoted-operator cases, and the policy-checkout wrapper hint from an unrelated
cwd, and `scripts/verify-fast.sh` SHALL run it.

#### Scenario: A bare heavy command is refused [cb_agent_refuse]

- **WHEN** a harness invokes `cargo nextest run --workspace` without a gate or
  wrapper
- **THEN** the shared policy writes the refusal to stderr and exits 2, and the
  adapter blocks the call before it runs
- **AND** a command that names a gate or the wrapper, `cargo fmt`, `--command`
  and stdin JSON inputs, and a non-heavy command pass

#### Scenario: The refusal names a checkout-relative wrapper [cb_policy_git_root]

- **WHEN** the policy refuses a command from an unrelated cwd, a subdirectory,
  or a linked worktree
- **THEN** the refusal names the wrapper from the policy's own checkout rather
  than a wrapper under the caller's cwd

#### Scenario: Quoted operators are data [cb_policy_quotes]

- **WHEN** the command contains a `|` or `;` inside a quoted argument and no
  unquoted separator before a heavy command
- **THEN** the policy allows the command
- **AND** a real unquoted `|`, `&&`, `||`, or `;` before a heavy command still
  refuses

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
