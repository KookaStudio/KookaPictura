# Test Plan

The budget arithmetic, validation, slice choice, exports, and marker are
exercised by `scripts/cpu-budget.py --self-check`, whose functions
`budget_for`, `validate_percent`, `choose_slice`, and `self_check` are named
below. The shell behavior of the lock wrapper is exercised by
`scripts/check-build-lock.sh`; its cases are bash, which
`scripts/check-test-plan.py` cannot resolve, so those rows are `manual:` with
the case named. The modified test-reporting scenarios are existing shell
behavior and stay `manual:`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cb_default` | Run the budget calculation with 16 allowed CPUs and no percent override | The budget is 12 | `cpu-budget::budget_for` |
| `cb_rounding` | Calculate with 3 allowed CPUs at the default percent, then with 1 allowed CPU | The budgets are 2 and 1 | `cpu-budget::budget_for` |
| `cb_percent_invalid` | Validate `0`, `101`, and `abc` | Each is rejected with the value named and a non-zero exit | `cpu-budget::validate_percent` |
| `cb_percent_full` | Calculate with 16 allowed CPUs and percent 100 | The budget is 16 | `cpu-budget::budget_for` |
| `cb_reexec_once` | Run the helper's exec path with the marker unset and a child that prints its marker and mask | The child sees the marker set and the budget slice | manual: verified by tasks 2.1 and 4.2/4.3; the helper-level mechanism is covered by `cpu-budget::self_check` |
| `cb_nested_reuse` | Run exec twice, the second time with the marker set | The second run keeps the first slice and does not narrow it | manual: verified by tasks 2.1 and 4.2/4.3; the helper-level mechanism is covered by `cpu-budget::self_check` |
| `cb_env_exports` | Run exec with no job variables set and print the five variables | All five equal the budget | `cpu-budget::self_check` |
| `cb_override` | Run exec with `CARGO_BUILD_JOBS=3` and print the variables and mask | Cargo is 3, the other four are the budget, and the mask is the budget | `cpu-budget::self_check` |
| `cb_pictura_jobs_output` | Run exec with `PICTURA_JOBS=99` and the budget 12 | The child sees `PICTURA_JOBS=12` | `cpu-budget::self_check` |
| `cb_marker_skip` | Run exec with only the marker set | Affinity is unchanged, the five job variables are filled to the budget, and the command runs | `cpu-budget::self_check` |
| `cb_slice` | Choose a slice from CPUs 0-15 with budget 12 | The slice is CPUs 0-11 | `cpu-budget::choose_slice` |
| `cb_slice_subset` | Choose a slice from CPUs 8-15 with budget 6 | The slice is CPUs 8-13 | `cpu-budget::choose_slice` |
| `cb_shared_slice` | Start two top-level gates from the same allowed set and sample aggregate CPU while both run | Both masks are identical, the runs contend inside the slice, and aggregate gate CPU stays within it | manual: needs two concurrent gate processes and a manual CPU sample; the deterministic choice is unit-checked by `cb_slice` |
| `cb_fallback` | Run the helper on a platform without process-affinity control, or force the mask call to fail | It warns when the call failed, exports the five job variables, and leaves the mask unchanged | manual: needs macOS or git-bash, or a forced mask failure |
| `cb_fast_pool` | Run `taskset -c 0-3 bash scripts/verify-fast.sh` and read the printed `start <name>`/batch lines | No more than three checks are in flight, all thirteen run, and a failure still fails the gate | manual: read the gate's printed start or batch lines; the full gate is too slow for a unit harness |
| `cb_self_check` | Run `python3 scripts/cpu-budget.py --self-check` | Every assertion holds and the script exits 0 | `cpu-budget::self_check` |
| `cb_self_check_fail` | Temporarily weaken the budget formula or the marker check and rerun the self-check | The failing assertion is reported and the script exits non-zero | manual: negative check by weakening the helper |
| `cb_docs` | Read the commands and worktree guidance in `AGENTS.md` and `DEVELOPING.md` | Both name the default, the percent range, the shared slice, and the wrapper | manual: reviewed in the documentation diff |
| `bl_wrap_queue` | Run two wrappers with the lock enabled and a held command | The second reports the holder, waits, and starts after release | manual: `check-build-lock.sh` `bl_wrap_queue` case; the parser resolves only Rust, Qt, and Python-script refs |
| `bl_wrap_failure` | Wrap a command that exits 7 | The wrapper exits 7 and leaves no lock | manual: `check-build-lock.sh` `bl_wrap_failure` case; the parser resolves only Rust, Qt, and Python-script refs |
| `bl_wrap_nested` | Wrap a gate that sources the lock helper, then wrap the real `verify-fast.sh` and `verify-full.sh` | The stub gate does not re-acquire, both real gates complete without deadlock, and the wrapper releases | manual: `check-build-lock.sh` `bl_wrap_nested` case plus task 4.5; the parser resolves only Rust, Qt, and Python-script refs |
| `bl_wrap_disabled` | Wrap a command with `PICTURA_VERIFY_LOCK` unset or `0` | The command runs, no lock appears, and its exit code propagates | manual: `check-build-lock.sh` `bl_wrap_disabled` case; the parser resolves only Rust, Qt, and Python-script refs |
| `bl_wrap_signal` | Send `INT` and `TERM` to a running wrapper | The wrapper releases its lock and does not leave the wrapped child running | manual: check-build-lock.sh bl_wrap_signal case; the parser resolves only Rust, Qt, and Python-script refs |
| `bl_wrap_docs` | Read the commands and worktree guidance in `AGENTS.md` and `DEVELOPING.md` | Both show the wrapper and state that the lock stays opt-in | manual: reviewed in the documentation diff |
| Every layer is summarised | Run `bash scripts/test-report.sh` with all layers passing | The report lists every layer and exits 0 | manual: existing shell behavior exercised by every gate; no unit harness for `test-report.sh` |
| A failure is listed and fails the run | Run `scripts/test-report.sh` with one failing layer | The FAILURES section names the test and the run exits non-zero | manual: existing shell behavior; the modified requirement does not change it |
| Colour output is selectable | Run the reporter with `--color never` and `--color always` | No ANSI escapes in the first, ANSI escapes in the second | manual: existing `report_tests.py` behavior |
| The Qt Test layer is folded in | Run `scripts/test-report.sh` with built Qt suites | The per-executable JUnit is folded into the report | manual: existing shell behavior exercised by every gate |
| `tr_qt_parallel` | Run `scripts/test-report.sh` on a 16-CPU host and read the CTest header | The header reads `-j 12` (the 80% budget) | manual: read the `== qt tests (ctest -j N) ==` line |
| `tr_qt_parallel_override` | Run with `CTEST_PARALLEL_LEVEL=2` | The header reads `-j 2` | manual: read the `== qt tests (ctest -j N) ==` line |
| `tr_qt_parallel_floor` | Run under `taskset -c 0` with no override | The header reads `-j 1` | manual: read the `== qt tests (ctest -j N) ==` line |
| `tr_stale_junit` | Remove a suite executable and rerun the report | The stale JUnit is cleared and the suite is reported as missing | manual: existing `test-report.sh` behavior |
