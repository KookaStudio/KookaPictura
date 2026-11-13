# Tasks

## 1. CPU budget helper

- [x] 1.1 Write the `--self-check` cases in `scripts/cpu-budget.py` for `cb_default`, `cb_rounding`, `cb_percent_invalid`, `cb_percent_full`, `cb_slice`, `cb_slice_subset`, `cb_env_exports`, `cb_override`, `cb_pictura_jobs_output`, `cb_marker_skip`, and `cb_self_check`, with the `budget_for`, `validate_percent`, `choose_slice`, and `self_check` functions they name; each exec case runs with `PICTURA_CPU_BUDGET_APPLIED` and the five job variables cleared, and the self-check includes the inherited-marker case; verify `python3 scripts/cpu-budget.py --self-check` fails on the unimplemented exec path
- [x] 1.2 Implement the budget arithmetic, validation, slice selection, and `exec` entry point (affinity, five job exports, `PICTURA_CPU_BUDGET_APPLIED`, the inherited-marker path) until `python3 scripts/cpu-budget.py --self-check` exits 0; the `cb_pictura_jobs_output` case verifies a preset `PICTURA_JOBS` is replaced, and each exec case clears `PICTURA_CPU_BUDGET_APPLIED` and the five job variables for its child
- [x] 1.3 Add a commented `PICTURA_CPU_BUDGET_PERCENT=80` entry to `.env.example` under "External build knobs (set in your shell; not read from this file)", with the comment "process-environment only; default 80, valid 1-100", and update the stale `CTEST_PARALLEL_LEVEL` comment from "default 4" to "defaults to the local CPU budget; CI uses the runner CPU count"; verify `rg -n PICTURA_CPU_BUDGET_PERCENT .env.example` shows it and the file still enables nothing as shipped

## 2. Gate wiring

- [x] 2.1 In `scripts/verify-fast.sh`, re-exec once through `scripts/cpu-budget.py exec` before sourcing the lock, resolving the script directory before `cd`; verify a run with `PICTURA_CPU_BUDGET_APPLIED=1` receives the five job variables without re-exec, a docs-only run reaches the docs-only exit, and a code-change run reaches the lock with the marker set
- [x] 2.2 In `scripts/verify-fast.sh`, start the independent checks in batches of `PICTURA_JOBS`, print a `start <name>` line as each check launches (or a per-batch line), and keep the existing capture and failure reporting; verify a `taskset -c 0-3` run never has more than three checks in flight (manual: read the printed start or batch lines)
- [x] 2.3 In `scripts/verify-full.sh`, re-exec once before sourcing the lock and build with `--parallel "$PICTURA_JOBS"`; verify the CMake build and its Corrosion cargo child report the same `Cpus_allowed_list` (manual: inspect `/proc/<pid>/status`)
- [x] 2.4 In `scripts/test-report.sh`, re-exec once preserving `"$@"` and take the CTest level from `CTEST_PARALLEL_LEVEL` or `PICTURA_JOBS`; verify the `== qt tests (ctest -j N) ==` line shows the budget on a 16-CPU host, `-j 2` with `CTEST_PARALLEL_LEVEL=2`, and a direct `bash scripts/test-report.sh never` still emits no ANSI after the re-exec
- [x] 2.5 Add `start_check cpu-budget python3 scripts/cpu-budget.py --self-check` to the fast-gate check pool; verify `rg -n 'cpu-budget.py --self-check' scripts/verify-fast.sh` shows the call
- [x] 2.6 Document the 80% default, the `PICTURA_CPU_BUDGET_PERCENT` range, the always-on gate re-exec, and the shared slice in `AGENTS.md` and `DEVELOPING.md`; verify the names, default, and range match the helper
- [x] 2.7 Add a `bl_budget_order` static assertion to `scripts/check-build-lock.sh` that the `cpu-budget.py` invocation precedes `. scripts/build-lock.sh` in `verify-fast.sh` and `verify-full.sh`; verify the assertion fails when the order is swapped

## 3. Build-lock wrapper

- [x] 3.1 Extend `scripts/check-build-lock.sh` with the `bl_wrap_queue`, `bl_wrap_failure`, `bl_wrap_nested`, `bl_wrap_disabled`, `bl_wrap_docs`, `bl_wrap_dashdash`, `bl_wrap_signal`, and `bl_wrap_cwd` cases before the wrapper exists; verify the check fails with the missing-wrapper assertion
- [x] 3.2 Implement `scripts/with-build-lock.sh -- <command...>`: source the sibling `build-lock.sh` by the wrapper's own directory without changing the caller's working directory, install the traps, acquire, export `PICTURA_VERIFY_LOCK_HELD=1` only when owned, strip an optional leading `--`, run the command as a child, propagate its exit code, and release; verify `bash scripts/check-build-lock.sh` exits 0
- [x] 3.3 Document the wrapper for direct `cmake --build`, `cargo build`, `cargo nextest`, and `ctest` commands in `AGENTS.md` and `DEVELOPING.md`, stating that the lock stays opt-in; verify both files name the wrapper

## 4. Integration and archive

- [x] 4.1 Run `bash -n` on `scripts/with-build-lock.sh`, `scripts/check-build-lock.sh`, `scripts/verify-fast.sh`, `scripts/verify-full.sh`, and `scripts/test-report.sh`, then `python3 scripts/cpu-budget.py --self-check`, `bash scripts/check-build-lock.sh`, `python3 scripts/check-test-plan.py`, and `openspec validate --all --strict`; verify all exit 0
- [x] 4.2 Archive with `openspec archive build-lock-and-cpu-budget --yes`; verify `openspec list --json` reports no active change and the deltas merged into `openspec/specs/meta/cpu-budget/spec.md`, `openspec/specs/verification/verification-harness/spec.md`, and `openspec/specs/verification/test-reporting/spec.md`
- [x] 4.3 After archiving, run `bash scripts/verify-fast.sh`; verify it is green and the cargo, nextest, self-test, and CTest children report the 80% affinity slice (manual: inspect `Cpus_allowed_list`)
- [x] 4.4 After archiving, run `bash scripts/verify-full.sh`; verify it is green and the CMake, cache-rebuild, and control children inherit the same slice
- [x] 4.5 Verify the cross-worktree behavior manually: two top-level gates started from the same allowed set show the same `Cpus_allowed_list`; sample aggregate gate CPU while both run (`pidstat` or `/proc/stat`) and confirm it stays within the slice; an invalid `PICTURA_CPU_BUDGET_PERCENT` fails with a clear message; and two `with-build-lock.sh` commands queue. Note that the sccache daemon is exempt from the mask but its compiler concurrency is bounded by the job count.
- [x] 4.6 With `PICTURA_VERIFY_LOCK=1`, run `bash scripts/with-build-lock.sh -- bash scripts/verify-fast.sh` and `bash scripts/with-build-lock.sh -- bash scripts/verify-full.sh`; verify both complete without deadlock and leave no lock
- [x] 4.7 After archiving, run `bash scripts/check-openspec-archived.sh` and `bash scripts/verify-fast.sh`; verify both exit 0
- [x] 4.8 List the test plan's `manual:` rows in the pull request body as a reviewer checklist; verify every `manual:` row is present
