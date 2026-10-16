# Spec Delta

## Purpose

Defines how the repository builds and publishes a macOS release artifact from a
release tag: an application bundle carrying the deployed Qt runtime, packed as
a DMG.

## ADDED Requirements

### Requirement: macOS artifact is built for a release tag

The repository SHALL build and attach a macOS artifact named
`KookaPictura-<version>-macOS.dmg` to each GitHub Release, where `<version>` is
the release tag without its leading `v`. The build SHALL start from the release
tag and SHALL NOT depend on artifacts produced by a `master` merge.

#### Scenario: A release is cut

- **WHEN** the release-please Release PR is merged and tag `vX.Y.Z` is created
- **THEN** the release gains `KookaPictura-X.Y.Z-macOS.dmg` as an asset

#### Scenario: The build is re-run for an existing tag

- **WHEN** the macOS workflow is dispatched by hand for tag `vX.Y.Z`
- **THEN** it rebuilds and re-uploads `KookaPictura-X.Y.Z-macOS.dmg` to the
  `vX.Y.Z` release, replacing any existing asset of the same name

### Requirement: The application builds and self-tests with the Apple toolchain

The application SHALL compile as a macOS application bundle with the Apple
toolchain (clang, arm64) and SHALL pass its headless self-test on macOS. The
build MUST NOT require an external Little CMS library to be present on macOS:
the vendored copy embedded through `lcms2-sys` SHALL be linked statically, as on
Windows.

#### Scenario: The application compiles on macOS

- **WHEN** the macOS build configures and compiles the C++ shell and the Rust
  static library
- **THEN** it completes without requiring a system Little CMS library, and
  produces `pictura.app`

#### Scenario: The self-test passes on macOS

- **WHEN** the deployed bundle's
  `pictura.app/Contents/MacOS/pictura --headless --self-test` is run
- **THEN** it exits 0 and prints `pictura self-test: SUMMARY passed=<n> failed=0`

### Requirement: The bundle is a deployable macOS application

The macOS build SHALL produce an application bundle whose `Info.plist` declares
the bundle identifier, display name, executable name, the project version, the
application icon, and the minimum macOS version. The bundle SHALL contain the
executable under `Contents/MacOS/`, and `macdeployqt` SHALL embed the Qt
frameworks and plugins the application uses, including the `platforms` plugins,
into `Contents/Frameworks/` and `Contents/PlugIns/`.

#### Scenario: The bundle layout is complete

- **WHEN** the bundle build and `macdeployqt` finish
- **THEN** `pictura.app/Contents/MacOS/pictura`,
  `pictura.app/Contents/Info.plist`, and
  `pictura.app/Contents/Frameworks/QtCore.framework` exist

#### Scenario: The offscreen platform plugin is deployed

- **WHEN** `macdeployqt` finishes
- **THEN** `pictura.app/Contents/PlugIns/platforms/` contains the offscreen
  platform plugin needed by `--headless`

### Requirement: Supported macOS versions and architecture

The macOS artifact SHALL target Apple Silicon (arm64) and SHALL run on macOS 13
or later, the floor of the bundled Qt 6.11. The bundle's `Info.plist` SHALL
declare `LSMinimumSystemVersion` 13.0. Intel (x86_64) and macOS 12 and earlier
are out of scope.

#### Scenario: The bundle declares its minimum OS

- **WHEN** the built bundle's `Info.plist` is inspected
- **THEN** `LSMinimumSystemVersion` is `13.0` and `CFBundleIdentifier` is set

#### Scenario: The DMG runs on a supported Mac

- **WHEN** the DMG is opened on an Apple Silicon Mac running macOS 13 or later
  and the application's `pictura --headless --self-test` is run
- **THEN** it exits 0 and prints `pictura self-test: SUMMARY passed=<n> failed=0`

### Requirement: The DMG packages the bundle

The macOS workflow SHALL pack the deployed bundle into
`KookaPictura-<version>-macOS.dmg` with `create-dmg`, including an Applications
drop link, and SHALL smoke-check the deployed bundle before packing it.

#### Scenario: The DMG contains the application

- **WHEN** the DMG is mounted
- **THEN** it contains the application bundle and a link to `/Applications`

#### Scenario: The smoke check runs before packing

- **WHEN** the workflow reaches the DMG step
- **THEN** the deployed bundle has already exited 0 from
  `--headless --self-test`

### Requirement: Signing and notarization are deferred

The macOS build MUST NOT require an Apple Developer certificate or notarization
credentials. It SHALL produce an unsigned bundle and DMG, and `DEVELOPING.md`
SHALL state that the artifacts are unsigned and how a user opens them past
Gatekeeper, until an Apple Developer account exists.

#### Scenario: The build runs without signing secrets

- **WHEN** the macOS workflow runs with no Apple signing secrets configured
- **THEN** it produces the DMG without attempting to sign or notarize it

### Requirement: The macOS workflow keys off the release and can dry-run

The macOS workflow SHALL be callable by `release-please.yml` when a release is
created and runnable by hand for an existing tag. The release-please Release PR
SHALL run the same build as a dry run that keeps the DMG as a workflow artifact
instead of uploading it to a release; feature pull requests SHALL NOT run it.

#### Scenario: release-please triggers the build

- **WHEN** `release-please.yml` detects a created release
- **THEN** it calls the macOS workflow with the release tag

#### Scenario: The Release PR dry-runs

- **WHEN** the release-please Release PR is updated
- **THEN** the workflow builds the DMG, keeps it as a workflow artifact, and
  uploads nothing to a release

### Requirement: macOS packaging is documented

`DEVELOPING.md` SHALL document the macOS packaging flow: the artifact name, the
supported macOS version and architecture, how the workflow is triggered
(release, dispatch, Release PR dry run), how to re-run it for an existing tag,
and that the artifacts are unsigned.

#### Scenario: A maintainer reads the docs

- **WHEN** a maintainer reads the packaging section of `DEVELOPING.md`
- **THEN** they find the macOS artifact, the supported macOS version and
  architecture, the trigger paths, the re-run command for an existing tag, and
  the unsigned-build caveat
