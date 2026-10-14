# Spec Delta

## Purpose

Defines how the repository compiles the application on macOS and publishes the
DMG from a release tag, including that the Qt private RHI headers resolve
through the exported Qt target on every supported platform.

## ADDED Requirements

### Requirement: The application compiles on macOS

The application SHALL compile the C++ shell and the Rust static library with the
Apple Clang toolchain on the macOS runner, including the translation unit that
includes `<rhi/qrhi.h>`, and SHALL link without a system Little CMS library.

#### Scenario: The RHI translation unit compiles on macOS

- **WHEN** the macOS build configures and compiles the C++ shell
- **THEN** the translation unit that includes `<rhi/qrhi.h>` compiles and the
  build does not stop with a missing-header error

#### Scenario: A Qt built without Vulkan still compiles

- **WHEN** the C++ shell is configured against a Qt built without Vulkan (the
  official macOS Qt, which ships no `QVulkanInstance` header)
- **THEN** the interop translation unit compiles with its diagnostic probe
  stubbed out rather than failing on a missing Vulkan header

#### Scenario: The static library links on macOS

- **WHEN** the macOS build links the application
- **THEN** it does not require an external Little CMS library to be present on
  the runner

### Requirement: Qt private headers resolve through the exported target

The build SHALL obtain the Qt GUI private (RHI) include directories from Qt's
exported private target rather than a hand-built path, so the same configuration
resolves the headers on the Linux, macOS framework, and Windows Qt layouts.

#### Scenario: The private include directory is platform-correct

- **WHEN** CMake configures the project against a Qt installation that provides
  private headers
- **THEN** the private include directories come from the Qt-exported private
  target and the hand-built `${QT_INSTALL_HEADERS}/QtGui/<version>/QtGui` path is
  not used

#### Scenario: A missing private module fails configuration clearly

- **WHEN** CMake configures the project against a Qt installation without the
  GUI private module
- **THEN** configuration fails with an error naming the missing Qt private
  package rather than compiling a translation unit with an unresolved include

### Requirement: macOS artifacts are built for a release tag

The repository SHALL build the macOS application bundle and attach a DMG named
`KookaPictura-<version>-macOS.dmg` to each GitHub Release, where `<version>` is
the release tag without its leading `v`, starting from the release tag.

#### Scenario: A release is cut

- **WHEN** the release-please Release PR is merged and tag `vX.Y.Z` is created
- **THEN** the release gains `KookaPictura-X.Y.Z-macOS.dmg` as an asset

#### Scenario: The build is re-run for an existing tag

- **WHEN** the macOS workflow is dispatched by hand for tag `vX.Y.Z`
- **THEN** it rebuilds and re-uploads the DMG to the `vX.Y.Z` release, replacing
  any existing asset of the same name

### Requirement: The macOS bundle runs its headless self-test

The deployed macOS bundle SHALL include the offscreen QPA plugin, so that
`pictura --headless` runs from the bundle alone without the build machine's Qt
install, and the release workflow's smoke check SHALL pass.

#### Scenario: The headless self-test passes from the bundle

- **WHEN** the deployed bundle is run as `pictura --headless --self-test` with
  the Qt plugin path unset
- **THEN** it finds the offscreen platform plugin inside the bundle, runs the
  self-test, and exits 0

#### Scenario: The GPU probe degrades where the platform has no backend

- **WHEN** the self-test runs on a platform whose wgpu build has no backend
  implemented for it (macOS, which the workspace builds Vulkan-only)
- **THEN** the GPU probe reports no GPU and the self-test continues on the CPU
  path rather than aborting the process
