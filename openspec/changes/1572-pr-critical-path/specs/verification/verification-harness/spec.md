# Spec Delta

## MODIFIED Requirements

### Requirement: Jobs are skipped when the change cannot affect them
For a change that is not docs-only (the docs-only case is governed by
"Docs-only changes run guards, not builds"), CI SHALL evaluate the changed
paths and publish four boolean outputs: `rust`, `cpp`, `render`, and `docs`.
The outputs MAY overlap. Skipping SHALL happen per job inside a workflow that
runs on the event, so a skipped job reports a successful conclusion instead of
reporting nothing at all; a skipped job MUST NOT depend on a workflow-level
path filter, so that a check later made required can never leave a pull request
waiting forever for a report.

The classifier SHALL apply these rules:

- `docs` SHALL be true only when every changed path matches `docs/**`,
  `openspec/**`, `**/*.md`, or `.serena/**`.
- `cpp` SHALL be true for every non-docs change because the Rust staticlib is
  linked into the Qt application and the headless job exercises that build.
- `rust` SHALL be true unless the change is docs-only or every changed path is
  under `crates/pictura-app/cpp/`.
- `render` SHALL be true only for the render-input allowlist defined by
  `verification/gpu-parity-ci`.
- Any change under `.github/workflows/**` SHALL set `rust`, `cpp`, and `render`
  to true.
- An empty or malformed changed-path set SHALL set `rust`, `cpp`, and `render`
  to true and `docs` to false.

The first changed-file fetch attempt MAY be allowed to fail without failing the
classifier job. The fetch SHALL be retried once after that failure. If the retry
succeeds, the classifier job SHALL recover. If both attempts fail, or the
classifier cannot parse the successful result, the classifier SHALL publish the
run-all outputs above, report the error, and exit non-zero.
`lint` and `test` SHALL gate on `rust`, `qt-headless` SHALL gate on `cpp`, and
`gpu-parity` and `windows-render` SHALL gate on `render`. Each condition SHALL
use `always()` and SHALL skip only when its output is exactly `false`, so a
failed classifier or missing output cannot suppress required coverage.

#### Scenario: C++-only change
- **WHEN** a pull request changes only files under `crates/pictura-app/cpp/`
- **THEN** `cpp` is true, `rust` and `render` are false, `qt-headless` runs, and `lint` and `test` are skipped

#### Scenario: Rust change runs required jobs [ci_rust_bucket]
- **WHEN** a pull request changes any file under `crates/` outside `crates/pictura-app/cpp/`
- **THEN** `rust` and `cpp` are true, and `lint`, `test`, and `qt-headless` run

#### Scenario: Mixed change runs every affected job [ci_mixed_buckets]
- **WHEN** a pull request changes both C++ sources and a render input
- **THEN** `rust`, `cpp`, and `render` are true, and every build job runs

#### Scenario: Unrecognized path runs required jobs [ci_unknown_path]
- **WHEN** a pull request changes a path not recognized by the classifier
- **THEN** `rust` and `cpp` are true, and the Qt application and Rust coverage are not skipped

#### Scenario: Workflow edit re-runs the suite [ci_workflow_edit]
- **WHEN** a pull request changes `.github/workflows/ci.yml`
- **THEN** `rust`, `cpp`, and `render` are true and every build job runs

#### Scenario: A skipped job still reports
- **WHEN** the gating condition skips a build job on a pull request
- **THEN** that job's check run reports a successful conclusion rather than never reporting, so it cannot block a required check

#### Scenario: A transient fetch failure is retried [ci_fetch_retry]
- **WHEN** the first changed-file fetch fails and the second succeeds
- **THEN** classification uses the second result, the classifier job succeeds, and the retry is the only extra fetch attempt

#### Scenario: An exhausted fetch failure runs coverage and fails the classifier [ci_fetch_fail_closed]
- **WHEN** both changed-file fetch attempts fail
- **THEN** the classifier publishes `rust=true`, `cpp=true`, `render=true`, and `docs=false`, exits non-zero, and the gated build jobs still run

#### Scenario: Missing classifier outputs cannot skip required jobs [ci_missing_outputs]
- **WHEN** the classifier job fails before publishing its outputs
- **THEN** each gated job treats its missing output as not false and runs

## ADDED Requirements

### Requirement: Critical-path reductions preserve settled CI coverage
CI SHALL keep format and Clippy in a `lint` job separate from the `test` job.
The `test` job SHALL keep the standalone `cargo nextest run -p pictura-render`
step in addition to `cargo nextest run --workspace`. Packaging dry runs SHALL
remain limited to the Release PR.

#### Scenario: Lint and test stay parallel [ci_separate_jobs]
- **WHEN** a change sets the `rust` bucket
- **THEN** `lint` and `test` start as separate jobs without a dependency between them

#### Scenario: Renderer tests keep their dedicated run [ci_render_nextest]
- **WHEN** the `test` job runs
- **THEN** it runs `cargo nextest run -p pictura-render` before the workspace nextest run

#### Scenario: Packaging dry runs stay Release-only [ci_release_packaging]
- **WHEN** a feature pull request changes packaging inputs
- **THEN** packaging dry runs do not run unless the pull request is the Release PR
