# meta/aarch64-appimage-release-build Specification

## Purpose
Defines how the release pipeline builds, verifies, and attaches an aarch64
AppImage of the application alongside the x86_64 one.

## Requirements

### Requirement: An aarch64 AppImage is built for a release tag

The release pipeline SHALL build an aarch64 AppImage of the application on a
native ARM64 Linux runner from the release tag, in addition to the x86_64
AppImage. It SHALL attach the aarch64 AppImage to that GitHub Release as
`KookaPictura-<version>-aarch64.AppImage`, where `<version>` is the tag without
its leading `v`. A manual re-run for an existing tag SHALL rebuild and replace
both AppImage assets.

#### Scenario: A release is cut [arm_appimage_release]

- **WHEN** the release-please Release PR is merged and tag `vX.Y.Z` is created
- **THEN** the release gains both `KookaPictura-X.Y.Z-x86_64.AppImage` and
  `KookaPictura-X.Y.Z-aarch64.AppImage`

#### Scenario: The build is re-run for an existing tag [arm_appimage_rerun]

- **WHEN** `appimage.yml` is dispatched by hand for tag `vX.Y.Z`
- **THEN** both AppImages are rebuilt and re-uploaded, replacing existing assets
  of the same names

#### Scenario: The Release PR dry-runs both architectures [arm_appimage_dry_run]

- **WHEN** the release-please Release PR is opened or updated
- **THEN** both AppImages are built and kept as separate per-architecture
  workflow artifacts, and nothing is uploaded to a release

### Requirement: The aarch64 AppImage is self-contained and self-tests

The aarch64 bundle SHALL deploy Qt 6.11.1 for `linux_gcc_arm64` and its library
dependencies with the aarch64 builds of linuxdeploy and linuxdeploy-plugin-qt.
Those tools SHALL be downloaded at the same pinned releases as the x86_64 tools
and verified against their own pinned SHA-256 digests before they run. The
packaged AppImage SHALL pass its headless self-test in a clean arm64
`ubuntu:24.04` container that has only the host GL, EGL, fontconfig, and
freetype libraries installed.

#### Scenario: Unverified deploy tools are refused [arm_appimage_pins]

- **WHEN** a downloaded linuxdeploy or plugin AppImage does not match the pinned
  SHA-256 for the runner's architecture, or the architecture has no pin
- **THEN** the bundle step fails before running it

#### Scenario: The packaged binary self-tests in a clean container [arm_appimage_smoke]

- **WHEN** the aarch64 AppImage has been packed
- **THEN** `--headless --self-test` exits 0 inside a clean arm64 `ubuntu:24.04`
  container
