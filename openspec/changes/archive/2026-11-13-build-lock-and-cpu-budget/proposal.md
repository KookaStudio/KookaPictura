# Proposal

## Why

The opt-in build lock from #468/#724 covers only `scripts/verify-fast.sh` and
`scripts/verify-full.sh`; a direct `cmake --build`, `cargo build`, `cargo
nextest`, or `ctest` still compiles beside another worktree. The lock also does
not bound a single run: the gates pass no job limits, Ninja defaults to CPUs+2,
nextest and Cargo default to every CPU, and nested rayon pools, ImageMagick
children, and mold fan out inside one process, so job flags alone cannot hold a
CPU ceiling. Two worktrees running gates can still saturate the machine.

## What Changes

- Add `scripts/with-build-lock.sh -- <command...>`: source `build-lock.sh`,
  install the signal traps, acquire the opt-in lock, export
  `PICTURA_VERIFY_LOCK_HELD=1` when it owns the lock, run the command as a
  child, propagate its exit code, and release on exit or signal. Document it for
  direct heavy commands in `AGENTS.md` and `DEVELOPING.md`.
- Add `scripts/cpu-budget.py`:
  - Budget = `max(1, floor(allowed_cpus * percent / 100))`, default percent 80,
    overridable with `PICTURA_CPU_BUDGET_PERCENT` (validated 1-100).
  - `exec <command...>` applies an affinity slice of the first N allowed CPUs
    through Python's `os.sched_setaffinity` when the platform provides it,
    exports `PICTURA_CPU_BUDGET_APPLIED=1`, `PICTURA_JOBS`,
    `CARGO_BUILD_JOBS`, `CMAKE_BUILD_PARALLEL_LEVEL`, `CTEST_PARALLEL_LEVEL`,
    and `NEXTEST_TEST_THREADS`, and re-execs the command. A variable already set
    in the environment wins. Without an affinity API (macOS, git-bash) it
    exports the job counts only.
  - `--self-check` covers the rounding floor, the one-CPU case, percent
    validation, slice selection, the job exports, explicit overrides, and the
    wrap-once marker.
- `verify-fast.sh`, `verify-full.sh`, and `test-report.sh` re-exec once through
  the helper before they source or acquire the lock; nested invocations
  (`verify-full` -> `verify-fast` -> `test-report`) reuse the same slice.
- `verify-full.sh` builds with `--parallel "$PICTURA_JOBS"`; `test-report.sh`
  takes the CTest level from `CTEST_PARALLEL_LEVEL` or `PICTURA_JOBS` instead of
  computing 80% of the already-sliced CPUs, and nextest reads the exported
  thread and build-job variables; `verify-fast.sh` bounds its parallel check
  pool to `PICTURA_JOBS`.
- Extend `scripts/check-build-lock.sh` with wrapper queue, failing-command,
  nested, and disabled scenarios.
- Document the budget, `PICTURA_CPU_BUDGET_PERCENT`, and the wrapper in
  `AGENTS.md`, `DEVELOPING.md`, and `.env.example`.
- No CI change: the dedicated runners keep every CPU.

## Capabilities

### New Capabilities

- `meta/cpu-budget`: the local verification CPU budget, the shared affinity
  slice, the job-count exports, the bounded fast-gate check pool, and the
  runnable budget self-check.

### Modified Capabilities

- `verification/verification-harness`: add a requirement for the wrapper that
  puts a direct heavy command under the existing opt-in lock.
- `verification/test-reporting`: the unified report's CTest parallel level now
  comes from the shared CPU budget instead of 80% of the CPUs available after
  the slice.

## Impact

- `scripts/verify-fast.sh`, `scripts/verify-full.sh`, `scripts/test-report.sh`,
  `scripts/check-build-lock.sh`.
- New `scripts/cpu-budget.py` and `scripts/with-build-lock.sh`.
- `AGENTS.md`, `DEVELOPING.md`, `.env.example`.
- No app or engine code, no new dependency, and no CI change. The lock remains
  opt-in; the CPU budget is on by default for the local gates, with percent 100
  as the escape hatch.
- Tracks issue #728; the wrapper half also closes the direct-command gap left by
  #468.
