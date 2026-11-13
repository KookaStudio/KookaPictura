# Design

## Context

See `proposal.md` — Why. The constraints that shape the approach:

- `scripts/build-lock.sh` already provides the opt-in lock, env-file
  precedence, stale reclaim, and traps. Only `verify-fast.sh` and
  `verify-full.sh` source it.
- `verify-full.sh` invokes `verify-fast.sh`, which invokes
  `scripts/test-report.sh`. The nested lock marker (`PICTURA_VERIFY_LOCK_HELD`)
  already handles re-entrancy.
- The gates run on Linux, macOS, and git-bash. `taskset` is Linux-only;
  `os.sched_getaffinity`/`os.sched_setaffinity` exist on Linux and are absent on
  macOS. The scripts support bash 3.2 (`${BASHPID:-$$}`), so they cannot use
  `wait -n` (4.3).
- CI does not invoke the gates; it runs nextest and CTest directly. No CI change
  is in scope.
- `scripts/check-local-build-caching.sh` already reads
  `CMAKE_BUILD_PARALLEL_LEVEL`, so it inherits the budget under
  `verify-full.sh` without a change.

## Goals / Non-Goals

**Goals:**

- Put a direct heavy command under the same opt-in lock with one wrapper.
- Make every gate, its nested gates, and the commands they spawn run in one
  affinity slice of the default 80% of allowed CPUs.
- Export job counts for tools that honor them and for platforms without an
  affinity API.
- Keep nested gates from re-slicing or re-acquiring.
- Prove both behaviors with runnable checks wired into the fast gate.

**Non-Goals:**

- Intercepting unwrapped commands (no PATH or shell shim).
- Cross-user or cross-machine serialization.
- Locking individual compile jobs.
- Applying the CPU budget to direct commands; the wrapper only locks.
- Reading `PICTURA_CPU_BUDGET_PERCENT` from the user-global env file; it is a
  process-environment knob only.
- Per-worktree private slices; the slice is shared.

## Decisions

### Python stdlib affinity instead of `taskset`

The helper sets its own affinity with `os.sched_setaffinity(0, cpus)` and then
`os.execvp`; the mask survives the exec, so the gate and every child inherit it.
When `os.sched_getaffinity` is absent (macOS, git-bash), it exports the job
counts and leaves affinity alone. Alternatives: `taskset -c` (the issue's
wording) adds an external binary and error surface for the same result; a
platform-specific shell branch would need `taskset` on Linux and nothing
elsewhere anyway. The stdlib also lets the self-check read the mask directly.

### The budget re-exec comes before the lock

`exec` replaces the process image: shell variables disappear, traps are reset,
and the EXIT trap does not run. If a gate acquired the lock and then exec'd, the
same PID would still be recorded as the live owner, the new image would not know
it owns the lock, and it would wait on itself until the timeout while the lock
leaks. Therefore each gate re-execs at the top, before it sources
`build-lock.sh`, and the second pass sees `PICTURA_CPU_BUDGET_APPLIED=1` and
continues normally. Each gate re-execs with its original arguments (`"$@"`), so
`test-report.sh` keeps its colour argument.

```
verify-full.sh
  +-- APPLIED unset -> exec cpu-budget.py exec -- bash <abs>/verify-full.sh
  +-- source lock; acquire; export LOCK_HELD when owned
  +-- cmake --build build --parallel "$PICTURA_JOBS"
  +-- verify-fast.sh   (APPLIED -> no re-exec; LOCK_HELD -> no acquire)
  +-- verify-control.sh (inherits the slice)
```

The re-exec resolves the script's directory before `cd` so a gate invoked as
`bash scripts/verify-full.sh` from any working directory re-execs the same file.

### The budget is always on; percent 100 is the escape hatch

The issue's acceptance requires every gate child to show the 80% slice without
an opt-in. The lock stays opt-in. A separate on/off variable would be a second
knob for the same thing, so `PICTURA_CPU_BUDGET_PERCENT=100` is the documented
way to use every CPU.

### `PICTURA_JOBS` is an output; per-tool variables are the overrides

The helper derives `PICTURA_JOBS` from the percent and the allowed set, then
fills `CARGO_BUILD_JOBS`, `CMAKE_BUILD_PARALLEL_LEVEL`, `CTEST_PARALLEL_LEVEL`,
and `NEXTEST_TEST_THREADS` only when they are absent. Treating `PICTURA_JOBS`
as an input too would create a second way to say "job count" that disagrees with
the slice. A user who wants fewer compile jobs sets `CARGO_BUILD_JOBS`; the
slice still caps the process tree.

