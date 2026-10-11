# Tasks

## 1. Make the application compile and self-test on Windows

- [x] 1.1 Guard the POSIX-only code in
  `crates/pictura-app/cpp/control_server.cpp`: replace `::getuid()` in the
  control socket path (line ~244) with a per-user location that works on Windows
  (e.g. derived from `QStandardPaths`/temp), and make the `S_ISSOCK` checks
  (lines ~253, ~281) `#ifdef`-guarded so they compile under MSVC. Verify the
  Linux build still compiles and `cargo`/CMake tests are unaffected
  (`cmake --build build --parallel` then the existing self-test passes).
- [x] 1.2 Guard the control self-test in
  `crates/pictura-app/cpp/selftest_control.cpp`: the `stat()`-based "user-only"
  mode check (lines ~107-114, exit code 465) has no Windows analogue for a
  named pipe. Express it as a platform guard so the check still runs on Linux
  and is replaced by a Windows-appropriate assertion (or explicitly skipped)
  rather than failing. Verify by reasoning through both branches and confirming
  the Linux self-test still passes.
- [x] 1.3 Audit the rest of `crates/pictura-app/cpp/` for other POSIX-only
  assumptions reachable in a headless run (only `control_server.cpp` and
  `selftest_control.cpp` currently include `<unistd.h>`/`<sys/stat.h>`); verify
  no other `<unistd.h>`/`<sys/socket.h>`/`<sys/un.h>` include remains unguarded.

## 2. Make the CMake build portable to Windows

- [x] 2.1 Move the Little CMS lookup in `CMakeLists.txt` into `if(NOT WIN32)`
  and append `"${LCMS2_LIBRARY}"` conditionally, leaving the existing
  `target_link_libraries(pictura_shell PUBLIC pictura_app Qt6::Svg
  Qt6::Network Qt6::PrintSupport ...)` call intact (do not wrap the whole call).
  Update the adjacent comment to state that `lcms2-sys`'s static fallback embeds
  the vendored Little CMS into `pictura_app` on Windows. Verify a Linux
  configure + build still succeeds.
- [x] 2.2 Enable the `RC` language (add `RC` to `project(... LANGUAGES ...)` or
  `enable_language(RC)`) and set `WIN32_EXECUTABLE ON` on `pictura`; call
  `qt6_finalize_target(pictura)` after setting
  `QT_WINDOWS_APP_PROJECT_IDENTIFIER`, and generate/add the Windows resource
  (`packaging/windows/pictura.rc.in` configured to `pictura.rc`) to the
  `pictura` sources. Verify the Linux configure + build is unaffected
  (all additions are Windows-gated).
- [ ] 2.3 Confirm the private-header include path for
  `crates/pictura-app/cpp/interop.cpp` (`<rhi/qrhi.h>`) resolves under the
  official Windows Qt install during the first dry run; if it does not, link
  `Qt6::GuiPrivate` (the only viable fallback — the include is unconditional, so
  guarding the `--interop-probe` path does not remove it). Verify by the
  dry-run configure reaching the C++ compile stage.
- [ ] 2.4 Confirm the finalization from 2.2 embeds the application manifest and
  that it declares Windows 10 and Windows 11 support (the `supportedOS` GUID
  `{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}`); verify by extracting the manifest
  (`mt.exe -inputresource:pictura.exe;#1 -out:manifest.xml` or `dumpbin`) in the
  dry-run build.

## 3. Add the Windows icon asset

- [x] 3.1 Generate `assets/icons/app.ico` (256/128/64/48/32/16) from
  `assets/icons/app.png`; verify the file exists and is a valid multi-size ICO.
- [x] 3.2 Add the `app.ico` entry to `assets/PROVENANCE.md` under raster
  artwork with the same CC0-1.0 / MIT license as `app.png`; verify the file
  lists it.
- [x] 3.3 Add `packaging/windows/pictura.rc.in` (configured to `pictura.rc`)
  embedding the icon (referenced as `@CMAKE_CURRENT_SOURCE_DIR@/...`) and a
  version-info block generated from `PROJECT_VERSION` (not hardcoded); verify
  the resource compiles in the dry-run build.

## 4. Add the Windows bundle composite action

