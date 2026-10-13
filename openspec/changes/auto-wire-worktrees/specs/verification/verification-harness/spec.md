# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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