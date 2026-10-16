# Spec Delta

## MODIFIED Requirements

### Requirement: Docs-only changes run guards, not builds

The build jobs (format/lint/test, Qt headless, and oracles) SHALL be skipped when
every changed path matches `docs/**`, `openspec/**`, `**/*.md`, or `.serena/**`,
using workflow-level `paths-ignore`. A separate workflow with no path filter
SHALL run `scripts/guard.sh`, `openspec validate --all --strict`,
`scripts/check-file-size.sh`, `scripts/check-selftest-budget.sh`,
`scripts/check-worktree-hook.sh`, and `scripts/check-openspec-archived.sh` on
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

- **WHEN** `scripts/guard.sh`, `openspec validate --all --strict`, `scripts/check-file-size.sh`, `scripts/check-selftest-budget.sh`, `scripts/check-worktree-hook.sh`, or `scripts/check-openspec-archived.sh` exits non-zero
- **THEN** the guards workflow fails

#### Scenario: The fast full gate runs the worktree-hook check

- **WHEN** `scripts/verify-fast.sh` runs its full gate
- **THEN** it runs `scripts/check-worktree-hook.sh` and fails if the script fails

#### Scenario: The guards workflow runs the archive check

- **WHEN** the guards workflow runs on any push or pull request
- **THEN** it runs `scripts/check-openspec-archived.sh` and fails on a non-archived change directory

### Requirement: Local verification fast path and compiler caching

`scripts/verify-fast.sh` SHALL detect the changed paths of the working tree
(committed diff against the merge base with `master`, plus untracked files). When
every changed path matches `docs/**`, `openspec/**`, `*.md`, or `.serena/**`, it
SHALL run only `scripts/guard.sh`, `scripts/check-openspec-archived.sh`, and
`openspec validate --all --strict` and skip format, lint, and tests. When the
changed-path set is empty or contains any other path, it SHALL run the full gate.
`scripts/verify-full.sh` SHALL pass
`-DCMAKE_CXX_COMPILER_LAUNCHER=sccache` to the CMake configure only when
`sccache` is available, and `DEVELOPING.md` SHALL document opting into
`RUSTC_WRAPPER=sccache` and cleaning `target/`.

#### Scenario: Docs-only working diff

- **WHEN** `scripts/verify-fast.sh` runs and every changed path is `docs/**`, `openspec/**`, `*.md`, or `.serena/**`
- **THEN** it runs the guard, the archive check, and OpenSpec validation and does not run fmt, clippy, or tests

#### Scenario: Clean tree or code change

- **WHEN** the working diff is empty or contains a path outside the fast-path set
- **THEN** `scripts/verify-fast.sh` runs the full gate

#### Scenario: sccache present

- **WHEN** `scripts/verify-full.sh` runs on a machine where `sccache` is on `PATH`
- **THEN** the CMake configure receives `-DCMAKE_CXX_COMPILER_LAUNCHER=sccache`

#### Scenario: sccache absent

- **WHEN** `sccache` is not on `PATH`
- **THEN** `scripts/verify-full.sh` configures CMake without a compiler launcher and still builds
