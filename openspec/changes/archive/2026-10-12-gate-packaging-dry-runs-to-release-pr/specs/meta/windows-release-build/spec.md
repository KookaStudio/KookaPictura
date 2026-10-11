# Spec Delta

## Purpose

Defines how the repository builds and publishes Windows release artifacts from a
release tag: a portable archive and an installable executable, both carrying the
deployed Qt runtime and the application binary.

## REMOVED Requirements

### Requirement: The Windows workflow keys off the release and can dry-run

**Reason**: The dry run is now gated to the release-please Release PR instead
of any pull request that touches the Windows build, so the scenario "A
packaging pull request dry-runs" no longer describes when the workflow runs.
The requirement is replaced, not amended, because that scenario cannot be
carried forward.

**Migration**: Read "The Windows workflow keys off the release and dry-runs on
the Release PR" below. It keeps the `release-please.yml` `workflow_call`, the
manual `workflow_dispatch` re-run for an existing tag, and the dry run's
keep-artifacts-instead-of-uploading behavior; only the pull-request trigger is
narrowed to the Release PR head branch.

## ADDED Requirements

### Requirement: The Windows workflow keys off the release and dry-runs on the Release PR

The Windows workflow SHALL be callable by `release-please.yml` when a release is
created and runnable by hand for an existing tag. The release-please Release PR
SHALL run the same build as a dry run that keeps the artifacts as workflow
artifacts instead of uploading them to a release. No other pull request SHALL
run the workflow: outside `pull_request` events targeting the Release PR head
branch, the job SHALL be skipped.

#### Scenario: release-please triggers the build

- **WHEN** `release-please.yml` detects a created release
- **THEN** it calls the Windows workflow with the release tag

#### Scenario: The Release PR dry-runs

- **WHEN** the release-please Release PR is opened or updated
- **THEN** the workflow builds the portable archive and installer, keeps them
  as workflow artifacts, and uploads nothing to a release

#### Scenario: A feature pull request does not run it

- **WHEN** a pull request whose head branch is not the release-please Release
  PR is opened or updated
- **THEN** the Windows job is skipped and no Windows runner is started

## MODIFIED Requirements

### Requirement: Windows packaging is documented

`DEVELOPING.md` SHALL document the Windows packaging flow: the two artifacts,
the supported Windows versions (64-bit Windows 10 1809+ and Windows 11), how the
workflow is triggered (release, dispatch, Release-PR dry run), and how to
re-run it for an existing tag.

#### Scenario: A maintainer reads the docs

- **WHEN** a maintainer reads the packaging section of `DEVELOPING.md`
- **THEN** they find the Windows artifacts, the supported Windows versions, the
  Release-PR dry-run trigger, and the re-run command for an existing tag