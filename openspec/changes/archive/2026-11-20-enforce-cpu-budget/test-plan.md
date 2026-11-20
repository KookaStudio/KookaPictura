# Test Plan

The wrapper's budget re-exec and child mask are shell behavior, exercised by
`scripts/check-build-lock.sh`; `scripts/check-test-plan.py` resolves only Rust,
Qt, and `scripts/*.py` references, so those rows are `manual:` with the case
named. The agent matcher is a Node test that the same checker cannot resolve,
so its row names the exact command. The sccache narrowing is a `cpu-budget.py`
function and carries a resolvable reference.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cb_wrap_budget` | Run `bash scripts/check-build-lock.sh`: `bl_wrap_budget_order` inspects the wrapper, and `bl_wrap_budget_mask` runs the wrapper with the marker and jobs cleared against a `sched_getaffinity` probe | `cpu-budget.py` is invoked before `build-lock.sh` is sourced, and the child mask is `max(1, current*percent//100)` | manual: `scripts/check-build-lock.sh` cases `bl_wrap_budget_order` and `bl_wrap_budget_mask`; the plan checker resolves only Python-script refs |
| `cb_agent_refuse` | `node --test .opencode/lib/heavy-command.test.mjs` | A bare `cargo`/`cmake`/`ctest`/`ninja` segment is refused with the wrapped form named; wrapped, gate, and non-heavy commands pass | manual: `node --test .opencode/lib/heavy-command.test.mjs`; the plan checker resolves only Rust/Qt refs |
| `cb_sccache_server` | `python3 scripts/cpu-budget.py --self-check` spawns a stub `sccache --server`, narrows it, and asserts its mask | The stub's mask is the chosen slice and the process is still alive | `cpu-budget::self_check` |
| `cb_docs` | Read the command blocks in `AGENTS.md`, `DEVELOPING.md`, `README.md`, and `.serena/memories/suggested_commands.md` | Every heavy form is wrapped, and AGENTS/DEVELOPING name the budget, range, shared slice, and wrapper behavior | manual: reviewed in the documentation diff |
