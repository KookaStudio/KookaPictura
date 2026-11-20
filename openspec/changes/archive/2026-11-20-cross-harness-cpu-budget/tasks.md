# Tasks

## 1. OpenSpec change

- [x] 1.1 Author `proposal.md`, `design.md`, `tasks.md`, `test-plan.md`, and the `meta/cpu-budget` spec delta with `cb_agent_refuse`, `cb_policy_git_root`, `cb_harness_hooks`, `cb_harness_worktree`, and the modified `cb_docs`

## 2. Shared policy

- [x] 2.1 Add `scripts/heavy_command_policy.py`: shell-segment matcher, gate/wrapper exemptions, `cargo fmt` exemption, git-root refusal path, `--command` and stdin JSON input, `--self-check`
- [x] 2.2 Rewire `.opencode/plugins/cpu-budget.ts` to spawn the policy; delete `.opencode/lib/heavy-command.mjs` and its node test
- [x] 2.3 Replace the `heavy-command` node check with `heavy-policy python3 scripts/heavy_command_policy.py --self-check` in `scripts/verify-fast.sh`

## 3. Harness adapters

- [x] 3.1 Add `.claude/settings.json` `PreToolUse` / `Bash` hook with `${CLAUDE_PROJECT_DIR}`, a 10 s timeout, and `onFailure: "block"`
- [x] 3.2 Add `.codex/hooks.json` `PreToolUse` / `Bash` hook resolving the policy from the main checkout via `--git-common-dir`, with a 10 s timeout and fail-open on a missing policy
- [x] 3.3 Add `scripts/check-agent-command-guard.py` (parse both files, matcher `Bash`, policy reference, timeout, worktree resolution, opencode delegation) and wire it into `scripts/verify-fast.sh`

## 4. Docs

- [x] 4.1 Add the guard, trust steps, and worktree behavior to `AGENTS.md` and `DEVELOPING.md`

## 5. Checks and archive

- [x] 5.1 Run `python3 scripts/heavy_command_policy.py --self-check`, `python3 scripts/check-agent-command-guard.py`, `node`-free `verify-fast` checks, and `openspec validate --all --strict`
- [x] 5.2 Run `bash scripts/verify-fast.sh` (note the pre-existing `check-shell-seams` ceiling if it is still red)
- [x] 5.3 Archive `cross-harness-cpu-budget` and confirm no non-archived change remains