- [x] 4.1 Create `.github/actions/windows-bundle/action.yml`: install Qt 6.11.1
  via `jurplel/install-qt-action` on `windows-2022` (omit `arch` so it defaults
  to `win64_msvc2022_64`), set up MSVC (`ilammy/msvc-dev-cmd`, pinned), choose
  an explicit shell, and configure Ninja/Release with
  `-DBUILD_TESTING=OFF -DCMAKE_INSTALL_PREFIX=<staging>` (no `-fuse-ld=lld`),
  then `cmake --build` and `cmake --install`. Verify with `actionlint` (or a
  YAML parse) that the action is valid.
- [x] 4.2 Add the `windeployqt --release` step to the action, deploying onto
  the installed `pictura.exe`. Verify on the dry run that the staging folder
  contains `pictura.exe`, the `Qt6*.dll` files, and `platforms\qwindows.dll`.

## 5. Add the NSIS installer packaging

- [x] 5.1 Add `packaging/windows/installer.nsi` that packages the staged folder
  into `KookaPictura-<version>-windows-x64-setup.exe`, taking the staging
  directory and version as `makensis` defines (never baked paths), and creating
  a Start-menu shortcut and an uninstall entry. Verify by building it locally on
  a Windows host or by the dry run producing the `.exe`.
- [ ] 5.2 Verify a silent install on a clean Windows machine installs a working
  binary (`pictura.exe --headless --self-test` exits 0) and that the registered
  uninstaller removes the files.

## 6. Add the Windows release workflow

- [x] 6.1 Create `.github/workflows/windows.yml` with `pull_request` (paths:
  the workflow, `windows-bundle/**`, `packaging/windows/**`, `CMakeLists.txt`,
  `assets/icons/app.ico`), `workflow_call` (tag input), and `workflow_dispatch`
  (tag input) triggers, mirroring `appimage.yml`. Verify with `actionlint` and
  by `gh workflow view` after push.
- [x] 6.2 Define `VERSION` the way the Linux workflows do
  (`${{ inputs.tag || format('pr{0}', github.event.number) }}`) and add the
  portable-archive step (`Compress-Archive` of the staged folder →
  `KookaPictura-<version>-windows-x64-portable.zip`) and the `makensis`
  installer step. Verify the dry run produces both artifacts.
- [x] 6.3 Add the smoke check `pictura.exe --headless --self-test` against the
  staged binary and assert exit 0 with `SUMMARY ... failed=0`; also assert the
  runtime DLLs (`vcruntime140*.dll`, `msvcp140*.dll`) are present in the
  staging folder. Verify it runs in the dry run.
- [x] 6.4 Add the upload logic: on pull requests keep both artifacts as
  workflow artifacts; otherwise `gh release upload "$TAG" ... --clobber`.
  Verify the dry run uploads artifacts and no release is touched.

## 7. Wire the workflow into release-please

- [x] 7.1 Add a `windows` job to `.github/workflows/release-please.yml` with
  `needs: release-please`, `if: needs.release-please.outputs.release_created ==
  'true'`, calling `./.github/workflows/windows.yml` with the tag. Verify
  with `actionlint` and by confirming the job's `uses`/`with` match
  `appimage.yml`'s wiring.

## 8. Document Windows packaging

- [x] 8.1 Add a "Windows packages" subsection to `DEVELOPING.md` next to the
  Linux packages section: the two artifacts, the supported Windows versions
  (64-bit Windows 10 1809+ and Windows 11), the release/dispatch/PR-dry-run
  triggers, and the re-run command for an existing tag. Verify the documented
  commands match the workflow (workflow name, tag input, artifact names). This
  is a separate commit (`DEVELOPING.md` is outside `docs/`, so no
  `TASK-ALLOWS-DOCS` marker is required; if `docs/dev/STATE.md` is also updated,
  that commit does need the marker).

## 9. Integration verification

- [ ] 9.1 Open the change as a pull request and confirm the dry run builds,
  self-tests with `failed=0`, and stores the portable zip and installer as
  workflow artifacts.
- [ ] 9.2 After merge, cut (or force) a release and confirm both Windows assets
  attach to the GitHub Release and the portable archive passes the self-test on
  a clean Windows machine.
- [ ] 9.3 Verify the supported-version floor manually: run the portable archive
  and the installed application on a clean 64-bit Windows 10 (1809+) machine and
  on a clean 64-bit Windows 11 machine, and confirm `pictura.exe --headless
  --self-test` exits 0 on both (the `windows-2022` runner cannot cover either
  OS).
