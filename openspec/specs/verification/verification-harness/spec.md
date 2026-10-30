# verification-harness Specification

## Purpose
Golden-image comparison, stable hashing, the pictura-diff CLI, CI gates, the non-goal guard, and a live control check.

## Requirements

### Requirement: Golden-image comparison
The system SHALL provide `compare(a, b, tolerance) -> Result<Diff, _>` that
compares two equal-length 8-bit buffers using an absolute per-sample tolerance
and reports the total sample count, the number of samples above tolerance, the
maximum delta, the summed delta, and the mean delta; it SHALL return an error
when the buffer lengths differ.

#### Scenario: Samples within tolerance
- **WHEN** every sample differs by no more than the tolerance
- **THEN** `differing` is zero and `Diff::is_empty()` is true

#### Scenario: Samples above tolerance
- **WHEN** a sample differs by more than the tolerance
- **THEN** it is counted in `differing` and `max_delta` records the largest delta

#### Scenario: Buffer length mismatch
- **WHEN** the two buffers have different lengths
- **THEN** `compare` returns an error and does not compare any samples

### Requirement: Stable content hash
The system SHALL provide `hash_bytes` that computes a deterministic content hash,
so identical bytes always hash equally and a changed byte changes the hash, for
use as a determinism gate.

#### Scenario: Identical bytes hash equally
- **WHEN** the same bytes are hashed twice, including in separate runs
- **THEN** both hashes are equal

#### Scenario: Changed bytes change the hash
- **WHEN** one input byte changes
- **THEN** the resulting hash differs

### Requirement: pictura-diff CLI
The system SHALL provide a `pictura-diff` binary that reads two raw 8-bit files,
accepts a `--tolerance` (or `-t`) value, prints the sample, differing, max-delta,
and mean-delta metrics, prints usage for `--help`, and exits non-zero on any
difference above tolerance, IO error, unknown option, or length mismatch.

#### Scenario: Identical files
- **WHEN** both input files are byte-identical
- **THEN** the metrics are printed and the process exits 0

#### Scenario: Files differ beyond tolerance
- **WHEN** any sample differs by more than the given tolerance
- **THEN** the process exits non-zero

#### Scenario: Wrong number of files
- **WHEN** fewer or more than two files are given
- **THEN** usage is printed and the process exits non-zero

### Requirement: CI runs format, lint, and tests

