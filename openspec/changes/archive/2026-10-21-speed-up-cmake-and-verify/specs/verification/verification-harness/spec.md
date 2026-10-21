# Spec Delta

## MODIFIED Requirements

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
run its `--headless --self-test` mode there. That job SHALL run the Qt Test layer
through CTest with the same bounded parallel level as the local report, and SHALL
install mold so the linker path used locally is exercised in CI. CI SHALL run a
separate job that
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

#### Scenario: CI runs Qt Test suites concurrently [vh_ci_ctest_parallel]

- **WHEN** the CI headless job runs CTest after the build
- **THEN** it passes a bounded parallel level and every registered suite still emits its own JUnit file

#### Scenario: CI exercises the configured linker [vh_ci_mold]

- **WHEN** the CI headless job installs the toolchain and configures the project
- **THEN** mold is available and the C++ links use it, so the local linker path is tested in CI

## ADDED Requirements

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
