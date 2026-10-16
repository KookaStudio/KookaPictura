# Spec Delta

## REMOVED Requirements

### Requirement: Local verification fast path and compiler caching

**Reason**: Split in two. The compiler-caching clause is now owned by the CMake
default documented and specified in `meta/local-build-caching`; `verify-full.sh`
no longer passes a compiler-launcher flag, so keeping that behaviour in this
requirement would leave it stale. The fast-path contract is re-added below.

**Migration**: Behaviour is unchanged for callers of `scripts/verify-fast.sh`.
Compiler-launcher behaviour now lives in `meta/local-build-caching`.

## ADDED Requirements

### Requirement: Local verification fast path

`scripts/verify-fast.sh` SHALL detect the changed paths of the working tree
(committed diff against the merge base with `master`, plus untracked files). When
every changed path matches `docs/**`, `openspec/**`, `*.md`, or `.serena/**`, it
SHALL run only `scripts/guard.sh` and `openspec validate --all --strict` and skip
format, lint, and tests. When the changed-path set is empty or contains any other
path, it SHALL run the full gate.

#### Scenario: Docs-only working diff

- **WHEN** `scripts/verify-fast.sh` runs and every changed path is `docs/**`, `openspec/**`, `*.md`, or `.serena/**`
- **THEN** it runs the guard and OpenSpec validation and does not run fmt, clippy, or tests

#### Scenario: Clean tree or code change

- **WHEN** the working diff is empty or contains a path outside the fast-path set
- **THEN** `scripts/verify-fast.sh` runs the full gate
