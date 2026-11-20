# Design

## Context

The shared policy is invoked by three adapters:

- opencode: the plugin from the session's checkout passes `--command`.
- Claude Code: `python3 "${CLAUDE_PROJECT_DIR}/scripts/heavy_command_policy.py"`.
- Codex: `--show-toplevel` first, then the main checkout through
  `--git-common-dir`; in a linked worktree the manifest itself always comes from
  the main checkout.

The wrapper and the policy are coupled by commit: a policy from checkout A should
name the wrapper from checkout A, because that is the wrapper version the policy
was tested with. The caller's cwd is not that checkout when Codex reads main's
policy inside a stale worktree.

## Decisions

### Resolve the wrapper from the policy's checkout

`policy_root()` is the parent of the directory containing the policy script
(`os.path.realpath(__file__)`). `wrapper_path()` prints
`scripts/with-build-lock.sh` when the cwd is that root and the absolute path
otherwise. This keeps the common hint short and makes the stale-worktree Codex
case name the main checkout's budgeted wrapper, which still runs the command in
the caller's worktree.

The git lookup is removed: the policy no longer needs to find the caller's
repository root, and the hint no longer changes with `cd`.

### Split segments quote-aware

A small scanner tracks single quotes, double quotes, and backslash escapes and
splits on `;`, `&&`, `||`, `|`, and newline only outside quotes. A quoted
`| sudo` or `; cargo` is data, not a second command. A real separator before a
heavy command still refuses.

- `ponytail:` this is not a shell parser. Command substitutions, backticks, and
  `bash -c '<command>'` are not descended into; env-prefixed and absolute-path
  invocations still bypass the matcher. Prose and the wrapper remain the
  baseline.

### Keep the compound suggestion

The wrapped suggestion uses `bash -c '<command>'` when the quote-aware split
found a real separator (or the command contains `bash -c`), so a pipeline with a
quoted `|` is suggested in its simple form instead of being wrapped in an
unnecessary subshell.

## Risks / Trade-offs

- **A symlinked policy** resolves its checkout through `realpath`; the hint
  follows the real file, which is the version that is actually running.
- **Nested shell syntax** (`$(…)`, backticks) can still hide a heavy command;
  this is the same ceiling as before and is named in the `ponytail:` comment.
- **Stale worktrees** still need a rebase for Claude and opencode to receive the
  guard at all; the fix only guarantees that the Codex path comments with a
  wrapper that budgets.