The system SHALL run, on every push to `master` and on every pull request,
`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, and the workspace test suite on the pinned toolchain, except when
the "Jobs are skipped when the change cannot affect them" requirement skips a
job because the changed paths cannot affect it. Format and
clippy SHALL run in a `lint` job and the test suite SHALL run in a separate
`test` job, so the two run in parallel. The `test` job SHALL run the whole
workspace under `cargo nextest run --workspace`; because that suite includes
the Qt application, the job SHALL install Qt. The workspace libraries SHALL
disable doctests, so no gate runs a separate doctest pass and no doc example
can become a test that nothing runs. `scripts/verify-fast.sh` SHALL run the
same nextest invocation, so the local fast gate and CI agree on the runner. The nextest run SHALL emit a JUnit XML artifact through
the repository's `.config/nextest.toml`, and CI SHALL upload that artifact. CI
SHALL build the Qt application under a pinned Qt 6 in a headless job and SHALL
run its `--headless --self-test` mode there. That job SHALL run the Qt Test layer
through CTest with the same bounded parallel level as the local report, and SHALL
install mold so the linker path used locally is exercised in CI. The `test` job
SHALL install the external comparison tools (ImageMagick, `psd-tools`, and
`ag-psd`) before its nextest run, so the oracle tests compare against those
tools in the same compile and run as the rest of the workspace, and no separate
job recompiles the workspace to run them. The job SHALL cache the ImageMagick
download between runs. CI SHALL cache Rust registry sources and compiled
artifacts between runs. The
system SHALL run `cargo deny check` when a `deny.toml` is present, installing
cargo-deny from a prebuilt release rather than compiling it from source. Every
CI job SHALL set a `timeout-minutes` bound.

#### Scenario: Push or pull request

- **WHEN** a commit is pushed to `master`, or a pull request is opened or synchronized
- **THEN** the workflow runs the format, clippy, and test steps in every job the changed-path rules do not skip, and fails the job on a non-zero exit

#### Scenario: A feature-branch push is covered once

- **WHEN** a commit is pushed to a branch that has an open pull request
- **THEN** the build jobs run once, via the pull request `synchronize` event, rather than once for the push and once for the pull request

#### Scenario: Tests run under nextest

- **WHEN** the workflow runs the test step
- **THEN** the workspace suite runs under `cargo nextest run --workspace` and no separate doctest run follows

#### Scenario: Lint and test run in parallel

- **WHEN** the workflow starts
- **THEN** format and clippy run in the `lint` job while the test suite runs in the `test` job, independent of each other

#### Scenario: The local fast gate uses nextest

- **WHEN** `scripts/verify-fast.sh` runs the test step
- **THEN** it runs `cargo nextest run --workspace --no-fail-fast` and no `cargo test --doc` run

#### Scenario: The JUnit artifact is uploaded

- **WHEN** the workflow test step completes
- **THEN** `target/nextest/default/junit.xml` has been written and is uploaded as an artifact

#### Scenario: The Qt app self-tests headlessly

- **WHEN** the workflow builds the Qt application with the pinned Qt 6 toolchain
- **THEN** it runs `--headless --self-test` with and without a document fixture and fails the job on a non-zero exit

#### Scenario: External oracles are exercised

- **WHEN** the `test` job runs its nextest step
- **THEN** ImageMagick, `psd-tools`, and `ag-psd` are installed, the tests that otherwise self-skip compare against the external tools instead of passing vacuously, and no other job compiles the workspace to run them

#### Scenario: Doctests are disabled [vh_no_doctests]

- **WHEN** the workspace manifests are inspected
- **THEN** every workspace library sets `doctest = false`, and neither CI nor `scripts/test-report.sh` runs `cargo test --doc`

#### Scenario: Compiled artifacts are reused

- **WHEN** a later workflow run starts with a populated Rust registry cache and compiler cache
- **THEN** dependencies and previously compiled objects are restored rather than rebuilt from scratch

#### Scenario: A runaway job is bounded

- **WHEN** a CI job exceeds its configured `timeout-minutes`
- **THEN** the run is cancelled rather than consuming minutes indefinitely

#### Scenario: CI runs Qt Test suites concurrently [vh_ci_ctest_parallel]

- **WHEN** the CI headless job runs CTest after the build
- **THEN** it passes a bounded parallel level and every registered suite still emits its own JUnit file

#### Scenario: CI exercises the configured linker [vh_ci_mold]

- **WHEN** the CI headless job installs the toolchain and configures the project
- **THEN** mold is available and the C++ links use it, so the local linker path is tested in CI

### Requirement: Non-goal guard
The system SHALL provide `scripts/guard.sh` that fails when tracked source under
`crates/` references `.8bf` plugin binaries, when an `artboard*` file appears
under `crates/`, or when `docs/` is modified without a `TASK-ALLOWS-DOCS` marker,
and SHALL exit 0 when none of those conditions hold.

#### Scenario: .8bf reference under crates/
- **WHEN** a tracked `crates/` source contains an `.8bf` reference
- **THEN** the guard exits non-zero and prints the offending lines

#### Scenario: artboard artifact under crates/
- **WHEN** a file named `artboard*` exists under `crates/`
- **THEN** the guard exits non-zero and names the file

#### Scenario: docs modified without a marker
- **WHEN** `docs/` differs from the guard base and no commit carries `TASK-ALLOWS-DOCS`
- **THEN** the guard exits non-zero and lists the changed files

#### Scenario: Clean tree
- **WHEN** no non-goal artifact is present and docs are unchanged or allowed
- **THEN** the guard exits 0

### Requirement: A live control-server end-to-end check

The repository SHALL provide `scripts/verify-control.sh` that launches a separate
`pictura --headless --control` process on a temporary local socket, connects over
that socket, and drives a documented recipe over the newline-delimited JSON
protocol: read `status`, apply a rectangular selection that is a strict subset of
the active layer, apply a `filter`, assert the filter changed at least one pixel
and changed no pixel outside the selection, `undo` and assert every pixel is
restored, and take a `screenshot`. The script SHALL exit non-zero on any mismatch
or if `./build/pictura` is absent, and SHALL tear the process down and remove the
socket on exit. `scripts/verify-full.sh` SHALL run the script after it builds the
binary, the build-free `verify-fast.sh` SHALL NOT, and the CI headless job SHALL
run it too.

#### Scenario: The live recipe passes on a built tree

- **WHEN** `scripts/verify-control.sh` runs against a freshly built binary and the committed fixture
- **THEN** every step's assertion holds and the script exits 0

#### Scenario: A mismatch fails the gate

- **WHEN** an assertion does not hold (a pixel that should have changed did not, the undo did not restore it, or the screenshot is not a PNG)
- **THEN** the script prints the failing step and exits non-zero

#### Scenario: The process and socket are cleaned up

- **WHEN** the script finishes, successfully or not
- **THEN** the launched `pictura` process is terminated and the temporary socket directory is removed

#### Scenario: The full gate runs the live check

- **WHEN** `scripts/verify-full.sh` runs
- **THEN** it runs `scripts/verify-control.sh` after the build and fails if the script fails

#### Scenario: CI runs the live check

- **WHEN** the CI headless job runs after building the app
- **THEN** it runs `scripts/verify-control.sh` and the job fails if the script fails

### Requirement: Superseded CI runs are cancelled
The CI workflow SHALL set a `concurrency` group keyed on the workflow and ref
with `cancel-in-progress: true`, so a newer push to the same ref cancels the
older in-progress run.

#### Scenario: Rapid pushes on one ref
- **WHEN** a second push to the same ref starts while the first run is in progress
- **THEN** the first run is cancelled and the second proceeds

#### Scenario: Different refs run independently
- **WHEN** pushes arrive on two different refs
- **THEN** both runs proceed without cancelling each other

### Requirement: A runnable worktree-hook check

The repository SHALL provide `scripts/check-worktree-hook.sh` that proves the hook end to end without touching the real checkout. The script SHALL build an isolated sandbox under `mktemp -d`: `git init` a temporary main repository, set a throwaway `user.name`/`user.email` for its commits, seed a `reference-fixtures` directory in it, set `core.hooksPath` to `.githooks`, commit the hook, and create a linked worktree. It SHALL assert that both symlinks resolve as specified by the worktree-wiring capability, run a second checkout and assert that nothing new appeared, exercise the missing-fixtures skip path, and remove the temporary repository on exit whether the assertions pass or fail. The script SHALL exit non-zero on any failed assertion.

#### Scenario: The check passes on a correct hook

- **WHEN** `scripts/check-worktree-hook.sh` runs with the committed `.githooks/post-checkout` in place
- **THEN** every assertion holds, the temporary repository is removed, and the script exits 0

#### Scenario: A broken hook fails the check

- **WHEN** the hook is changed so a fresh linked worktree does not get one of the required symlinks
- **THEN** the script prints the failing assertion and exits non-zero

#### Scenario: The fixtures guard stays green

- **WHEN** `scripts/guard.sh` runs with `scripts/check-worktree-hook.sh` tracked
- **THEN** the untracked-fixtures check does not flag the script

### Requirement: Docs-only changes run guards, not builds

The build jobs (format/lint/test and Qt headless) SHALL be skipped when
every changed path matches `docs/**`, `openspec/**`, `**/*.md`, or `.serena/**`,
using workflow-level `paths-ignore`. A separate workflow with no path filter
SHALL run `scripts/guard.sh`, `openspec validate --all --strict`,
`scripts/check-file-size.sh`, `scripts/check-selftest-budget.sh`,
`scripts/check-test-plan.py`, `scripts/check-worktree-hook.sh`, and
`scripts/check-openspec-archived.sh` on
every push and pull request, so a docs-only change is still gated. A mixed change
SHALL run the build jobs, because `paths-ignore` skips a workflow only when all
changed paths match. `scripts/verify-fast.sh` SHALL run
`scripts/check-worktree-hook.sh` and `scripts/check-openspec-archived.sh` in its
full gate alongside the other script checks.

#### Scenario: Docs-only change

- **WHEN** a push or pull request changes only `docs/**`, `openspec/**`, `*.md`, or `.serena/**`
- **THEN** the build jobs do not run and the guards workflow runs the guard, OpenSpec validation, file-size, self-test budget, worktree-hook, and archive checks

#### Scenario: Code change

- **WHEN** a change touches any path outside the ignored set
- **THEN** the build jobs run as before

#### Scenario: Guards fail

- **WHEN** `scripts/guard.sh`, `openspec validate --all --strict`, `scripts/check-file-size.sh`, `scripts/check-selftest-budget.sh`, `scripts/check-test-plan.py`, `scripts/check-worktree-hook.sh`, or `scripts/check-openspec-archived.sh` exits non-zero
- **THEN** the guards workflow fails

#### Scenario: The fast full gate runs the worktree-hook check

- **WHEN** `scripts/verify-fast.sh` runs its full gate
- **THEN** it runs `scripts/check-worktree-hook.sh` and fails if the script fails

#### Scenario: The guards workflow runs the archive check

- **WHEN** the guards workflow runs on any push or pull request
- **THEN** it runs `scripts/check-openspec-archived.sh` and fails on a non-archived change directory

### Requirement: Jobs are skipped when the change cannot affect them
For a change that is not docs-only (the docs-only case is governed by
"Docs-only changes run guards, not builds"), CI SHALL evaluate the changed
paths — the pull request diff, or the commits of a push — and SHALL skip each
build job (`lint`, `test`, `qt-headless`) whose inputs the change
cannot affect, while still starting the workflow. Skipping SHALL happen per job
inside a workflow that runs on the event, so a skipped job reports a successful
conclusion instead of reporting nothing at all; a skipped job MUST NOT depend on
a workflow-level path filter, so that a check later made required can never
leave a pull request waiting forever for a report. The evaluation SHALL apply
these rules:

- Any changed path that is not recognized by the rules below SHALL cause every
  job to run (run-on-doubt).
- Any change under `.github/workflows/**` SHALL cause every job to run.
- Any change to Rust sources or other inputs that feed the Rust build SHALL
  cause every job to run, because the Rust staticlib is linked into the Qt
  application and exercised by the test suite.
- A change limited to the hand-written C++ sources under
  `crates/pictura-app/cpp/` SHALL run `qt-headless` (and the guards workflow
  continues to run as before) and SHALL skip `lint` and `test`,
  because the cargo build does not compile those translation units.
- The CI workflow file itself SHALL be treated as a path that causes every job
  to run, so editing the gating rules re-runs the full suite.

#### Scenario: C++-only change
- **WHEN** a pull request changes only files under `crates/pictura-app/cpp/`
- **THEN** `qt-headless` runs, `lint` and `test` are skipped, and the guards workflow still runs

#### Scenario: Rust change runs everything
- **WHEN** a pull request changes any file under `crates/` outside `crates/pictura-app/cpp/`
- **THEN** `lint`, `test`, and `qt-headless` all run

#### Scenario: Mixed change runs everything
- **WHEN** a pull request changes both C++ sources and any other recognized input
- **THEN** every build job runs

#### Scenario: Unrecognized path runs everything
- **WHEN** a pull request changes a path not covered by the rules (for example a root-level config file)
- **THEN** every build job runs

#### Scenario: Workflow edit re-runs the suite
- **WHEN** a pull request changes `.github/workflows/ci.yml`
- **THEN** every build job runs even though the diff is otherwise empty of source

#### Scenario: A skipped job still reports
- **WHEN** the gating condition skips a build job on a pull request
- **THEN** that job's check run reports a successful conclusion rather than never reporting, so it cannot block a required check

### Requirement: Paint present-tiling self-check

The C++ self-test SHALL include a check, with the next free append-only exit
code, that paints several dabs of one stroke between two presents and asserts
that the view pyramid is rebuilt at most twice across the frame — strictly fewer
times than dabs — and that the committed pixels equal the same stroke committed
through the CPU path. The check SHALL cover the GPU path when an adapter is
present and the frame-bounded exact path otherwise, and SHALL report PASS with
the observed dab and rebuild counts.

#### Scenario: The frame rebuild is observed [vh_pyramid_selfcheck]

- **WHEN** the paint present-tiling self-check runs
- **THEN** it reports the dab count and a view-pyramid rebuild count no larger
  than twice the frame count and smaller than the dab count

#### Scenario: The committed pixels match the CPU path [vh_pyramid_commit]

- **WHEN** the same stroke is committed on the checked path and on the CPU path
- **THEN** their committed images are byte-identical

#### Scenario: The collapsed commit blit equals the level-0 crop [vh_commit_blit]

- **WHEN** the unit check commits a single-region stroke and compares its blit
  image with the level-0 crop of the same rectangle
- **THEN** every pixel channel is identical

### Requirement: Local verification fast path

`scripts/verify-fast.sh` SHALL detect the changed paths of the working tree
(committed diff against the merge base with `master`, plus untracked files). When
every changed path matches `docs/**`, `openspec/**`, `*.md`, or `.serena/**`, it
SHALL run only `scripts/guard.sh`, `scripts/check-openspec-archived.sh`,
`scripts/check-test-plan.py`, and `openspec validate --all --strict` and skip
format, lint, and tests. When the changed-path set is empty or contains any other
path, it SHALL run the full gate.

#### Scenario: Docs-only working diff

- **WHEN** `scripts/verify-fast.sh` runs and every changed path is `docs/**`, `openspec/**`, `*.md`, or `.serena/**`
- **THEN** it runs the guard, the archive check, the test plan, and OpenSpec validation and does not run fmt, clippy, or tests

#### Scenario: Clean tree or code change

- **WHEN** the working diff is empty or contains a path outside the fast-path set
- **THEN** `scripts/verify-fast.sh` runs the full gate

### Requirement: Opt-in serialization of local verification runs

The local verification scripts SHALL support an **opt-in** lock shared by all
worktrees of the invoking user on the machine, so concurrent runs queue instead
of overlapping their heavy CMake, cargo, Qt link, and test steps. The lock SHALL
be disabled unless enabled by `PICTURA_VERIFY_LOCK=1`. Both
`PICTURA_VERIFY_LOCK` and `PICTURA_VERIFY_LOCK_TIMEOUT` SHALL be resolved from
the process environment first and, only when a variable is absent there, from
the user-global env file
`${KOOKA_ENV_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/kooka-pictura/env}` read as
data. A variable present in the process environment SHALL win even when its
value is `0` or empty. No repository-tracked file SHALL be read by default;
`KOOKA_ENV_FILE` SHALL be taken only from the process environment.

When enabled, the lock SHALL use an atomic, portable primitive that works on
local filesystems on Linux, macOS, and git-bash without requiring `flock`,
`/proc`, GNU `stat`, or Linux-only sockets. It SHALL be scoped per user and
SHALL create its directory with owner-only permissions. Before reading or
reclaiming a pre-existing lock directory, the scripts SHALL verify that the
invoking user owns it and that it is not a symlink; otherwise they SHALL fail
with a clear message. The lock SHALL record the owner PID, worktree, branch (or
short commit when detached), script, and start time.

A run that cannot take the lock SHALL wait, report the current holder and its
worktree, and proceed once the holder releases it. While one run holds the lock,
a nested invocation of the other verification script SHALL recognize that it is
already inside the critical section, SHALL NOT acquire the lock again, and SHALL
NOT release a lock it did not acquire. `scripts/verify-full.sh` SHALL acquire the
lock before its first heavy build step and hold it through its whole run,
including the nested `scripts/verify-fast.sh` and the live control check.
`scripts/verify-fast.sh` SHALL acquire the lock only after its docs-only early
exit, so a docs-only `verify-fast` run SHALL NOT create, acquire, or wait on the
lock; `scripts/verify-full.sh` always builds and therefore always locks when
enabled.

The wait SHALL be bounded by `PICTURA_VERIFY_LOCK_TIMEOUT` in seconds (default
`3600`; `0` means no limit). A value that is neither `0` nor a positive decimal
integer SHALL be rejected with a clear error and a non-zero exit. On expiry the
run SHALL exit non-zero, name the holder, and leave no lock behind. A lock whose
recorded owner PID is not a live process, or whose owner record is absent after
a short grace period, SHALL be reclaimed instead of blocking indefinitely, and
the reclaim SHALL be race-safe so two waiters cannot remove a freshly created
lock. Releasing the lock SHALL happen on normal exit and on interrupt or
termination.

#### Scenario: The lock is disabled by default [bl_off]

- **WHEN** a verification script runs with `PICTURA_VERIFY_LOCK` absent from the
  process environment and the user-global env file, or set to `0`
- **THEN** no lock is created or acquired and the run behaves as before

#### Scenario: The process environment wins over the env file [bl_precedence]

- **WHEN** the user-global env file sets `PICTURA_VERIFY_LOCK=1` and the process
  environment sets it to `0`
- **THEN** the lock is disabled for that run

#### Scenario: Concurrent runs queue [bl_queue]

- **WHEN** the lock is enabled and a second run starts while another run holds it
- **THEN** the second run reports the holder and its worktree, waits, and
  proceeds only after the first releases the lock

#### Scenario: The nested full-to-fast invocation does not deadlock [bl_nested]

- **WHEN** `scripts/verify-full.sh` holds the lock and invokes
  `scripts/verify-fast.sh`
- **THEN** the nested script treats itself as already inside the critical
  section, does not acquire the lock again, and completes the full run

#### Scenario: The nested invocation does not release the parent's lock [bl_nested_release]

- **WHEN** a nested `scripts/verify-fast.sh` exits while the parent holds the lock
- **THEN** the lock directory still exists and a third run continues to queue

#### Scenario: A docs-only fast run stays lock-free [bl_docs_only]

- **WHEN** the lock is enabled and `scripts/verify-fast.sh` detects a docs-only
  working diff
- **THEN** it runs its docs-only fast path without creating, acquiring, or
  waiting on the lock

#### Scenario: The wait expires with the holder identified [bl_timeout]

- **WHEN** the lock remains held until `PICTURA_VERIFY_LOCK_TIMEOUT` elapses
- **THEN** the waiting run exits non-zero, names the holder, and leaves no lock
  behind

#### Scenario: The opt-in is read from the user-global env file [bl_envfile]

- **WHEN** `PICTURA_VERIFY_LOCK` is absent from the process environment and the
  user-global env file sets it to `1`
- **THEN** the scripts treat the lock as enabled

#### Scenario: The timeout is overridden [bl_timeout_override]

- **WHEN** `PICTURA_VERIFY_LOCK_TIMEOUT` is set to a positive number of seconds
- **THEN** the run waits at most that long instead of the 3600-second default,
  and `0` waits without a deadline

#### Scenario: An invalid timeout is rejected [bl_invalid_timeout]

- **WHEN** `PICTURA_VERIFY_LOCK_TIMEOUT` is not `0` or a positive decimal integer
- **THEN** the run reports the invalid value and exits non-zero instead of
  silently using a different bound

#### Scenario: A dead holder is reclaimed [bl_stale]

- **WHEN** the lock records an owner PID that is not a live process
- **THEN** the next run reclaims the lock and proceeds instead of blocking

#### Scenario: Reclaim is race-safe [bl_reclaim_race]

- **WHEN** two waiters find the same dead holder at the same time
- **THEN** exactly one reclaims it and the waiters do not both enter the
  critical section or remove a freshly created lock

#### Scenario: An owner-less lock directory is reclaimed [bl_empty_owner]

- **WHEN** a lock directory has no owner record
- **THEN** a waiter re-reads it after a short grace period and reclaims it if it
  is still empty, rather than blocking until the full timeout

#### Scenario: Interrupt or termination releases the lock [bl_signal]

- **WHEN** a lock holder receives `INT` or `TERM`
- **THEN** it removes the lock it owns before exiting

#### Scenario: A repository-tracked file cannot enable the lock [bl_notracked]

- **WHEN** a repository-local `.env` sets `PICTURA_VERIFY_LOCK=1` and no
  `KOOKA_ENV_FILE` override is present in the process environment
- **THEN** the lock stays disabled

#### Scenario: The opt-in is documented [bl_docs]

- **WHEN** a reader opens the worktree guidance in `DEVELOPING.md`
- **THEN** it states that serialization is opt-in and off by default, names the
  enabling and timeout variables, gives the default timeout, and explains that
  the lock covers CMake and cargo work across worktrees

### Requirement: A runnable build-lock check

The repository SHALL provide a check that proves the opt-in lock without
touching the real lock or the real user-global env file. It SHALL build an
isolated sandbox and test the lock helper directly, without invoking
`scripts/verify-fast.sh` or `scripts/verify-full.sh`, so the check cannot
recurse or run a build. It SHALL assert that: a second run waits for a held lock
and proceeds after release; a nested invocation neither acquires nor releases
the parent's lock; a docs-only fast run takes no lock; an expired wait fails
without leaving a lock; a dead owner is reclaimed race-safely; signal release
works; the process environment wins over the env file; and the disabled path
never locks. The check SHALL exit non-zero on any failed assertion and SHALL be
run by the full local fast gate. The repository SHALL NOT wire it into CI for
this change.

#### Scenario: The check passes on a correct lock [bl_check]

- **WHEN** the build-lock check runs against the committed scripts
- **THEN** every assertion holds and the check exits 0

#### Scenario: A broken lock fails the check [bl_check_fail]

- **WHEN** a change lets a nested invocation re-acquire or release the held lock,
  or skips a required wait
- **THEN** the check reports the failing assertion and exits non-zero

### Requirement: Placement-independent screen-relative self-tests

A headless self-test that measures a floating overlay against the screen SHALL
NOT depend on the main window's size or position, and SHALL NOT require a band
between the window and the screen edge, because the window's minimum size
exceeds the offscreen screen on every platform and such a band does not exist.
The check SHALL place the main window at a known position, move the overlay
past the screen's bottom-right corner, and pass only when the overlay comes
back clamped inside the screen's available geometry and resting on the
screen's bottom-right edge, which is what tells a screen-relative clamp apart
from a window-relative one.

#### Scenario: The float screen-bounds check ignores default window placement [vh_float_screen_bounds]

- **WHEN** the `float_screen_bounds` self-test check runs under the offscreen QPA
  from any initial frame position and with any frame size
- **THEN** it places the frame at the screen origin, moves the float past the
  screen's bottom-right corner, and passes only when the float is clamped
  inside the screen's available geometry and rests on the screen's bottom-right
  edge

### Requirement: Warning-free verification output

The local fast gate, the local full gate, and a clean CMake build SHALL emit no
warnings caused by project code or configuration. A warning caused by a
deliberate private dependency, generated bindings, or third-party headers SHALL
be suppressed narrowly at the boundary with a documented reason rather than by
filtering the verification log. Warnings emitted only by third-party packaging
tools outside those gates SHALL be documented; they need not be suppressed.

#### Scenario: A clean configure and build emit no project warnings [vh_warning_free_build]

- **WHEN** the project is configured and built from a clean build directory
- **THEN** CMake and the C++ compiler print no warning caused by project code or
  configuration

#### Scenario: The local verification gates emit no project warnings [vh_warning_free_gates]

- **WHEN** `scripts/verify-fast.sh` and `scripts/verify-full.sh` run
- **THEN** their output contains no warning caused by project code or
  configuration

### Requirement: The local full gate preserves the build tree

`scripts/verify-full.sh` SHALL NOT delete compiled objects from the real
`build/` tree. Its cache proof SHALL run against that real build directory,
remove at most one probe object owned by the target it rebuilds, and leave the
test objects, the shell archive or shared library, and the linked test
executables in place. The proof SHALL still fail when the compiler is not
running through the cache.

#### Scenario: test objects survive the full gate [vh_fast_gate_preserves]

- **WHEN** `scripts/verify-full.sh` runs after a completed build
- **THEN** no Qt Test object or test executable is removed, and the next build recompiles only what changed

#### Scenario: the cache proof still proves a hit [vh_cache_probe]

- **WHEN** the cache is warm and the full gate runs its rebuild proof
- **THEN** cache hits increase after the probe object is removed and rebuilt

#### Scenario: a broken cache wiring fails the proof [vh_cache_probe_fails]

- **WHEN** the compiler launcher is removed from the generated build rules and the rebuild proof runs
- **THEN** the proof fails instead of passing on an uncached rebuild

### Requirement: Independent fast-gate checks run concurrently

`scripts/verify-fast.sh` SHALL run its independent script checks concurrently
with each other, capture each check's output, and fail after all checks complete
when any exits non-zero. A check is independent when it does not compile
project code, write the workspace build directory, or depend on another check's
output. A check that rewrites a shared repository file SHALL NOT run alongside a
check that reads that file. The Cargo format, clippy, and test steps SHALL remain
sequential because they share the workspace build directory. When a check fails,
its captured output SHALL be shown.

#### Scenario: all checks pass [vh_parallel_checks]

- **WHEN** `scripts/verify-fast.sh` runs its full gate and every independent check passes
- **THEN** every check runs and the gate reports success once they all finish

#### Scenario: one check fails [vh_parallel_failure]

- **WHEN** one independent check exits non-zero while the others pass
- **THEN** the gate fails after all checks finish and prints the failing check's output

### Requirement: The local gates detect a stale build tree

The local verification gates SHALL detect a build tree that is older than the
sources it is asked to test. The full gate SHALL fail when the application
binary in the build tree predates a tracked source under the application
crate, or when a CTest-registered suite has no executable in the tree, and
SHALL name the file or the suite that proves the staleness. The fast gate
SHALL report the same staleness, because it runs the same tests against the
same tree.

#### Scenario: a stale tree fails the full gate [vh_stale_tree]

- **WHEN** the application binary in the build tree predates a tracked source under the application crate
- **THEN** the full gate fails and names the source that is newer than the binary

#### Scenario: a missing suite executable fails the full gate [vh_missing_suite_binary]

- **WHEN** a CTest-registered suite has no executable in the build tree
- **THEN** the full gate fails and names the suite
