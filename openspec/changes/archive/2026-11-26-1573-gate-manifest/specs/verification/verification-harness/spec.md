# Spec Delta

## ADDED Requirements

### Requirement: One checked-in gate manifest

The repository SHALL maintain one checked-in gate manifest with an entry for
every `scripts/check-*` script. Each entry SHALL name a unique check ID, scope,
cost class, execution phase, command, and local fast, local full, and guards CI
placement. An entry MAY also name docs-only or build-CI placement. An entry for
a check composed by another check SHALL name that script in the composing entry. Both
`scripts/verify-fast.sh` and the guards CI workflow SHALL select their commands
from this manifest rather than maintain separate check lists.

A check scheduled locally SHALL also be scheduled in guards CI or name one of
the existing build-CI equivalents. Manual tooling SHALL have no automatic local
or CI placement. Every guards CI entry SHALL be present in the local full run.
A check removed from local fast SHALL remain in local full and guards CI, and
local full SHALL require one explicit opt-in flag. The existing mandatory guards
set SHALL include icon provenance, shell seams, Flatpak cargo-source coverage,
and the build-lock check. Format, clippy, and test commands SHALL remain sequential;
independent local checks MAY run concurrently within the CPU budget.

The repository SHALL provide a drift check that fails for an unlisted
`scripts/check-*` script, a manifest path absent from the tree, a duplicate or
malformed entry, a missing mandatory CI entry, a guards CI entry absent from
local full, or an unscoped local-only entry. Its self-test SHALL plant each
failure class and prove that the check rejects it.

#### Scenario: A new check script has no entry [gm_unlisted]

- **WHEN** a `scripts/check-*` script exists and no manifest entry names it
- **THEN** the drift check exits non-zero and names the script

#### Scenario: A new check has one source of placement [gm_new_check]

- **WHEN** a contributor adds a check script and one valid manifest entry for it
- **THEN** both local full and guards CI select that entry without edits to either runner

#### Scenario: The mandatory CI set cannot shrink [gm_mandatory_ci]

- **WHEN** a manifest edit removes or disables an existing mandatory guards CI entry
- **THEN** the drift check exits non-zero and names the missing check ID

#### Scenario: Local-only placement is rejected [gm_no_local_only]

- **WHEN** an entry schedules a check locally without guards CI or a recognized build-CI equivalent
- **THEN** the drift check exits non-zero rather than allowing an accidental local-only gate

#### Scenario: Full-local mode is explicit [gm_full_opt_in]

- **WHEN** a contributor defers an expensive CI-owned check from local fast
- **THEN** local fast reports the deferral, local full and guards CI still run it, and only the explicit full-local flag enables it locally

#### Scenario: Runner parity is checked [gm_runner_parity]

- **WHEN** the manifest selects a guards CI check that is absent from local full
- **THEN** the drift check exits non-zero and names the check ID

#### Scenario: Guards CI runs the build-lock check [gm_build_lock_ci]

- **WHEN** the guards workflow selects its commands
- **THEN** the build-lock check is mandatory and a non-zero exit fails the workflow

## MODIFIED Requirements

### Requirement: Docs-only changes run guards, not builds

The build jobs (format/lint/test and Qt headless) SHALL be skipped when
every changed path matches `docs/**`, `openspec/**`, `**/*.md`, or `.serena/**`,
using workflow-level `paths-ignore`. A separate workflow with no path filter
SHALL select its mandatory checks from the checked-in gate manifest on every
push and pull request, so a docs-only change is still gated. That set SHALL
include `scripts/guard.sh`, `openspec validate --all --strict`,
`scripts/check-file-size.sh`, `scripts/check-selftest-budget.sh`,
`scripts/check-test-plan.py`, `scripts/check-worktree-hook.sh`,
`scripts/check-selftest-codes.py`, `scripts/check-openspec-archived.sh`,
`scripts/check-icon-provenance.py`, `scripts/check-shell-seams.py`, and
`scripts/check-flatpak-cargo-sources.py`, together with the other existing
mandatory guards. A mixed change SHALL run the build jobs, because
`paths-ignore` skips a workflow only when all changed paths match.
`scripts/verify-fast.sh` SHALL select its docs-only checks from the manifest's
docs-only placement and SHALL skip format, lint, tests, and other checks not
placed there. Its full gate SHALL select the local-fast placement, or the
local-full placement when the explicit full-local flag is set.

