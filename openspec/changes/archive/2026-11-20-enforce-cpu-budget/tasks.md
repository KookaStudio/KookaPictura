# Tasks

## 1. OpenSpec change

- [x] 1.1 Author `proposal.md`, `design.md`, `tasks.md`, `test-plan.md`, and the `meta/cpu-budget` spec delta with `cb_wrap_budget`, `cb_agent_refuse`, `cb_sccache_server`, and the modified `cb_docs`
- [x] 1.2 Record the sccache probe output in `design.md` and choose the narrowing fix because a pre-existing wide server is reused

## 2. Wrapper budget

- [x] 2.1 Add the one-shot `PICTURA_CPU_BUDGET_APPLIED` guard and the `PICTURA_JOBS` assertion to `scripts/with-build-lock.sh` before the lock source, keeping the working directory and exit code
- [x] 2.2 Update the wrapper header to say it applies the CPU budget and takes the opt-in lock

## 3. Agent guard

- [x] 3.1 Add `.opencode/lib/heavy-command.mjs` with the shell-segment matcher, the gate/wrapper exemptions, and the wrapped-form message, including the `ponytail:` ceiling
- [x] 3.2 Add `.opencode/plugins/cpu-budget.ts` with the `tool.execute.before` refusal
- [x] 3.3 Add `.opencode/lib/heavy-command.test.mjs` and wire `node --test .opencode/lib/heavy-command.test.mjs` into the `verify-fast.sh` check pool

## 4. Checks

- [x] 4.1 Add `bl_wrap_budget_order` and `bl_wrap_budget_mask` to `scripts/check-build-lock.sh`
- [x] 4.2 Extend `bl_wrap_docs` so the wrapper and the budget wording stay documented in `AGENTS.md` and `DEVELOPING.md`
- [x] 4.3 Add the sccache server discovery and best-effort narrowing to `scripts/cpu-budget.py`, with the stub-daemon case in `--self-check`

## 5. Docs and archive

- [x] 5.1 Wrap the direct heavy forms in `AGENTS.md`, `DEVELOPING.md`, `README.md`, and `.serena/memories/suggested_commands.md`
- [x] 5.2 Run the checks, archive `enforce-cpu-budget`, and confirm no non-archived change remains
