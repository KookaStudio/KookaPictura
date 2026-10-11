# Design

## Context

See `proposal.md` for motivation. The constraints that shape the approach:

- The Linux release path is already factored as a composite action
  (`.github/actions/linux-bundle`) that a thin workflow (`appimage.yml`,
  `linux-packages.yml`) wraps: build → `cmake --install` to a staging tree →
  deploy runtime → pack a format. The Windows path mirrors that shape.
- Two app files are POSIX-only and are compiled into every build:
  `crates/pictura-app/cpp/control_server.cpp` (`<unistd.h>`, `::getuid()`,
  `S_ISSOCK`) and `crates/pictura-app/cpp/selftest_control.cpp` (a `stat()` mode
  check). The self-test runs the control checks unconditionally, so on Windows
  the build fails and then the self-test would fail even if it compiled.
- `CMakeLists.txt` links Little CMS unconditionally
  (`find_library(LCMS2_LIBRARY NAMES lcms2 REQUIRED)` + `target_link_libraries`).
  `pictura-color` depends on the `lcms2` crate, whose `lcms2-sys` dependency
  defaults to `dynamic` + `static-fallback`: when pkg-config finds no system
  `lcms2`, it compiles the vendored C into the Rust `staticlib`. There is no
  system `lcms2` on a Windows runner, so the `REQUIRED` find would fail even
  though the dependency is already satisfied.
- `crates/pictura-app/cpp/interop.cpp` includes the private header
  `<rhi/qrhi.h>` unconditionally, resolved through the hand-assembled include
  directory `${QT_HEADERS_DIR}/QtGui/${QT_VERSION_STR}/QtGui`.
- `pictura` is a plain `add_executable`; Qt's Windows manifest generation runs
  only from its target-finalization hook.
- `main.cpp` already selects the `offscreen` QPA plugin under `--headless`, so
  `pictura --headless --self-test` is a valid smoke check once the POSIX code is
  guarded.
- release-please calls the packaging workflows it wants from
  `release-please.yml` (`release_created == 'true'`).

## Goals / Non-Goals

**Goals:**
- Make the application compile and pass its headless self-test on Windows.
- Produce a portable archive and an NSIS installer for `windows` x86_64 from a
  release tag, with the same trigger surface as the Linux workflows.
- Keep the Linux build and artifacts unaffected.

**Non-Goals:**
- Code signing / notarization (no certificate; the issue marks this deferred).
- MSIX / Microsoft Store packaging.
- A GUI feature pass for Windows; this change only makes the existing app build,
  package, and pass its headless self-test there.

## Decisions

### Guard the POSIX-only control code

Add `#ifdef _WIN32` branches in `control_server.cpp` and `selftest_control.cpp`:

- The control socket path uses `::getuid()`; on Windows it should use a
  per-user, user-only location (e.g. under `QStandardPaths` or the temp dir)
  without `getuid`.
- `S_ISSOCK` and the `stat()`-based "user-only" mode check have no Windows
  analogue for a named pipe. On Windows the check is skipped and the condition
  is expressed as a platform guard rather than a POSIX mode test. Note the
  Windows endpoint is a machine-global named pipe with no per-user ACL (Qt's
  `UserAccessOption` is Unix-only), so this is a deliberate reduction in the
  control endpoint's isolation on Windows, not parity with the Linux mode check.

Linux behavior is unchanged. This is the root-cause fix: one guard in the shared
function, not a skip in every caller.

*Alternative considered:* exclude `control_server.cpp` from the Windows build
entirely and stub the control path. Rejected — it would fork the self-test and
lose coverage that already exists.

### Deploy Qt with an explicit `windeployqt` call, not the Qt CMake deploy API

Call `windeployqt --release <staging>\bin\pictura.exe` in the action after
`cmake --install`. The repo already deploys explicitly on Linux (linuxdeploy);
the Qt CMake API (`qt_generate_deploy_app_script`) would run at install time and
couple the CMake build to packaging.

### Windows lcms2 link guard

Keep the `target_link_libraries(pictura_shell PUBLIC pictura_app Qt6::Svg
Qt6::Network Qt6::PrintSupport ...)` call intact and append `"${LCMS2_LIBRARY}"`
only when it is found / not on Windows. The `find_library(... REQUIRED)` moves
inside `if(NOT WIN32)`, with a comment that `lcms2-sys`'s static fallback embeds
the vendored Little CMS into `pictura_app` on Windows.

*Risk if wrong:* if the static fallback is not absorbed into the staticlib, the
Windows link reports missing `cms*` symbols. `LCMS2_STATIC=1` is **not** a fix
for this (pkg-config already fails on Windows, so the static path already runs);
the real mitigation is to build and ship the Little CMS library and link it.
To be confirmed on the first dry run.