#### Scenario: Docs-only change

- **WHEN** a push or pull request changes only `docs/**`, `openspec/**`, `*.md`, or `.serena/**`
- **THEN** the build jobs do not run and the guards workflow runs every check placed in guards CI by the manifest

#### Scenario: Code change

- **WHEN** a change touches any path outside the ignored set
- **THEN** the build jobs run as before

#### Scenario: Guards fail

- **WHEN** any command selected for guards CI exits non-zero
- **THEN** the guards workflow fails and identifies that check

#### Scenario: The fast full gate runs the worktree-hook check

- **WHEN** `scripts/verify-fast.sh` runs its full gate
- **THEN** it runs `scripts/check-worktree-hook.sh` and fails if the script fails

#### Scenario: The guards workflow runs the archive check

- **WHEN** the guards workflow runs on any push or pull request
- **THEN** it runs `scripts/check-openspec-archived.sh` and fails on a non-archived change directory

#### Scenario: The docs-only fast path still runs the self-test codes guard [vh_docs_only_guard]

- **WHEN** `scripts/verify-fast.sh` sees a diff of only `docs/**`, `openspec/**`, `*.md`, or `.serena/**` paths
- **THEN** it selects the manifest's docs-only checks, skips fmt, clippy, and tests, and fails on a self-test-code violation

### Requirement: Independent fast-gate checks run concurrently

`scripts/verify-fast.sh` SHALL select its independent script checks from the
checked-in gate manifest and run them concurrently with each other, bounded by
the local CPU budget when one is set, capture each check's output, and fail
after all checks complete when any exits non-zero. A check is independent when
it does not compile project code, write the workspace build directory, or
depend on another check's output. A check that rewrites a shared repository
file SHALL NOT run alongside a check that reads that file. The Cargo format,
clippy, build-tree, and test steps SHALL remain sequential because they share
the workspace build directory. When a check fails, its captured output SHALL be
shown. Guards CI SHALL execute its selected commands sequentially and stop on
the first failure.

#### Scenario: all checks pass [vh_parallel_checks]

- **WHEN** `scripts/verify-fast.sh` runs its full gate and every independent check passes
- **THEN** every selected check runs and the gate reports success once they all finish

#### Scenario: one check fails [vh_parallel_failure]

- **WHEN** one independent check exits non-zero while the others pass
- **THEN** the gate fails after all checks finish and prints the failing check's output

#### Scenario: CI reports the failing check [gm_ci_failure]

- **WHEN** a guards CI command exits non-zero
- **THEN** the workflow stops and identifies that command's manifest ID

### Requirement: Local verification fast path

`scripts/verify-fast.sh` SHALL detect the changed paths of the working tree
(committed diff against the merge base with `master`, plus untracked files).
When every changed path matches `docs/**`, `openspec/**`, `*.md`, or `.serena/**`,
it SHALL run only the commands selected by the manifest's docs-only placement
and skip format, lint, tests, and other full-gate checks. That placement SHALL
include `scripts/guard.sh`, `scripts/check-openspec-archived.sh`,
`scripts/check-test-plan.py`, `scripts/check-selftest-codes.py`, and
`openspec validate --all --strict`. When the changed-path set is empty or
contains any other path, it SHALL select the local-fast placement, or the
local-full placement when the explicit full-local flag is set.

#### Scenario: Docs-only working diff

- **WHEN** `scripts/verify-fast.sh` runs and every changed path is `docs/**`, `openspec/**`, `*.md`, or `.serena/**`
- **THEN** it runs the manifest's docs-only checks and does not run fmt, clippy, tests, or full-only checks

#### Scenario: Clean tree or code change

- **WHEN** the working diff is empty or contains a path outside the fast-path set
- **THEN** `scripts/verify-fast.sh` runs the selected local fast gate, or local full gate with the explicit flag
