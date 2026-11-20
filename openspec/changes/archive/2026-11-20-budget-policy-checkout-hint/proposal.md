# Proposal

## Why

Testing the merged cross-harness guard against stale worktrees exposed two
policy problems.

1. **The refusal can point at an unbudgeted wrapper.** The policy resolved the
   wrapper from the caller's git checkout. A Codex session in a linked worktree
   reads the hook manifest and policy from the **main checkout**, so it refuses
   a bare command but named the worktree's own `scripts/with-build-lock.sh`. A
   worktree that predates the budgeted wrapper therefore sends the agent to an
   unbudgeted command. The policy checkout and its wrapper are versioned
   together; the hint has to come from the policy's own checkout.
2. **The segment split ignores quoting.** The policy split on `;`, `&`, `|`, and
   newline without regard to quotes, so a legitimate diagnostic such as
   `ps -eo ... | awk '$2 ~ /^(cmake|ninja)$/ {print}'` was refused because the
   `|` inside the awk program looked like a pipeline.

## What Changes

- `scripts/heavy_command_policy.py` resolves the wrapper from the directory that
  contains the policy script, not from the caller's cwd. The hint is relative
  when the cwd is that checkout, absolute otherwise.
- The policy splits shell segments only on `;`, `&&`, `||`, `|`, and newline
  **outside** single and double quotes, and treats a command as compound only
  when a real (unquoted) separator is present.
- `--self-check` proves the policy-checkout hint from an unrelated cwd and the
  quoted-operator cases: a quoted pipe or semicolon passes, a real pipe or
  `&&` before a heavy command refuses.
- Document the policy-checkout hint in `AGENTS.md` and `DEVELOPING.md`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `meta/cpu-budget`: the agent-refusal requirement resolves the wrapper from the
  policy's checkout and splits segments quote-aware.

## Impact

- `scripts/heavy_command_policy.py` and its `--self-check`.
- `AGENTS.md`, `DEVELOPING.md`.
- No harness config changes; the adapters keep running the policy.
- Tracks issue #1460; follows PR #1464.
