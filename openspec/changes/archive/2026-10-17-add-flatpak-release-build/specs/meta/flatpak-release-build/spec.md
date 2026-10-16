# Spec Delta

## Purpose

Defines how the repository builds and publishes an x86_64 Flatpak bundle from a
release tag: the app built against the KDE runtime and sandboxed by a minimal
`finish-args` set, packaged as a single-file bundle attached to the GitHub
Release.

## ADDED Requirements

### Requirement: The Flatpak bundle is built for a release tag

The repository SHALL build and attach a single-file Flatpak bundle named
`KookaPictura-<version>-x86_64.flatpak` to each GitHub Release, where `<version>`
is the release tag without its leading `v`. The build SHALL start from the
release tag and SHALL NOT depend on artifacts produced by a `master` merge.

#### Scenario: A release is cut

- **WHEN** the release-please Release PR is merged and tag `vX.Y.Z` is created
- **THEN** the release gains `KookaPictura-X.Y.Z-x86_64.flatpak` as an asset

#### Scenario: The build is re-run for an existing tag

- **WHEN** the Flatpak workflow is dispatched by hand for tag `vX.Y.Z`
- **THEN** it rebuilds and re-uploads `KookaPictura-X.Y.Z-x86_64.flatpak` to the
  `vX.Y.Z` release, replacing any existing asset of the same name

### Requirement: The application builds offline against the KDE runtime

The Flatpak build SHALL build the C++ shell and the Rust static library against
`org.kde.Platform`/`org.kde.Sdk` version 6.11, and the build SHALL perform no
network I/O. Every build-time dependency SHALL be declared as a manifest source:
the Rust crates SHALL be resolved from a committed `cargo-sources.json` with
cargo in offline mode, and the `cxx-qt-cmake` and `corrosion` dependencies that
CMake would otherwise fetch SHALL be provided as vendored sources that
`FetchContent` is pointed at. The Rust toolchain SHALL come from the freedesktop
`rust-stable` SDK extension, which satisfies the workspace's minimum Rust
version.

#### Scenario: The application builds from manifest sources only

- **WHEN** the Flatpak build runs with no network access
- **THEN** it configures and compiles the Rust static library and the C++ shell
  from the declared sources and produces the `pictura` binary

#### Scenario: CMake fetches nothing over the network

- **WHEN** the build configures the project
- **THEN** `FetchContent` resolves both `CxxQt` and `Corrosion` from their
  vendored source directories rather than cloning `cxx-qt-cmake` or `corrosion`
  over the network

### Requirement: The app self-tests inside the Flatpak sandbox

The built application SHALL pass its headless self-test when run from the
Flatpak, so a released bundle is known to run under the sandbox rather than only
on the build host.

#### Scenario: The self-test passes from the bundle

- **WHEN** the built bundle is installed and run as
  `flatpak run io.github.KookaStudio.KookaPictura --headless --self-test`
- **THEN** it exits 0 and prints `pictura self-test: SUMMARY passed=<n> failed=0`

### Requirement: The bundle installs as a registered application

The bundle SHALL carry the application's desktop entry, icon, and AppStream
metainfo named after the application id
`io.github.KookaStudio.KookaPictura`, so that installing it registers the
application and `flatpak run io.github.KookaStudio.KookaPictura` resolves to the
`pictura` command. The bundle SHALL record the runtime repository that supplies
its `org.kde.Platform` runtime, so the runtime is resolved on install.

#### Scenario: The application is registered on install

- **WHEN** the bundle is installed on a machine with the Flathub remote configured
- **THEN** `io.github.KookaStudio.KookaPictura` is listed as an installed
  application and its desktop entry and icon are exported

#### Scenario: The bundle records its runtime repository

- **WHEN** the built bundle's metadata is inspected
- **THEN** it records the runtime repository that supplies
  `org.kde.Platform//6.11`, so installing the bundle can resolve the runtime

### Requirement: The Flatpak uses a minimal sandbox

The Flatpak SHALL grant only the sandbox permissions the application needs for
display, GPU, and printing: shared-memory IPC, Wayland with a fallback X11
socket, direct rendering, and the CUPS print socket. The sandbox already
provides writable per-app XDG config/cache/data directories, so the manifest
SHALL NOT request them, nor any other filesystem access, nor broad home or host
access.

#### Scenario: The declared permissions are minimal

- **WHEN** the manifest's `finish-args` are inspected
- **THEN** they contain `--share=ipc`, `--socket=wayland`,
  `--socket=fallback-x11`, `--device=dri`, and `--socket=cups`, and contain no
  `--filesystem` grant and no `--socket=session-bus` grant

### Requirement: The Flatpak workflow keys off the release and dry-runs on the Release PR

The Flatpak workflow SHALL be callable by `release-please.yml` when a release is
created and runnable by hand for an existing tag. The release-please Release PR
SHALL run the same build as a dry run that keeps the bundle as a workflow
artifact instead of uploading it to a release. A cheap manifest-lint check SHALL
run on pull requests that change the manifest; the full build SHALL NOT run on
any pull request other than the Release PR.

#### Scenario: release-please triggers the build

- **WHEN** `release-please.yml` detects a created release
- **THEN** it calls the Flatpak workflow with the release tag

#### Scenario: The Release PR dry-runs

- **WHEN** the release-please Release PR is opened or updated
- **THEN** the workflow builds the bundle, keeps it as a workflow artifact, and
  uploads nothing to a release

#### Scenario: A feature pull request lints but does not build

- **WHEN** a pull request whose head branch is not the release-please Release PR
  changes a Flatpak file
- **THEN** the manifest-lint job runs and the Flatpak build job is skipped

### Requirement: Flatpak packaging is documented

`DEVELOPING.md` SHALL document the Flatpak packaging flow: the artifact name and
its application id, the KDE 6.11 runtime and x86_64 architecture, how the
workflow is triggered (release, dispatch, Release-PR dry run, PR lint), how to
re-run it for an existing tag, that the bundle does not embed its runtime, and
that Flathub submission is not yet wired up.

#### Scenario: A maintainer reads the docs

- **WHEN** a maintainer reads the packaging section of `DEVELOPING.md`
- **THEN** they find the Flatpak artifact and application id, the runtime and
  architecture, the trigger paths including the Release-PR dry run and PR lint,
  the re-run command for an existing tag, and the no-embedded-runtime caveat
