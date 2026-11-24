# Spec Delta

## MODIFIED Requirements

### Requirement: Docs-only changes run guards, not builds

The build jobs (format/lint/test and Qt headless) SHALL be skipped when
every changed path matches `docs/**`, `openspec/**`, `**/*.md`, or `.serena/**`,
using workflow-level `paths-ignore`. A separate workflow with no path filter
SHALL run `scripts/guard.sh`, `openspec validate --all --strict`,
`scripts/check-file-size.sh`, `scripts/check-selftest-budget.sh`,
`scripts/check-test-plan.py`, `scripts/check-worktree-hook.sh`,
`scripts/check-selftest-codes.py`, and
`scripts/check-openspec-archived.sh` on
every push and pull request, so a docs-only change is still gated. A mixed change
SHALL run the build jobs, because `paths-ignore` skips a workflow only when all
changed paths match. `scripts/verify-fast.sh` SHALL run
`scripts/check-worktree-hook.sh`, `scripts/check-selftest-codes.py`, and
`scripts/check-openspec-archived.sh` in its
full gate alongside the other script checks, and SHALL also run
`scripts/check-selftest-codes.py` on the path that handles a docs-only diff, so a
change that touches only markdown, specs, or `docs/` cannot land a self-test
invariant regression unnoticed.

#### Scenario: Docs-only change

- **WHEN** a push or pull request changes only `docs/**`, `openspec/**`, `*.md`, or `.serena/**`
- **THEN** the build jobs do not run and the guards workflow runs the guard, OpenSpec validation, file-size, self-test budget, self-test codes, worktree-hook, and archive checks

#### Scenario: Code change

- **WHEN** a change touches any path outside the ignored set
- **THEN** the build jobs run as before

#### Scenario: Guards fail

- **WHEN** `scripts/guard.sh`, `openspec validate --all --strict`, `scripts/check-file-size.sh`, `scripts/check-selftest-budget.sh`, `scripts/check-test-plan.py`, `scripts/check-worktree-hook.sh`, `scripts/check-selftest-codes.py`, or `scripts/check-openspec-archived.sh` exits non-zero
- **THEN** the guards workflow fails

#### Scenario: The fast full gate runs the worktree-hook check

- **WHEN** `scripts/verify-fast.sh` runs its full gate
- **THEN** it runs `scripts/check-worktree-hook.sh` and fails if the script fails

#### Scenario: The guards workflow runs the archive check

- **WHEN** the guards workflow runs on any push or pull request
- **THEN** it runs `scripts/check-openspec-archived.sh` and fails on a non-archived change directory

#### Scenario: The docs-only fast path still runs the self-test codes guard [vh_docs_only_guard]

- **WHEN** `scripts/verify-fast.sh` sees a diff of only `docs/**`, `openspec/**`, `*.md`, or `.serena/**` paths
- **THEN** it skips fmt, clippy, and the tests but still runs `scripts/check-selftest-codes.py` and fails the gate on a violation
