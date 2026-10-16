# Spec Delta

## Purpose

Keeps the OpenSpec change backlog empty: every change is authored, implemented,
and archived in the pull request that ships it, enforced by a check over
`openspec/changes/`.

## ADDED Requirements

### Requirement: No non-archived change survives a pull request

The repository SHALL provide `scripts/check-openspec-archived.sh` that fails when
any directory exists directly under a changes directory other than `archive/`,
and exits zero when only `archive/` is present. It SHALL print each offending
path. When run with no argument it SHALL check `openspec/changes/`.

#### Scenario: An unarchived change fails the check

- **WHEN** a directory other than `archive/` exists under `openspec/changes/`
- **THEN** the check prints the offending path and exits non-zero

#### Scenario: An empty backlog passes the check

- **WHEN** the checked directory contains only `archive/`
- **THEN** the check exits zero

### Requirement: The check runs in CI and locally

The guards workflow SHALL run `scripts/check-openspec-archived.sh` on every push
and pull request, and `scripts/verify-fast.sh` SHALL run it in both its docs-only
path and its full gate, so a changed-path set that skips the build jobs still
gates the archive policy. The release pull request SHALL be gated the same way,
because the guards workflow is triggered on it by the release-please token.

#### Scenario: Guards run the check

- **WHEN** the guards workflow runs
- **THEN** it executes `scripts/check-openspec-archived.sh` and fails the workflow on a non-zero exit

#### Scenario: The fast gate runs the check on every path

- **WHEN** `scripts/verify-fast.sh` runs, whether the diff is docs-only or not
- **THEN** it runs `scripts/check-openspec-archived.sh` and fails if the script fails

#### Scenario: The release pull request is gated

- **WHEN** the release-please pull request runs the guards workflow
- **THEN** the archive check runs on it

### Requirement: The policy is documented

`DEVELOPING.md` and `AGENTS.md` SHALL state the zero-open policy: a change is
authored, implemented, and archived within the pull request that ships it, and a
non-archived change fails the archive check.

#### Scenario: A contributor finds the policy

- **WHEN** a contributor reads the OpenSpec workflow in `DEVELOPING.md` or `AGENTS.md`
- **THEN** it states that a change must be archived in the same pull request

### Requirement: The check proves itself

`scripts/check-openspec-archived.sh --self-test` SHALL build a throwaway sandbox
and assert that the check fails on a non-archived change directory and passes
when only `archive/` is present, exiting non-zero on any failed assertion.

#### Scenario: The self-test passes

- **WHEN** `scripts/check-openspec-archived.sh --self-test` runs
- **THEN** the fail case and the pass case both behave as specified and it exits zero

#### Scenario: A regression fails the self-test

- **WHEN** the check is changed so it no longer flags a non-archived directory
- **THEN** the self-test prints the failing assertion and exits non-zero
