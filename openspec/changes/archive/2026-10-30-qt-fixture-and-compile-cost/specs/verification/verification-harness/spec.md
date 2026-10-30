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
through CTest with a parallel level equal to the runner's CPU count, overridable
through `CTEST_PARALLEL_LEVEL`, SHALL configure the C++ build with precompiled
headers disabled, so a source that compiles only through a precompiled header
fails CI, and SHALL install mold so the linker path used locally is exercised
in CI. The `test` job
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
- **THEN** it passes the runner's CPU count as the parallel level unless `CTEST_PARALLEL_LEVEL` is set, and every registered suite still emits its own JUnit file

#### Scenario: CI builds without precompiled headers [vh_ci_no_pch]

- **WHEN** the CI headless job configures and builds the C++
- **THEN** precompiled headers are disabled in its configure and every source compiles without one

#### Scenario: CI exercises the configured linker [vh_ci_mold]

- **WHEN** the CI headless job installs the toolchain and configures the project
- **THEN** mold is available and the C++ links use it, so the local linker path is tested in CI