### nextest uses the exported variables, not new flags

`cargo nextest run --help` shows `NEXTEST_TEST_THREADS` as the environment form
of `--test-threads`, and `CARGO_BUILD_JOBS` caps its build phase. Relying on the
exports keeps `test-report.sh` unchanged in that step and preserves an explicit
user override without a flag-versus-environment conflict. Explicit
`--test-threads`/`--build-jobs` flags were rejected as a second source of the
same number.

### CTest level comes from the budget, not `nproc`

`nproc` respects the affinity mask. Under a 12-CPU slice, the old
`nproc * 8 / 10` formula would pick 9, double-discounting the 80% budget. The
script therefore uses `${CTEST_PARALLEL_LEVEL:-$PICTURA_JOBS}`. The existing
`CTEST_PARALLEL_LEVEL` override still wins, and CI's direct CTest invocation is
unchanged.

### The fast-gate check pool uses batches

`verify-fast.sh` starts at most `PICTURA_JOBS` checks, waits for that batch, and
starts the next. Batching is a few lines of bash and works on bash 3.2; `wait
-n` (4.3) would be more work-conserving but adds a version requirement the
scripts do not otherwise have. On a 16-CPU host the default budget is 12 and the
thirteen checks run as two batches (12 + 1). The cost is a barrier per batch on
smaller machines, acceptable for thirteen mostly cheap checks. Alternatives:
`wait -n` (rejected for bash 3.2), `xargs -P` (the checks are shell functions,
not exportable), and leaving the pool unbounded (violates the acceptance).

### The wrapper runs the command as a child

`scripts/with-build-lock.sh` sources `build-lock.sh`, installs the traps,
acquires, exports `PICTURA_VERIFY_LOCK_HELD=1` only when it owns the lock, runs
`"$@"`, records the status, releases, and exits with that status. It must not
`exec` the command: exec skips the EXIT trap and leaves the lock directory
behind (the next run would reclaim it as stale, but the contract says release on
exit). A gate run under the wrapper sees the held marker and neither acquires
nor releases.

### Two checks, matching the existing patterns

`scripts/check-build-lock.sh` gains wrapper cases: queue, failing-command exit
code, nested gate, disabled path, and the documentation assertion. It already
has the sandbox and the helper-level style. `scripts/cpu-budget.py
--self-check` covers the arithmetic, validation, slice choice, exports,
overrides, and marker; `verify-fast.sh` runs it in the check pool. Neither is
wired into CI, matching the existing lock check.

### The slice is the lowest-numbered CPUs of the current allowed set

Sorting the allowed set makes the choice deterministic, so two top-level gates
that start from the same mask pick the same slice. A gate started under an
already-restricted mask slices from that mask, which is the useful behavior for
a nested or externally constrained run.

## Risks / Trade-offs

- **A gate execs after acquiring the lock** → the ordering rule above; the
  check asserts the budget call precedes the lock source in each gate.
- **sccache daemon children inherit the daemon's mask, not the gate's** → the
  aggregate-CPU check is a manual sample, and the daemon is exempt from the
  mask; the number of concurrent compiler jobs is still bounded by the job
  count, and the acceptance names the gate's ctest, cargo, and self-test
  children.
- **Explicit job overrides can oversubscribe the slice** → the slice is the hard
  ceiling; an override only changes how many jobs share it, by user intent.
- **macOS and git-bash have no hard ceiling** → job counts are exported and the
  fallback is documented; the acceptance's affinity check is Linux-specific.
- **Batches add barriers on small machines** → accepted for thirteen checks; the
  16-CPU case is two batches.
- **A stale inherited `PICTURA_CPU_BUDGET_APPLIED` marker skips the affinity
  restriction** → the helper still fills the job variables, so the gate runs but
  without a hard CPU ceiling; the cost is a lost optimization, not a crash.
- **A relative `$0` breaks the re-exec after `cd`** → resolve the script
  directory before changing directory.
- **Two gates starting from different allowed sets pick different slices** →
  the shared-slice guarantee is stated for the same allowed set.
- **The docs-only fast path re-execs once** → harmless and cheap; it still never
  acquires the lock.

## Migration Plan

- No data or build migration. Local gates change behavior immediately; percent
  100 restores the old CPU usage.
- Rollback: set `PICTURA_CPU_BUDGET_PERCENT=100`, or revert the scripts. The
  lock wrapper is opt-in and can be ignored.
- CI is unaffected.

## Open Questions

None.
