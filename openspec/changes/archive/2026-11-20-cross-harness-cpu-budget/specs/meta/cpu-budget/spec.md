# meta/cpu-budget Specification

## ADDED Requirements

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
The refusal SHALL name
`bash <repo-root>/scripts/with-build-lock.sh -- <command>` for a simple command
or `... -- bash -c '<command>'` for a compound one, where `<repo-root>` is
resolved from the current checkout so the hint works from a subdirectory or a
worktree. The opencode plugin SHALL delegate to this policy instead of carrying
its own matcher. A `--self-check` SHALL prove the allow and refuse cases, both
input forms, and the git-root resolution, and `scripts/verify-fast.sh` SHALL run
it.

#### Scenario: A bare heavy command is refused [cb_agent_refuse]

- **WHEN** a harness invokes `cargo nextest run --workspace` without a gate or
  wrapper
- **THEN** the shared policy writes the refusal to stderr and exits 2, and the
  adapter blocks the call before it runs
- **AND** a command that names a gate or the wrapper, `cargo fmt`, `--command`
  and stdin JSON inputs, and a non-heavy command pass

#### Scenario: The refusal names a checkout-relative wrapper [cb_policy_git_root]

- **WHEN** the policy refuses a command from a subdirectory or a linked worktree
- **THEN** the refusal names the wrapper under the checkout root rather than a
  path relative to the current directory

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
the shared agent policy, its Claude Code and Codex hooks, the trust steps, and
the worktree behavior.

#### Scenario: The documentation names the budget and the wrapper [cb_docs]

- **WHEN** a reader opens the commands or worktree guidance in `AGENTS.md`,
  `DEVELOPING.md`, `README.md`, or `.serena/memories/suggested_commands.md`
- **THEN** every heavy command form goes through the wrapper, and `AGENTS.md`
  and `DEVELOPING.md` name the default budget, the percent override and its
  valid range, the shared slice across worktrees, the wrapper's budget-and-lock
  behavior, and the agent guard's trust and worktree behavior
