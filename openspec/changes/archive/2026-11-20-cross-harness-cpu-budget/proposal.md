# Proposal

## Why

#1457 (PR #1458) puts opencode's heavy commands under the 80% CPU budget, but
Claude Code and Codex sessions still run bare `cargo`, `cmake`, `ctest`, and
`ninja` outside any gate. Their command guidance is `AGENTS.md` prose only:
`CLAUDE.md` imports it, and Codex reads it, but neither harness enforces it. A
harness session and a budgeted gate can therefore still add up past the
machine-wide cap.

Both harnesses expose a documented pre-tool hook that can deny a shell command
before it runs, so the gap can be closed with committed configuration instead of
an installer:

- Claude Code: `.claude/settings.json` `PreToolUse` / `Bash`, committed project
  settings, deny with exit 2 or `permissionDecision: "deny"`.
- Codex: `.codex/hooks.json` `PreToolUse` / `Bash`, deny with exit 2 or
  `permissionDecision: "deny"`. In a linked worktree Codex reads the project
  hook manifest from the **main checkout**.

The worktree and trust behavior differs between the two, so the design has to be
explicit about it.

## What Changes

- Replace the opencode-only JavaScript matcher with one shared policy,
  `scripts/heavy_command_policy.py`. It accepts `--command <text>` or the hook
  JSON on stdin (`tool_input.command`), writes the refusal to stderr, exits 2,
  and exits 0 otherwise.
- Rewire `.opencode/plugins/cpu-budget.ts` to delegate every bash command to the
  shared policy; delete `.opencode/lib/heavy-command.mjs` and its node test.
- Commit `.claude/settings.json` with a `PreToolUse` / `Bash` hook that runs the
  policy from `${CLAUDE_PROJECT_DIR}` and fails closed on a hook error.
- Commit `.codex/hooks.json` with a `PreToolUse` / `Bash` hook that resolves the
  policy from the main checkout (`git rev-parse --path-format=absolute
  --git-common-dir`), so linked worktrees use the same manifest and script.
- Add `scripts/check-agent-command-guard.py` to the `verify-fast.sh` check pool:
  it proves both hook files parse, match `Bash`, reference the policy, carry a
  timeout, and use the correct worktree-relative resolution.
- Document the trust steps (`/hooks` in Codex, the workspace trust dialog in
  Claude Code) and the worktree behavior in `AGENTS.md` and `DEVELOPING.md`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `meta/cpu-budget`: the agent refusal becomes one shared policy with three
  adapters, Claude Code and Codex run the policy from committed hooks, and the
  documentation names the guard and its trust/worktree behavior.

## Impact

- `scripts/heavy_command_policy.py`, `scripts/check-agent-command-guard.py`,
  `scripts/verify-fast.sh`.
- `.opencode/plugins/cpu-budget.ts`; delete `.opencode/lib/heavy-command.mjs`
  and `.opencode/lib/heavy-command.test.mjs`.
- New `.claude/settings.json`, `.codex/hooks.json`.
- `AGENTS.md`, `DEVELOPING.md`.
- No app, engine, build-system, or dependency change. Hooks are guards, not
  sandboxes; the wrapper remains the only path that applies the budget.
- Tracks issue #1460; follows #1457 / PR #1458.
