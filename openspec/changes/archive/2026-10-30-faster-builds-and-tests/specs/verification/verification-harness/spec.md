## MODIFIED Requirements

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
