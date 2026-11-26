# Design

## Context

`scripts/test-report.sh` already owns separate self-test captures, CTest JUnit
collection, stale-result cleanup, and missing-report validation through
`scripts/report_tests.py`. CI repeats the ordering in YAML and calls the
reporter without CTest registration data. `scripts/verify-full.sh` separately
configures and builds every target before `verify-fast.sh` reaches the shared
script.

The execution path must keep one writer in the CMake build tree, preserve the
self-test token protocol and exit codes, and leave nextest outside the Qt
critical path. #1570 owns the four-vCPU measurement that decides whether the
build-tail overlap is worth retaining.

## Goals / Non-Goals

**Goals:**

- One Qt execution path with thin CI and local adapters.
- A staged build that links `pictura` before either self-test starts.
- An opt-in, measurable overlap with the Qt Test build tail.
- Strict CTest staging, fresh report inputs, deterministic failure identity,
  and child-process cleanup.
- Stage timings that #1570 can compare without inferring durations from logs.

**Non-Goals:**

- Changing either self-test invocation, token bytes, check registry, or exit
  code allocation.
- Merging the two self-test runs. That is #1570's separate scope.
- Overlapping nextest with Qt work or moving nextest under CTest.
- Adding a scheduler, dependency, or new build system.
- Claiming a wall-time win before #1570 measures it.

## Decisions

### Extend the existing script instead of adding an executor

`scripts/test-report.sh` gains `--qt-only`, `--configure`, `--build`, and
repeatable `--cmake-arg` options. Its default local mode still runs nextest
first, then the Qt stages. `--qt-only` skips nextest for the CI Qt job.

This is smaller than a new scheduler and keeps the reporter invocation,
captures, cleanup, and CTest selection in one place. The alternative, a second
Qt script, would duplicate the behavior this change is meant to consolidate.

### Put staging in the shared script

The Qt stage order is:

```text
nextest, unless --qt-only
configure, if --configure
capture CTest registration and remove stale Qt JUnit
build pictura
start both self-test captures
  default: wait for both, then build all remaining targets
  PICTURA_QT_SELFTEST_OVERLAP=1: build all remaining targets concurrently
wait for the build tail and both self-tests
CTest
report_tests.py
```

The configure step keeps an existing generator and uses Ninja for a new tree.
It drops cached linker flags before probing mold/lld, matching the current full
gate. CI passes its compiler launcher and no-PCH options through `--cmake-arg`.
Local `verify-full.sh` passes `--configure --build` through `verify-fast.sh` to
the shared script and removes its own CMake build.

`PICTURA_QT_SELFTEST_OVERLAP` defaults to disabled. #1570 can enable it for
measurement, then the adapters can adopt it only after a positive four-vCPU
result. This keeps the candidate available without retaining an unproven
critical-path change.

### Keep one Ninja writer and make CTest a hard later stage

The script waits for the `pictura` build before starting self-tests. It never
starts the all-target build while the `pictura` build is still running. In
overlap mode, the all-target build is the only background build writer. CTest
starts only after that writer exits successfully and both self-test processes
have been reaped.

A failed `pictura` build skips self-tests, the build tail, and CTest. A failed
build tail skips CTest. The reporter still runs with whatever captures exist
and with CTest registration data when configuration succeeded, so missing Qt
reports remain visible.

### Preserve failure identity with deterministic priority

Each stage records its real exit status. The final status is the first non-zero
status in this order: nextest, Configure, `pictura` build, bare self-test,
document self-test, Qt Test build tail, CTest, reporter. The reporter always
runs before the final exit so test names and missing reports are not lost.

The two self-test outputs stay in separate temporary files and are passed as
two `--selftest` arguments. No C++ code changes, so the token contract and the
`ST_FAIL` exit identity remain intact.

### Clean stale inputs before work and reap children on every exit

Old `build/qt-test-results/*.xml` files are removed after configuration, or at
the start of Qt execution when no configure was requested, before any build or
test can refresh them. CTest registration JSON is captured after configuration
and before the staged builds. Current XML files and that JSON go to the existing
reporter unchanged.

The script tracks build and self-test PIDs. Its EXIT trap terminates and waits
for live children, then removes the temporary capture directory. SIGINT and
SIGTERM exit as 130 and 143. Normal runs wait for every child explicitly so a
failed stage cannot leave a writer or self-test running behind the reporter.

### Keep CI setup outside the executor

The workflow installs Qt with TIFF support, extracts the TIFF runtime, sets
`LD_LIBRARY_PATH`, and sets `PICTURA_CPU_BUDGET_PERCENT=100` so the shared script
uses the runner's full CPU count. One step then runs the shared Qt mode. The
separate Configure, Build, Self-test, Self-test with document, CTest, and Qt
report steps are removed. The live control check stays after the shared step.

### Test the shell contract with fake tools

A new standard-library self-check creates a temporary repository, copies the
real executor and reporter, and puts fake `cmake`, `cargo`, `ctest`, and
`pictura` commands on `PATH`. Event logs prove ordering, overlap, one-writer
behavior, stale cleanup, separate captures, failure identity, missing-report
naming, signal cleanup, and adapter wiring. The fake build refuses a second
writer, so the invariant has a runnable check without compiling the project.

## Risks / Trade-offs

- The overlap can contend with compilation on four vCPUs. It stays off by
  default, has an explicit disable switch, and #1570 must measure it before an
  adapter enables it.
- CI and #1570 are likely to edit the same `qt-headless` steps. The shared
  executor reduces the conflict to one workflow step plus TIFF/CPU setup.
- Capturing CTest registration before the build means a failed build reports
  all configured suites as missing. That is noisy but honest, and the build's
  real exit status remains the failure identity.
- Waiting for self-tests before CTest gives up any self-test/CTest overlap. The
  change targets the build tail, where `pictura` readiness creates a safe new
  overlap point, and avoids adding CPU contention to the long CTest layer.
- A shell executor is harder to unit-test than a Python module. The fake-tool
  self-check exercises the real script and reporter rather than a model of
  them.

## Migration Plan

1. Add the shared script options and staged Qt execution while preserving the
   default local command.
2. Route `verify-full.sh` through the shared build and execution path.
3. Replace the CI Qt orchestration with one adapter step and keep TIFF setup
   before it.
4. Run the fake-tool self-check in local verification and CI.
5. Leave overlap disabled until #1570 records a positive four-vCPU result.

Rollback is a one-line adapter change: remove `--configure --build` to return
to prebuilt-tree execution, or leave
`PICTURA_QT_SELFTEST_OVERLAP` unset to use sequential self-tests and the build
tail.
