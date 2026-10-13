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
workspace under `cargo nextest run --workspace` and SHALL run doctests
explicitly with `cargo test --workspace --doc`; because that suite includes the
Qt application, the job SHALL install Qt. `scripts/verify-fast.sh` SHALL run the
same nextest invocation plus the explicit doctest run, so the local fast gate and
CI agree on the runner. The nextest run SHALL emit a JUnit XML artifact through
the repository's `.config/nextest.toml`, and CI SHALL upload that artifact. CI
SHALL build the Qt application under a pinned Qt 6 in a headless job and SHALL
run its `--headless --self-test` mode there. CI SHALL run a separate job that
installs the external comparison tools (ImageMagick and `psd-tools`) and runs the
workspace suite excluding the Qt application (`cargo test --workspace --exclude
pictura_app`), so that job neither installs Qt nor rebuilds the application, and
SHALL cache Rust registry sources and compiled artifacts between runs. The
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
- **THEN** the workspace suite runs under `cargo nextest run --workspace` and doctests run explicitly

#### Scenario: Lint and test run in parallel
- **WHEN** the workflow starts
- **THEN** format and clippy run in the `lint` job while the test suite runs in the `test` job, independent of each other

#### Scenario: The local fast gate uses nextest
- **WHEN** `scripts/verify-fast.sh` runs the test step
- **THEN** it runs `cargo nextest run --workspace --no-fail-fast` followed by `cargo test --workspace --doc`

#### Scenario: The JUnit artifact is uploaded
- **WHEN** the workflow test step completes
- **THEN** `target/nextest/default/junit.xml` has been written and is uploaded as an artifact

#### Scenario: The Qt app self-tests headlessly
- **WHEN** the workflow builds the Qt application with the pinned Qt 6 toolchain
- **THEN** it runs `--headless --self-test` with and without a document fixture and fails the job on a non-zero exit

#### Scenario: External oracles are exercised
- **WHEN** the oracle job runs with ImageMagick and `psd-tools` installed and `pictura_app` excluded
- **THEN** the tests that otherwise self-skip compare against the external tools instead of passing vacuously, without a Qt install

#### Scenario: Compiled artifacts are reused
- **WHEN** a later workflow run starts with a populated Rust registry cache and compiler cache
- **THEN** dependencies and previously compiled objects are restored rather than rebuilt from scratch

#### Scenario: A runaway job is bounded
- **WHEN** a CI job exceeds its configured `timeout-minutes`
- **THEN** the run is cancelled rather than consuming minutes indefinitely

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
The build jobs (format/lint/test, Qt headless, and oracles) SHALL be skipped when
every changed path matches `docs/**`, `openspec/**`, `**/*.md`, or `.serena/**`,
using workflow-level `paths-ignore`. A separate workflow with no path filter
SHALL run `scripts/guard.sh`, `openspec validate --all --strict`,
`scripts/check-file-size.sh`, `scripts/check-selftest-budget.sh`, and
`scripts/check-worktree-hook.sh` on every push and pull request, so a docs-only
change is still gated. A mixed change SHALL run the build jobs, because
`paths-ignore` skips a workflow only when all changed paths match.
`scripts/verify-fast.sh` SHALL run `scripts/check-worktree-hook.sh` in its full
gate alongside the other script checks.

#### Scenario: Docs-only change
- **WHEN** a push or pull request changes only `docs/**`, `openspec/**`, `*.md`, or `.serena/**`
- **THEN** the build jobs do not run and the guards workflow runs the guard, OpenSpec validation, file-size, self-test budget, and worktree-hook checks

#### Scenario: Code change
- **WHEN** a change touches any path outside the ignored set
- **THEN** the build jobs run as before

#### Scenario: Guards fail
- **WHEN** `scripts/guard.sh`, `openspec validate --all --strict`, `scripts/check-file-size.sh`, `scripts/check-selftest-budget.sh`, or `scripts/check-worktree-hook.sh` exits non-zero
- **THEN** the guards workflow fails

#### Scenario: The fast full gate runs the worktree-hook check
- **WHEN** `scripts/verify-fast.sh` runs its full gate
- **THEN** it runs `scripts/check-worktree-hook.sh` and fails if the script fails

### Requirement: Jobs are skipped when the change cannot affect them
For a change that is not docs-only (the docs-only case is governed by
"Docs-only changes run guards, not builds"), CI SHALL evaluate the changed
paths — the pull request diff, or the commits of a push — and SHALL skip each
build job (`lint`, `test`, `qt-headless`, `oracles`) whose inputs the change
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
  application and exercised by the test and oracle suites.
- A change limited to the hand-written C++ sources under
  `crates/pictura-app/cpp/` SHALL run `qt-headless` (and the guards workflow
  continues to run as before) and SHALL skip `lint`, `test`, and `oracles`,
  because the cargo build does not compile those translation units.
- The CI workflow file itself SHALL be treated as a path that causes every job
  to run, so editing the gating rules re-runs the full suite.

#### Scenario: C++-only change
- **WHEN** a pull request changes only files under `crates/pictura-app/cpp/`
- **THEN** `qt-headless` runs, `lint`, `test`, and `oracles` are skipped, and the guards workflow still runs

#### Scenario: Rust change runs everything
- **WHEN** a pull request changes any file under `crates/` outside `crates/pictura-app/cpp/`
- **THEN** `lint`, `test`, `qt-headless`, and `oracles` all run

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

### Requirement: Local verification fast path and compiler caching
`scripts/verify-fast.sh` SHALL detect the changed paths of the working tree
(committed diff against the merge base with `master`, plus untracked files). When
every changed path matches `docs/**`, `openspec/**`, `*.md`, or `.serena/**`, it
SHALL run only `scripts/guard.sh` and `openspec validate --all --strict` and skip
format, lint, and tests. When the changed-path set is empty or contains any other
path, it SHALL run the full gate. `scripts/verify-full.sh` SHALL pass
`-DCMAKE_CXX_COMPILER_LAUNCHER=sccache` to the CMake configure only when
`sccache` is available, and `DEVELOPING.md` SHALL document opting into
`RUSTC_WRAPPER=sccache` and cleaning `target/`.

#### Scenario: Docs-only working diff
- **WHEN** `scripts/verify-fast.sh` runs and every changed path is `docs/**`, `openspec/**`, `*.md`, or `.serena/**`
- **THEN** it runs the guard and OpenSpec validation and does not run fmt, clippy, or tests

#### Scenario: Clean tree or code change
- **WHEN** the working diff is empty or contains a path outside the fast-path set
- **THEN** `scripts/verify-fast.sh` runs the full gate

#### Scenario: sccache present
- **WHEN** `scripts/verify-full.sh` runs on a machine where `sccache` is on `PATH`
- **THEN** the CMake configure receives `-DCMAKE_CXX_COMPILER_LAUNCHER=sccache`

#### Scenario: sccache absent
- **WHEN** `sccache` is not on `PATH`
- **THEN** `scripts/verify-full.sh` configures CMake without a compiler launcher and still builds

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