Also reconcile the existing comment at `CMakeLists.txt:380-381` ("a staticlib
does not propagate its native link dependencies"), which is true only for the
Linux dynamic pkg-config case.

### Enable the CMake `RC` language and anchor the icon path

Add `RC` to `project(... LANGUAGES CXX RC)` (or `enable_language(RC)`) so the
`.rc` resource compiles under Ninja/MSVC, and reference the icon as
`${CMAKE_CURRENT_SOURCE_DIR}/...` inside the `.rc` (RC compiles from the build
directory). The version block in the `.rc` is generated from `PROJECT_VERSION`,
not hardcoded, so it tracks the `x-release-please-version` marker.

### Finalize the target so the Windows manifest applies

`pictura` is a plain `add_executable`, so `QT_WINDOWS_APP_PROJECT_IDENTIFIER`
(and Qt's auto-generated manifest) would be a silent no-op. Either switch
`pictura` to `qt_add_executable`, or set the property and call
`qt6_finalize_target(pictura)` after it. Prefer the explicit
`qt6_finalize_target` to avoid changing the target's other behavior.

The same generated manifest carries the `supportedOS` declaration for Windows 10
and Windows 11 (GUID `{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}`), which is what
keeps Windows from applying legacy compatibility shims. The supported floor is
64-bit Windows 10 version 1809 (build 17763), Qt 6.11's documented minimum;
Windows 11 is covered by the same declaration. This is why the manifest
finalization above is a requirement, not a nicety.

### Qt 6.12 is the last Qt that supports Windows 10

Qt's own release notes state Qt 6.12 will be the last version to support
Windows 10. Pinning Qt at 6.11.1 for the release build therefore keeps the
Windows 10 floor; a future Qt bump that drops Windows 10 is a supported-
platform decision to make deliberately, not incidentally.

### Installer via `makensis` + a tracked `.nsi`

`packaging/windows/installer.nsi` packages the staged folder into
`KookaPictura-<version>-windows-x64-setup.exe`: files, Start-menu shortcut,
uninstall registration, `.ico`. NSIS is preinstalled on GitHub Windows runners,
so no install step. Portable is `Compress-Archive` of the staged folder. Both
take the staging directory and version as `makensis` defines rather than baking
paths in.

### Both artifacts ship the loose VC++ runtime DLLs

Both the portable zip and the installer must run without the user preinstalling
anything, so both keep `windeployqt`'s default (it copies `vcruntime140.dll`,
`msvcp140.dll`, …). Qt's docs note these loose DLLs are not intended for
redistribution and recommend the official redistributable; the project accepts
that trade-off for now rather than bundling a `vc_redist` run, which a later
change can add. This is a recorded trade-off, not encoded as a mechanism in the
spec — the spec states the observable behavior: the archive runs on a clean
machine.

### Toolchain and generator

`windows-2022` runner + `jurplel/install-qt-action` (Qt 6.11.1, arch omitted so
it defaults to `win64_msvc2022_64`) + `ilammy/msvc-dev-cmd` for the MSVC
environment + Ninja + `Swatinem/rust-cache`. Configure with
`-DBUILD_TESTING=OFF` (matching `linux-bundle`; otherwise every Qt Test binary
builds). Do **not** pass `-fuse-ld=lld` (Linux-only). Rust target is the runner
default `x86_64-pc-windows-msvc`.

## Risks / Trade-offs

- **Private Qt header path fails on Windows** → The official Qt SDK ships
  private headers, so the existing include path should resolve. The
  "guard the `--interop-probe` path" idea does **not** work — the include is
  unconditional — so the only fallback is linking `Qt6::GuiPrivate`.
- **lcms2 static-fallback not embedded** → as above; the mitigation is to build
  and ship Little CMS, not `LCMS2_STATIC`.
- **Control-server/self-test Windows semantics** → the guard must keep the
  Linux check intact and give Windows a real per-user path; a broad skip would
  hide a genuine failure.
- **Runner smoke check cannot prove "clean machine"** → the runner has Qt and
  the VC++ redist, so it only proves the binary runs with dependencies present;
  the clean-machine requirement is verified manually (task-level), and CI
  additionally asserts the runtime DLLs are present in the archive.
- **CI cannot prove the Windows 10/11 floor** → the `windows-2022` runner is
  Windows Server 2022, a different OS from both Windows 10 and Windows 11, and
  GitHub hosts no Windows 10 runner image. The manifest declaration is asserted
  in CI (inspect the embedded manifest); actually running on Windows 10 (1809+)
  and Windows 11 is a manual verification step.
- **Qt 6.11.1 MSVC arch availability / runner image drift** → pin the Qt
  version and keep the runner on `windows-2022`.
- **Loose runtime redistribution licensing caveat** → documented; the installer
  is the recommended channel.
- **NSIS quoting/paths under GitHub paths with spaces** → pass the staging
  directory as a `makensis` define, never baked into the `.nsi`.

## Migration Plan

1. Land the POSIX guards, CMake, asset, and packaging changes and the new
   action/workflow.
2. Add the Windows job to `release-please.yml`.
3. Verify on a PR dry run (artifacts + self-test), then a real release.
4. Rollback: remove the `windows` job from `release-please.yml`; the workflow and
   packaging files can stay dormant. The POSIX guards are behavior-preserving on
   Linux and need no rollback.
