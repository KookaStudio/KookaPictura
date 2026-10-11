# Proposal

## Why

The release pipeline ships Linux artifacts only (AppImage, `.deb`, `.rpm`), so
Windows users have nothing to download even though `meta/release-please` names
Windows as a packaging target. Closing that gap (issue #198) also requires the
application itself to build and run its self-test on Windows: the control-server
and control self-test code currently use POSIX-only APIs, so a Windows build
fails before packaging is even reached.

## What Changes

- Add a Windows release build that produces two artifacts for a release tag:
  a **portable `.zip`** (the `windeployqt`-deployed folder) and an **NSIS
  installer `.exe`**, targeting 64-bit Windows 10 (version 1809 or later) and
  Windows 11, with the executable manifest declaring both.
- Make the application build and self-test on Windows:
  - guard the POSIX-only code in `crates/pictura-app/cpp/control_server.cpp`
    (`<unistd.h>`, `::getuid()`, `S_ISSOCK`) and in the control self-test
    (`crates/pictura-app/cpp/selftest_control.cpp`, which asserts a POSIX socket
    mode that does not exist for a Windows named pipe);
  - the `--headless --self-test` path must reach a passing summary on Windows.
- Add a composite action `.github/actions/windows-bundle` that configures a
  Ninja/Release CMake build with MSVC, installs the app into a staging directory,
  and gathers its Qt dependencies there with `windeployqt`.
- Add `.github/workflows/windows.yml`, modelled on `appimage.yml` /
  `linux-packages.yml`: it runs on release tags (`workflow_call` +
  `workflow_dispatch`), runs as an artifact-producing dry run on pull requests
  that touch the packaging, smoke-checks the deployed binary with
  `--headless --self-test`, and uploads the two artifacts to the release.
- Wire the new workflow into `release-please.yml`.
- Make the CMake build portable to Windows:
  - do not require an external Little CMS on Windows (`lcms2-sys` compiles its
    vendored copy into the Rust `staticlib` there);
  - mark the executable as a Windows GUI binary with an embedded icon and a
    manifest identifier, enabling the CMake `RC` language for the resource.
- Add `assets/icons/app.ico` (converted from the existing `app.png`) and record
  it in `assets/PROVENANCE.md`.
- Add `packaging/windows/` (a `.rc` resource and an NSIS `installer.nsi`) and a
  "Windows packages" section in `DEVELOPING.md`.
- No BREAKING changes; the Linux build, the Linux artifacts, and the
  application's Linux runtime behavior are unaffected.

## Capabilities

### New Capabilities

- `meta/windows-release-build`: the repository makes the application compile and
  self-test under MSVC and produces Windows release artifacts (portable zip and
  NSIS installer) from a release tag, deploying Qt with `windeployqt`, keyed off
  the release event like the other packaging workflows, and dry-runnable from a
  pull request.

### Modified Capabilities

<!-- None: the release-trigger contract in meta/release-please already covers
     packaging generically and its requirements do not change. The Windows
     control-server/self-test platform guard is a packaging-enabling change to
     code that has no dedicated behavior spec. -->

## Impact

- **App code**: `crates/pictura-app/cpp/control_server.cpp` and
  `crates/pictura-app/cpp/selftest_control.cpp` gain Windows branches; no Linux
  behavior change.
- **CI**: new `.github/actions/windows-bundle`, new `.github/workflows/windows.yml`,
  and a new job in `.github/workflows/release-please.yml`; one new action
  (`ilammy/msvc-dev-cmd`); `jurplel/install-qt-action` and
  `Swatinem/rust-cache` are already used.
- **Build**: `CMakeLists.txt` gains a Windows lcms2 link guard, `RC` language,
  `WIN32_EXECUTABLE`, a Windows manifest identifier (via target finalization),
  and an icon resource; no requirement changes on Linux.
- **Assets**: new `assets/icons/app.ico` plus a `PROVENANCE.md` entry.
- **Packaging**: new `packaging/windows/pictura.rc.in` (configured to
  `pictura.rc`), `packaging/windows/installer.nsi`.
- **Docs**: `DEVELOPING.md` gains a Windows packaging subsection.
- **Dependencies**: no new Rust or C++ dependencies; `lcms2-sys` already bundles
  its source as a fallback.
