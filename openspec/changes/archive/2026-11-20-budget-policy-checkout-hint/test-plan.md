# Test Plan

The policy is a Python script with a `--self-check`; `scripts/check-test-plan.py`
resolves `scripts/*.py` references, so the policy self-check is cited directly.
The live harness denial is exercised by the existing reviewer checklist from the
cross-harness change and is not repeated here.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cb_agent_refuse` | `python3 scripts/heavy_command_policy.py --self-check`; then run a bare heavy command in a harness session | The policy refuses with the wrapped form; wrapped, gate, and `cargo fmt` commands pass | manual: `python3 scripts/heavy_command_policy.py --self-check` covers the policy; the live adapter path is unchanged from #1464 |
| `cb_policy_git_root` | Run the policy from an unrelated cwd (the self-check uses a temporary directory) | The refusal names the wrapper under the policy's checkout, not one under the caller's cwd | `heavy_command_policy::self_check` |
| `cb_policy_quotes` | Run the policy on `ps -eo pid,comm,args \| awk '$2 ~ /^(cmake\|ninja)$/ {print}'`, `echo 'x; cargo build'`, and `git status && cargo test` | The quoted pipe and semicolon pass; the real `&&` before `cargo test` refuses | `heavy_command_policy::self_check` |
| `cb_docs` | Read the worktree and guard sections of `AGENTS.md` and `DEVELOPING.md` | Both name the policy-checkout hint and the trust/worktree behavior | manual: reviewed in the documentation diff |
