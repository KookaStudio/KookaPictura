# Spec Delta

## Purpose

Defines how the repository builds and publishes Windows release artifacts from a
release tag: a portable archive and an installable executable, both carrying the
deployed Qt runtime and the application binary.

## ADDED Requirements

### Requirement: Windows artifacts are built for a release tag

The repository SHALL build and attach two Windows x86_64 artifacts to each
GitHub Release: a portable archive named
`KookaPictura-<version>-windows-x64-portable.zip` and an installer named
`KookaPictura-<version>-windows-x64-setup.exe`, where `<version>` is the release
tag without its leading `v`. The build SHALL start from the release tag and
SHALL NOT depend on artifacts produced by a `master` merge.

#### Scenario: A release is cut

- **WHEN** the release-please Release PR is merged and tag `vX.Y.Z` is created
- **THEN** the release gains `KookaPictura-X.Y.Z-windows-x64-portable.zip` and
  `KookaPictura-X.Y.Z-windows-x64-setup.exe` as assets

#### Scenario: The build is re-run for an existing tag

- **WHEN** the Windows workflow is dispatched by hand for tag `vX.Y.Z`
- **THEN** it rebuilds and re-uploads both assets to the `vX.Y.Z` release,
  replacing any existing assets of the same name

### Requirement: The application builds and self-tests under MSVC

The application SHALL compile with the MSVC x86_64 toolchain and SHALL pass its
headless self-test on Windows. Platform-specific facilities the code relies on
(such as the control server's socket path and its mode check) SHALL degrade to a
Windows-appropriate behavior rather than failing the build or the self-test. The
build MUST NOT require an external Little CMS library to be present on Windows.

#### Scenario: The application compiles on Windows

- **WHEN** the Windows build configures and compiles the C++ shell and the Rust
  static library
- **THEN** it completes without POSIX-only headers or functions failing to
  compile, and the link needs no system Little CMS library

#### Scenario: The self-test passes on Windows

- **WHEN** the built `pictura.exe --headless --self-test` is run on Windows
- **THEN** it exits 0 and prints `pictura self-test: SUMMARY passed=<n> failed=0`

### Requirement: The Windows build deploys Qt

The Windows build SHALL gather the Qt and runtime dependencies into the staging
folder with `windeployqt` in release mode, so the staging folder contains the
executable, the Qt libraries it uses, and the `platforms/qwindows.dll` platform
plugin.

#### Scenario: The staging folder is complete

- **WHEN** the bundle action finishes
- **THEN** the staging folder contains `pictura.exe`, the required `Qt6*.dll`
  libraries, and `platforms\qwindows.dll`

#### Scenario: The executable is a GUI binary

- **WHEN** the built executable is inspected
- **THEN** it is a Windows GUI-subsystem binary (no console window) carrying the
  application icon

### Requirement: Supported Windows versions

The Windows artifacts SHALL run on 64-bit Windows 10 (version 1809 / build
17763 or later) and 64-bit Windows 11. The built executable SHALL carry an
application manifest that declares support for Windows 10 and Windows 11, so the
operating system does not apply legacy compatibility behavior. Windows 7 and
Windows 8.1 are out of scope.

#### Scenario: The manifest declares Windows 10 and 11 support

- **WHEN** the built executable's embedded application manifest is inspected
- **THEN** it contains the `supportedOS` declarations for Windows 10 and
  Windows 11 (`{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}`)

#### Scenario: The artifacts run on Windows 10

- **WHEN** the portable archive or the installed application is run on a clean
  64-bit Windows 10 (1809 or later) machine
- **THEN** `pictura.exe --headless --self-test` exits 0 with
  `SUMMARY ... failed=0`

#### Scenario: The artifacts run on Windows 11

- **WHEN** the portable archive or the installed application is run on a clean
  64-bit Windows 11 machine
- **THEN** `pictura.exe --headless --self-test` exits 0 with
  `SUMMARY ... failed=0`

### Requirement: The portable archive runs self-contained

The portable archive SHALL run on a Windows machine with neither Qt nor the
Visual C++ redistributable preinstalled, so the packaged executable passes the
application self-test without the user installing anything first.

#### Scenario: The archive runs on a clean machine

- **WHEN** `KookaPictura-<version>-windows-x64-portable.zip` is extracted on a
  clean Windows machine and `pictura.exe --headless --self-test` is run
- **THEN** the process exits 0 and prints
  `pictura self-test: SUMMARY passed=<n> failed=0`

### Requirement: The installer installs the deployed application

The installer SHALL install the deployed application tree, create a Start-menu
entry, register an uninstaller, and ship an executable that passes the
application self-test.

#### Scenario: A silent install succeeds

- **WHEN** the installer is run unattended on a clean Windows machine
- **THEN** the application is installed, a Start-menu entry and an uninstall
  entry exist, and the installed `pictura.exe --headless --self-test` exits 0

#### Scenario: Uninstall removes the application

- **WHEN** the registered uninstaller is run
- **THEN** the installed files and the uninstall entry are removed

### Requirement: The Windows workflow keys off the release and can dry-run

The Windows workflow SHALL be callable by `release-please.yml` when a release is
created and runnable by hand for an existing tag. A pull request that touches
the Windows build, its packaging, or its workflow SHALL run the same build as a
dry run that keeps the artifacts as workflow artifacts instead of uploading them
to a release.

#### Scenario: release-please triggers the build

- **WHEN** `release-please.yml` detects a created release
- **THEN** it calls the Windows workflow with the release tag

#### Scenario: A packaging pull request dry-runs

- **WHEN** a pull request changes the Windows workflow, its bundle action, the
  Windows packaging files, or the build files the Windows job depends on
- **THEN** the workflow builds the portable archive and installer, keeps them as
  workflow artifacts, and uploads nothing to a release

### Requirement: Windows packaging is documented

`DEVELOPING.md` SHALL document the Windows packaging flow: the two artifacts,
the supported Windows versions (64-bit Windows 10 1809+ and Windows 11), how the
workflow is triggered (release, dispatch, pull-request dry run), and how to
re-run it for an existing tag.

#### Scenario: A maintainer reads the docs

- **WHEN** a maintainer reads the packaging section of `DEVELOPING.md`
- **THEN** they find the Windows artifacts, the supported Windows versions, the
  trigger paths, and the re-run command for an existing tag
