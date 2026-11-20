# Test Plan

The shared policy and the guard wiring are script behavior; the harness
registration itself is configuration that a static check can parse, while the
live deny/allow path needs a real Claude Code or Codex session. The plan checker
resolves Rust, Qt, and Python-script references, so `scripts/*.py` rows cite the
script directly and the live rows are `manual:`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cb_agent_refuse` | `python3 scripts/heavy_command_policy.py --self-check`; then open a Claude Code, a Codex, and an opencode session and run `cargo nextest run --workspace`, `bash scripts/with-build-lock.sh -- cargo nextest run --workspace`, and `cargo fmt --all` | The bare run is denied before execution with the wrapped form named; the wrapper run and `cargo fmt` proceed; both `--command` and stdin JSON forms refuse | manual: `python3 scripts/heavy_command_policy.py --self-check` covers the policy; the per-harness live denial is a reviewer checklist item (needs a real session and hook trust) |
| `cb_policy_git_root` | Run the policy from a worktree root and from `crates/pictura-app` with a bare `cargo build` | The refusal names `<checkout>/scripts/with-build-lock.sh`, not `scripts/...` relative to the current directory | `heavy_command_policy::self_check` |
| `cb_harness_hooks` | `python3 scripts/check-agent-command-guard.py`; delete one hook or its matcher and rerun | The check passes on the tree; removing a hook, the `Bash` matcher, the policy reference, or the timeout fails it | `check-agent-command-guard::self_check` |
| `cb_harness_worktree` | The same check reads both files; then run Claude Code and Codex from a linked worktree and trigger a bare heavy command | The Claude hook uses `${CLAUDE_PROJECT_DIR}`, the Codex hook uses `--show-toplevel` with a `--git-common-dir` fallback; both deny in the worktree; a missing policy fails open | manual: `python3 scripts/check-agent-command-guard.py` verifies the wiring; the live worktree run is a reviewer checklist item |
| `cb_docs` | Read the command and worktree sections of `AGENTS.md` and `DEVELOPING.md` | The budget, wrapper, shared policy, harness hooks, trust steps, and worktree behavior are named; no bare heavy form appears | manual: reviewed in the documentation diff |
