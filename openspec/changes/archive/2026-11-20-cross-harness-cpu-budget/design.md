# Design

## Context

Three harnesses run shell commands in this repository:

| Harness | Hook surface | Worktree behavior |
|---|---|---|
| opencode 1.19 | `.opencode/plugins/*.ts`, `tool.execute.before` | Project plugin is discovered from the checkout; a worktree needs the commit. |
| Claude Code 2.1 | `.claude/settings.json` `PreToolUse` / `Bash` | Committed project hooks are branch-local in CLI worktree sessions; desktop worktree sessions read project config from the main checkout, and `${CLAUDE_PROJECT_DIR}` follows the session root. |
| Codex 0.166 | `.codex/hooks.json` `PreToolUse` / `Bash` | In a linked worktree Codex reads the project hook manifest from the main checkout. Every non-managed hook is trusted per hash in `/hooks`. |

The deny spellings differ in detail but agree on the one that matters: exit 2
with the reason on stderr. Both harnesses also accept
`hookSpecificOutput.permissionDecision: "deny"`; the shared policy uses exit 2 so
there is one contract.

## Decisions

### One policy, three adapters

`scripts/heavy_command_policy.py` owns the matcher and the message. It accepts a
`--command` argument (opencode, which already has the string) and hook JSON on
stdin (`tool_input.command`, Claude and Codex). The output is exit 0 for allow
and exit 2 + stderr for refuse. Keeping the policy out of JavaScript removes the
duplicate matcher and its test; the opencode plugin becomes a thin spawner.

A missing policy script fails open (`exit 0`) in the Codex and opencode adapters
so an old checkout does not turn every shell command into an error. Claude's
handler sets `onFailure: "block"` because its settings file and the policy ship
in the same commit, so a policy error is a real error.

### Worktree resolution

- Claude: `python3 "${CLAUDE_PROJECT_DIR}/scripts/heavy_command_policy.py"`.
  `${CLAUDE_PROJECT_DIR}` is the session root, so the hook and the policy always
  come from the same checkout. `onFailure: "block"`.
- Codex: the hook runs the policy from the current checkout
  (`git rev-parse --show-toplevel`) and falls back to the main checkout
  (`git rev-parse --path-format=absolute --git-common-dir`) when the file is
  missing. The manifest itself always comes from the main checkout in a linked
  worktree; the fallback keeps a stale worktree covered, while a branch that
  changes the policy sees its own copy. If neither path has the policy the hook
  exits 0.
- opencode: `worktree ?? directory ?? process.cwd()` from the plugin context.

### Refusal path

The policy resolves the repository root with `git rev-parse --show-toplevel`
(falling back to the current directory) and prints the absolute wrapper path, so
the suggested command works from a subdirectory and from a worktree.

### Timeout and failure

Both hooks use a 10 second timeout, well under Codex's 600 second default and
Claude's default, so a hung policy cannot stall every shell call. The policy is
a Python process with no I/O beyond git, so the normal path is tens of
milliseconds.

### Hooks, not PATH shims

A shim directory on `PATH` would cover more harnesses at once, but it needs a
per-user install and silently changes tool resolution. The harness hooks are
committed, reviewable, and version with the repository; the wrapper stays the
only mechanism that applies the budget.

## Risks / Trade-offs

- **Hooks are guards, not sandboxes.** `FOO=1 cargo build`, absolute paths, and
  shell-constructed commands bypass the matcher. Prose and the wrapper remain the
  baseline.
- **Trust friction.** Codex needs `/hooks` approval and re-approval after every
  policy edit because trust is a hash. Claude asks for workspace trust in
  interactive sessions; `claude -p` and SDK sessions treat the folder as trusted,
  so a committed hook runs in an unreviewed clone.
- **Branch-local coverage in Claude.** A worktree on a branch without the commit
  has no hook until it is rebased. Codex is different: the main checkout's
  manifest applies to every worktree.
- **Overhead.** Claude and Codex pay a Python startup on each `Bash` tool call;
  opencode pays it on each bash call. That is acceptable for a guard.
