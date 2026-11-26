# Proposal

## Why

CI and local verification duplicate Qt execution policy, and CI loses the stale-result and missing-report checks already implemented by `scripts/test-report.sh`. The Qt job also pays for two serial self-test runs after a full build, although `pictura` could link before the Qt Test build tail.

## What Changes

- Make `scripts/test-report.sh` the single Qt execution path for CI and local gates. It will own optional configure, staged builds, both self-test captures, stale JUnit cleanup, CTest, missing-report validation, and reporter invocation.
- Build `pictura` first. Keep the two self-test runs concurrent with each other. Add an opt-in overlap with the remaining Qt Test build tail for #1570 to measure, while default runs stay sequential until that measurement is positive.
- Start CTest only after the full Qt Test build and both self-test runs finish. Never run two Ninja writers against one build tree.
- Preserve separate self-test captures, token contracts, per-suite JUnit, named missing reports, and the first failing stage's exit status.
- Record Configure, `pictura` build, Qt Test build tail, each self-test run, CTest, and reporting durations.
- Keep Rust nextest as a separate stage and CI job. Replace the CI Qt steps with a thin adapter that still owns Qt installation, TIFF runtime setup, and CI CPU settings.
- Add a standard-library self-check that drives the real shell executor with fake build, self-test, and CTest commands.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `verification/test-reporting`: staged Qt execution, shared CI/local ordering, overlap gating, failure identity, cleanup, and timing.
- `verification/verification-harness`: the CI Qt job delegates execution to the shared script while retaining workflow-only setup.

## Impact

- `scripts/test-report.sh`, `scripts/verify-fast.sh`, and `scripts/verify-full.sh`.
- `.github/workflows/ci.yml`, with expected conflicts alongside #1570.
- A new `scripts/test_report_execution.py` self-check. `scripts/report_tests.py` and both C++ self-test token contracts remain unchanged.
- No dependencies, build-system changes, or Rust test-runner changes.
