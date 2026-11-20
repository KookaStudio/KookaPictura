# Design

## Context

See `proposal.md` — Why. The measured state on this 16-CPU host:

| Path | Allowed CPUs |
|---|---|
| bare shell | 16 |
| `python3 scripts/cpu-budget.py exec -- <cmd>` | 12 |
| `bash scripts/with-build-lock.sh -- <cmd>` (before this change) | 16 |
| `bash scripts/with-build-lock.sh -- <cmd>` (after this change) | 12 |

The gates are already correct: `verify-fast.sh`, `verify-full.sh`, and
`test-report.sh` re-exec once through `cpu-budget.py` before sourcing the lock,
nested gates reuse `PICTURA_CPU_BUDGET_APPLIED`, and `cpu-budget.py` selects the
lowest `budget` CPUs of the current mask. The two unbound paths are the direct
wrapper and the guidance that teaches agents to run heavy commands bare.

`sccache` probe (Linux, `Cpus_allowed_list` from `/proc/<pid>/status`):

```
bare:     pid=77822 Cpus_allowed_list: 0-15
budgeted: pid=78205 Cpus_allowed_list: 0-11
```

A server started under the budget inherits the slice, but a pre-existing wide
server is reused by a budgeted build; in the probe both server processes were
alive, the wide one at `0-15`. The fix therefore narrows a running server.

## Goals / Non-Goals

**Goals:**

- Make `scripts/with-build-lock.sh` apply the same budget the gates apply,
  before it touches the lock.
- Refuse an agent's bare heavy command before it runs, naming the exact
  wrapped form.
- Hold an already-running sccache server to the slice.
- Keep the documentation's heavy command forms wrapped.
- Prove all three with runnable checks wired into `verify-fast.sh`.

**Non-Goals:**

- Enforcing the budget outside opencode (other harnesses, a human's bare
  shell). Prose and the wrapper remain the baseline.
- Catching env-prefixed (`FOO=1 cargo build`), absolute-path (`/usr/bin/cargo`),
  or otherwise shell-constructed invocations the matcher does not see.
- Restarting or reconfiguring sccache.
- Changing the opt-in lock's default, the gate re-exec, or the shared-slice
  rule.

## Decisions

### The wrapper re-execs before the lock, exactly like the gates

The gates re-exec at the top because `exec` replaces the process image: traps
reset, and a lock acquired before the exec would leak against the same PID.
The wrapper follows the same shape and the same guard:

```bash
if [ -z "${PICTURA_CPU_BUDGET_APPLIED:-}" ]; then
    exec python3 "$script_dir/cpu-budget.py" exec -- bash "$script_dir/with-build-lock.sh" "$@"
fi
: "${PICTURA_JOBS:?...}"
```

`$script_dir` is absolute, so the re-exec survives any working directory, and
`"$@"` keeps the optional `--` and the command. The second pass sees the marker
and continues to the lock, exactly as before. A wrapper invoked from a gate
inherits the marker and does not re-exec.

### The plugin refuses instead of rewriting

opencode 1.19's `tool.execute.before` hook receives the tool name and
`output.args`; a thrown error aborts the call and the message reaches the
model. Throwing makes the refusal visible and leaves the requested command
intact for the user to inspect, where silently rewriting `output.args.command`
would hide the change and could mangle quoting. The matcher lives in a pure
`.mjs` module so `node --test` runs it without a TypeScript loader, and the
plugin imports it from the `.ts` entry point. The matcher is deliberately
coarse: it anchors on shell segments and exempts anything whose text names a
gate or the wrapper.

```
ponytail: segment match only; env-prefixed, absolute-path, and non-opencode
invocations bypass it. Prose guidance and the wrapper remain the baseline.
```

### sccache is narrowed, never restarted

`cpu-budget.py` scans `/proc/*/cmdline` for `sccache` plus `--server` and calls
`os.sched_setaffinity(pid, slice)` best-effort, silently skipping a process it
cannot address or a platform without `/proc`/affinity control. It runs after the
process's own mask is applied, so the slice is the one the budget just chose.
Restarting the daemon would discard a cache warm-up and could race a concurrent
build; narrowing is idempotent and a no-op when the server is already inside the
slice. The self-check spawns a stub daemon (`python -c sleep sccache --server`),
narrows only that PID, and asserts its mask.

### The documented forms become the wrapped forms

`AGENTS.md` and `DEVELOPING.md` keep their budget-and-lock prose and show
wrapped direct commands. `README.md` and
`.serena/memories/suggested_commands.md` follow. `cargo fmt` may stay bare: it
runs no compiler. A compound configure-and-build line uses
`bash scripts/with-build-lock.sh -- bash -c '...'` so the wrapper sees one
command.

## Risks / Trade-offs

- **The matcher is segment-based, not a shell parser** → `cargo fmt` is exempted
  because it runs no compiler; other cheap `cargo` subcommands and
  env-prefixed or absolute-path forms are not special-cased. The wrapper and
  prose carry the baseline.
- **A shell-constructed or env-prefixed command bypasses the matcher** →
  documented as a residual; the wrapper and prose carry the baseline.
- **Narrowing a shared server can slow a concurrent build that started outside
  the budget** → the server's work is bounded by the same debug/release cache it
  serves, and a concurrent gate is expected to use the same shared slice.
- **Narrowing during `--self-check` can lower a live server's mask** → the
  self-check only narrows its own stub PID; production narrowing uses the
  server discovery path.
- **A stale inherited marker skips the wrapper's affinity restriction** → the
  helper still fills the job variables; same accepted trade-off as the gates.

## Migration Plan

- No data or build migration. Direct commands change behavior immediately;
  `PICTURA_CPU_BUDGET_PERCENT=100` restores every allowed CPU.
- Rollback: revert the wrapper guard, the plugin, and the helper's sccache
  narrowing.

## Open Questions

None.
