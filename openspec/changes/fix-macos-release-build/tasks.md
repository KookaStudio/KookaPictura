# Tasks

## 1. Cross-platform RHI include

- [x] 1.1 In `CMakeLists.txt`, add `GuiPrivate` to the Qt component lookup, drop
  the `qmake6 -query QT_INSTALL_HEADERS`/`QT_VERSION` block (`:59-64`), drop the
  `"${QT_HEADERS_DIR}/QtGui/${QT_VERSION_STR}/QtGui"` include dir (`:417`), link
  `Qt6::GuiPrivate` PRIVATE on `pictura_shell` (`:425`), and fix the stale
  Debian-only comment. Keep `find_program(QMAKE_EXECUTABLE ...)`: `cxx_qt_import_crate`
  still needs it. Verify: a clean `cmake -S . -B build -G Ninja` configures, and
  `cmake --build build` compiles `interop.cpp` (the Linux `qt-headless` build).
- [x] 1.2 Add a note to `DEVELOPING.md` that a Qt install with private headers is
  required (`qt6-base-private-dev` on Debian-derived systems; CI's
  `install-qt-action` already provides it). Verify: the documented package name
  resolves the `Qt6::GuiPrivate` configure failure on a system missing it.
- [x] 1.3 Guard `interop.cpp`'s Vulkan-only includes (`<rhi/qrhi.h>`,
  `<QtGui/QVulkanInstance>`) with the same `QT_CONFIG(vulkan)` /
  `__has_include(<vulkan/vulkan.h>)` condition the probe body uses, so the
  translation unit compiles against a Qt built without Vulkan (the macOS Qt).
  Verify: compile the translation unit with the guards forced to the
  no-Vulkan outcome and confirm it builds.
- [ ] 1.4 Copy Qt's offscreen platform plugin (`libqoffscreen.dylib`) into the
  macOS bundle in `macos-bundle/action.yml` before `macdeployqt`, so the
  `--headless` smoke check runs from the bundle alone (linux-bundle and
  windows-bundle already deploy it). Verify: the macOS job's `Smoke-check the
  deployed bundle` step passes on this PR.
- [ ] 1.5 Guard the GPU probes (`pictura-render` `request_device()` and
  `pictura-app` `gpu.rs::create_device()`) against the `wgpu::Instance::new`
  panic when no implemented backend is compiled for the target (Apple),
  returning "no GPU" so the self-test runs on the CPU path. Verify: the macOS
  self-test runs to completion on this PR.

## 2. Enable the macOS build on this PR (temporary)

- [x] 2.1 In `.github/workflows/macos.yml`, drop the release-please-only `if:`
  (`:43`) and add `crates/pictura-app/cpp/**` and `packaging/macos/**` to the
  `pull_request.paths:` list, each with a `ponytail:` comment naming the ceiling
  and the revert. Verify: pushing this branch triggers the **macos** job on the
  pull request, and it still runs for `workflow_dispatch`/`workflow_call`.

## 3. Native macOS menu bar verification

- [x] 3.1 Add a Qt Test under `crates/pictura-app/cpp/tests/` that asserts
  `QMainWindow::menuBar()->isNativeMenuBar()` reports the platform-appropriate
  value (true on macOS, false elsewhere) and is unchanged after entering and
  leaving Full Screen With Menu Bar and Full Screen; register it in
  `crates/pictura-app/cpp/tests/CMakeLists.txt`. Verify: `ctest --test-dir build -R '^tst_' --output-on-failure` passes offscreen on Linux.
- [ ] 3.2 Run the check on the macOS runner and, if the offscreen QPA makes
  `isNativeMenuBar()` unreliable there, replace it with a documented manual macOS
  step recorded in `design.md`. Verify: the macOS job reports the check green, or
  the manual step and its macOS result are recorded.

## 4. Integration and revert

- [ ] 4.1 Confirm the full macOS dry run is green on this PR (build,
  smoke-check, DMG artifact) and the Linux `qt-headless` job still builds and
  self-tests. Verify: both checks green on the same commit.
- [ ] 4.2 Revert the temporary `macos.yml` gate/paths edits from group 2 once the
  macOS build is green on `master`, and record the revert. Verify: the mac job no
  longer runs on ordinary feature PRs and still runs on the Release PR/tag, and
  its `paths:` list is back to the original.
- [x] 4.3 Update `docs/dev/STATE.md` with the macOS build fix and the follow-up
  (the same release-only gate on `appimage.yml:40`). Verify: the resume anchor
  names this issue's outcome and the follow-up; commit carries
  `TASK-ALLOWS-DOCS`.
