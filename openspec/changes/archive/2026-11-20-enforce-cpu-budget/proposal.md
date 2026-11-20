# Proposal

## Why

The gates apply the local CPU budget, but two paths still run heavy commands
unbudgeted. `scripts/with-build-lock.sh` takes the opt-in lock and never
re-execs through `scripts/cpu-budget.py`, so a direct command it wraps compiles
on every allowed CPU. And the agent-facing guidance in `AGENTS.md`,
`DEVELOPING.md`, `README.md`, and `.serena/memories/suggested_commands.md`
prescribes bare `cmake`, `cargo`, and `ctest` forms, which an agent executes
outside any gate. On a 16-CPU host that is 16 CPUs for a bare shell, 12 for the
budget helper, and 16 again through the wrapper; two budgeted gates plus one
unbudgeted tree exceed 80% of the machine. A pre-existing `sccache --server`
that a budgeted build connects to also keeps the mask it started with.

## What Changes

- `scripts/with-build-lock.sh` re-execs once through
  `scripts/cpu-budget.py exec` before sourcing the lock or acquiring it, so the
  wrapped command and its children run in the budget's affinity slice with the
  budget's job counts. The lock stays opt-in and the wrapper keeps the caller's
  working directory and the wrapped exit code.
- Add a committed opencode plugin (`.opencode/plugins/cpu-budget.ts` plus the
  pure matcher `.opencode/lib/heavy-command.mjs` and its `node --test` suite)
  that refuses a bare `cargo`, `cmake`, `ctest`, or `ninja` segment before it
  runs and names the wrapped form.
- `scripts/cpu-budget.py` narrows a running `sccache --server` it can address
  to the applied slice, best-effort, without restarting it, so a server started
  outside the budget cannot compile wider than the slice.
- `scripts/check-build-lock.sh` proves the wrapper's budget re-exec ordering,
  the resulting child mask, and the wrapper's documented budget-and-lock role.
- Wrap the direct heavy command forms in `AGENTS.md`, `DEVELOPING.md`,
  `README.md`, and `.serena/memories/suggested_commands.md`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `meta/cpu-budget`: the documented wrapper budgets as well as locks, an agent
  refuses bare heavy commands, the documentation never shows a bare heavy
  command, and a running sccache server is held to the slice.

## Impact

- `scripts/with-build-lock.sh`, `scripts/check-build-lock.sh`,
  `scripts/cpu-budget.py`, `scripts/verify-fast.sh`.
- New `.opencode/plugins/cpu-budget.ts`, `.opencode/lib/heavy-command.mjs`, and
  `.opencode/lib/heavy-command.test.mjs`.
- `AGENTS.md`, `DEVELOPING.md`, `README.md`,
  `.serena/memories/suggested_commands.md`.
- No app, engine, build-system, or dependency change. The lock remains opt-in.
  Tracks issue #1457.
