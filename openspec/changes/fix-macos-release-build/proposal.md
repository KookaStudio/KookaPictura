# Proposal

## Why

The **macos** release job fails on every Release PR: compiling
`crates/pictura-app/cpp/interop.cpp` aborts with `fatal error: 'rhi/qrhi.h'
file not found`. The RHI headers are pulled in through a hand-rolled include
path that only matches the Debian layout (`CMakeLists.txt:59-64` queries
`qmake6 -query QT_INSTALL_HEADERS`/`QT_VERSION` and `CMakeLists.txt:417` adds
`"${QT_HEADERS_DIR}/QtGui/${QT_VERSION_STR}/QtGui"`); on the macOS framework
install that path does not contain `rhi/qrhi.h`. Because `macos.yml` only runs
the mac build on the release-please PR (`macos.yml:43`), feature CI never
catches this class of breakage until release time (#461).

The same header inclusion also determines whether the app presents the macOS
**native menu bar** correctly: on macOS `QMenuBar` wraps the system-wide bar by
default, and this change is the first time the mac build runs at all, so the
menu-bar contract for macOS is unverified.

## What Changes

- Replace the Debian-specific RHI include path with the Qt-exported private
  target: `find_package(Qt6 REQUIRED COMPONENTS ... GuiPrivate)` and link
  `Qt6::GuiPrivate`, so `#include <rhi/qrhi.h>` resolves on Linux, macOS
  frameworks, and Windows alike. Remove the `qmake6 -query` header/version block
  and the hand-built include directory.
- Guard `interop.cpp`'s Vulkan-only includes (`<rhi/qrhi.h>` and
  `<QtGui/QVulkanInstance>`) behind the same `QT_CONFIG(vulkan)` /
  `__has_include(<vulkan/vulkan.h>)` condition its probe body already uses. The
  official macOS Qt is built without Vulkan, so it ships no `QVulkanInstance`
  header; the unconditional include fails once the include path resolves. This
  is the one source change the build fix needs (see the issue's "build-only"
  note under Impact).
- Copy Qt's offscreen platform plugin into the macOS bundle, so the release
  smoke check (`pictura --headless --self-test` from the deployed bundle) runs
  from the bundle alone, matching what `linux-bundle` and `windows-bundle`
  already deploy.
- **Temporarily** let `macos.yml` run on ordinary pull requests (drop the
  release-please-only `if:` at `macos.yml:43`, and add the C++ source and macOS
  packaging paths to its `paths:` filter) so the fix can be validated in CI
  before merge. This relaxation is scaffolding and is reverted once the macOS
  build is green on `master`.
- Add a macOS menu-bar requirement to the application shell: the app SHALL use
  the native macOS menu bar (the Qt default, no opt-out), and the shell SHALL
  keep that working with screen modes and frame layout. Verification runs on
  the macOS CI runner.

## Capabilities

### New Capabilities

- `meta/macos-release-build`: how the repository compiles the application on
  macOS and publishes the DMG from a release tag, including that the Qt private
  RHI headers resolve through the exported Qt target rather than a
  platform-specific path.

### Modified Capabilities

- `ui/application-shell`: add a requirement that the menu bar is the native
  macOS menu bar on macOS, with the shell not fighting it and no widget
  overlaying it.

## Impact

- Build plus one small app source change (still no engine crate or Qt bridge
  change), unlike the issue's "keep the fix build-only" direction, which did not
  anticipate the Vulkan header:
  - `CMakeLists.txt` — `find_package(Qt6 ... GuiPrivate)`, drop the
    `qmake6 -query` header/version block, drop the hand-built
    `${QT_HEADERS_DIR}/QtGui/${QT_VERSION_STR}/QtGui` include dir, link
    `Qt6::GuiPrivate`.
  - `crates/pictura-app/cpp/interop.cpp` — guard the Vulkan-only includes so
    the translation unit compiles against a Qt built without Vulkan.
  - `.github/actions/macos-bundle/action.yml` — bundle the offscreen QPA plugin
    for the headless smoke check.
  - `.github/workflows/macos.yml` — temporary PR gating/paths relaxation
    (reverted after the fix).
- App C++ for the menu-bar requirement: `crates/pictura-app/cpp/frame.cpp`
  (screen modes) and `frame_columns.cpp` (workspace frame offset) are the code
  the verification exercises; no behavior change is assumed until the mac run
  says otherwise.
- New developer dependency: a Qt install with private headers
  (`qt6-base-private-dev` on Debian-derived systems); CI's `install-qt-action`
  already provides them.
- Follow-up (not in this change): the same release-only gate on `appimage.yml`
  is a latent copy of this bug.
