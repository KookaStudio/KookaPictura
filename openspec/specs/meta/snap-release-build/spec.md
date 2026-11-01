# meta/snap-release-build Specification

## Purpose
Defines how the repository builds a strictly-confined snap of the application
from a release tag, attaches it to the GitHub Release, and dry-runs the build on
pull requests before it is gated to the Release PR.

## Requirements

### Requirement: Snap artifact is built for a release tag

The repository SHALL build a snap of the application for x86_64 from a release
tag and attach it to that GitHub Release as a release asset named
`kooka-pictura_<version>_<arch>.snap`, where `<version>` is the release tag
without its leading `v`. The build SHALL start from the release tag and SHALL
NOT depend on artifacts produced by a `master` merge.

#### Scenario: A release is cut

- **WHEN** the release-please Release PR is merged and tag `vX.Y.Z` is created
- **THEN** the release gains `kooka-pictura_X.Y.Z_amd64.snap` as an asset

#### Scenario: The build is re-run for an existing tag

- **WHEN** the snap workflow is dispatched by hand for tag `vX.Y.Z`
- **THEN** it rebuilds and re-uploads the `.snap` to the `vX.Y.Z` release,
  replacing any existing asset of the same name

### Requirement: The application builds and self-tests under the snap

The snap SHALL build the application from source inside snapcraft against the
`kde-neon-6` extension on base `core24`, so it links the Qt runtime the
extension provides, and SHALL use `strict` confinement. The packaged application
SHALL pass its headless self-test, and the snap SHALL declare the filesystem and
integration plugs the application needs beyond the desktop plugs the extension
supplies (`home`, `removable-media`, `cups`).

#### Scenario: The application compiles under the snap base

- **WHEN** the snap build configures and compiles the C++ shell and the Rust
  static library inside snapcraft
- **THEN** the build completes against the extension's Qt SDK and the
  resulting snap links the extension's Qt rather than a bundled copy

#### Scenario: The self-test passes under strict confinement

- **WHEN** the built `pictura --headless --self-test` is run from the installed
  snap
- **THEN** it exits 0 and prints `pictura self-test: SUMMARY passed=<n> failed=0`

#### Scenario: The snap declares its plugs

- **WHEN** the snap's declared plugs are inspected
- **THEN** they include `home`, `removable-media`, and `cups` alongside the
  desktop, GPU, and network plugs supplied by the extension

### Requirement: The snap workflow keys off the release and dry-runs on the Release PR

The snap workflow SHALL be callable by `release-please.yml` when a release is
created and runnable by hand for an existing tag. The release-please Release PR
SHALL run the same build as a dry run that keeps the `.snap` as a workflow
artifact instead of uploading it to a release. No other pull request SHALL run
the workflow.

#### Scenario: release-please triggers the build

- **WHEN** `release-please.yml` detects a created release
- **THEN** it calls the snap workflow with the release tag

#### Scenario: The Release PR dry-runs

- **WHEN** the release-please Release PR is opened or updated
- **THEN** the workflow builds the snap, keeps it as a workflow artifact, and
  uploads nothing to a release

#### Scenario: A feature pull request does not run it

- **WHEN** a pull request whose head branch is not the release-please Release PR
  is opened or updated
- **THEN** the snap job is skipped

### Requirement: Snap packaging is documented

`DEVELOPING.md` SHALL document the snap packaging flow: the artifact and its
confinement, the release-asset publish (and that the Snap Store is not yet
wired), how the workflow is triggered (release, dispatch, Release-PR dry run),
and how to re-run it for an existing tag.

#### Scenario: A maintainer reads the docs

- **WHEN** a maintainer reads the packaging section of `DEVELOPING.md`
- **THEN** they find the snap artifact and confinement, the release-asset
  publish and the Snap Store non-goal, the dry-run trigger, and the re-run
  command for an existing tag

### Requirement: The unsigned snap's install command reaches users

While the snap is published only as a release asset, `README.md`,
`public_docs/installing.md`, the site's Download panel, and each GitHub Release's
notes SHALL give the install command with the `--dangerous` flag. The Download
panel SHALL show the command only when the latest release has a snap asset, and
SHALL use that asset's filename. The snap workflow SHALL add the command to the
release notes without duplicating it when re-run for the same tag.

#### Scenario: A user reads the install documentation

- **WHEN** a user reads `README.md` or `public_docs/installing.md`
- **THEN** they find `sudo snap install --dangerous` with the snap's filename
  pattern

#### Scenario: The Download panel lists a snap

- **WHEN** the latest release has a `.snap` asset and a visitor opens the Linux
  download choices
- **THEN** the snap entry shows the install command with the asset's filename,
  and no other format shows a command

#### Scenario: The workflow runs for a tag

- **WHEN** the snap workflow uploads the `.snap` to a release
- **THEN** that release's notes contain the install command

#### Scenario: The workflow is re-run for the same tag

- **WHEN** the snap workflow runs for a tag whose notes already contain the
  install command
- **THEN** the notes are left unchanged

#### Scenario: The documented command installs the release snap

- **WHEN** a user runs the documented command on a release `.snap`
- **THEN** snapd installs it and the app's headless self-test reports `failed=0`
