# Tasks

## 1. OpenSpec change

- [x] 1.1 Author `proposal.md`, `design.md`, `tasks.md`, `test-plan.md`, and the `meta/cpu-budget` delta with the modified `cb_agent_refuse`, `cb_policy_git_root`, and the new `cb_policy_quotes`

## 2. Policy

- [x] 2.1 Resolve the refusal wrapper from the policy's own checkout, relative when the cwd is that checkout and absolute otherwise
- [x] 2.2 Split segments only on unquoted `;`, `&&`, `||`, `|`, and newline, with the compound suggestion driven by the same split
- [x] 2.3 Extend `--self-check` with the policy-checkout hint from an unrelated cwd and the quoted-operator cases

## 3. Docs

- [x] 3.1 Name the policy-checkout hint in `AGENTS.md` and `DEVELOPING.md`

## 4. Checks and archive

- [x] 4.1 Run `python3 scripts/heavy_command_policy.py --self-check`, `python3 scripts/check-agent-command-guard.py`, `python3 scripts/check-test-plan.py`, and `openspec validate --all --strict`
- [x] 4.2 Run `bash scripts/verify-fast.sh` (note the pre-existing `check-shell-seams` ceiling)
- [x] 4.3 Archive `budget-policy-checkout-hint` and confirm no non-archived change remains
