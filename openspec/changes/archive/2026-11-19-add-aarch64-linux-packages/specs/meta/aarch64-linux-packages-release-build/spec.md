# Spec Delta

## Purpose

Defines how the release pipeline builds, verifies, and attaches an arm64 `.deb`
and an aarch64 `.rpm` of the application next to the x86_64 packages.

## ADDED Requirements

### Requirement: arm64 deb and rpm packages are built for a release tag

The release pipeline SHALL build the `.deb` and `.rpm` on a native ARM64 Linux
runner from the release tag, in addition to the x86_64 packages. It SHALL attach
them to that GitHub Release as `KookaPictura_<version>_arm64.deb` and
`KookaPictura-<version>-1.aarch64.rpm`, where `<version>` is the tag without its
leading `v`. A manual re-run for an existing tag SHALL rebuild and replace all
four package assets.

#### Scenario: A release is cut [arm_pkg_release]

- **WHEN** the release-please Release PR is merged and tag `vX.Y.Z` is created
- **THEN** the release gains `KookaPictura_X.Y.Z_amd64.deb`,
  `KookaPictura-X.Y.Z-1.x86_64.rpm`, `KookaPictura_X.Y.Z_arm64.deb`, and
  `KookaPictura-X.Y.Z-1.aarch64.rpm`

#### Scenario: The build is re-run for an existing tag [arm_pkg_rerun]

- **WHEN** `linux-packages.yml` is dispatched by hand for tag `vX.Y.Z`
- **THEN** all four packages are rebuilt and re-uploaded, replacing existing
  assets of the same names

#### Scenario: The Release PR dry-runs both architectures [arm_pkg_dry_run]

- **WHEN** the release-please Release PR is opened or updated
- **THEN** both architectures' packages are built and kept as separate
  per-architecture workflow artifacts, and nothing is uploaded to a release

### Requirement: The arm64 packages declare their architecture and self-test

The arm64 `.deb` SHALL declare `Architecture: arm64` and the aarch64 `.rpm`
SHALL be built for `aarch64` only. Each package SHALL install, with its
dependencies resolved from the distribution, into a clean container of its own
architecture (`ubuntu:24.04` for the deb, `fedora:latest` for the rpm). Each
SHALL install the license and AppStream metainfo files, and the installed
`pictura` SHALL pass its headless self-test there.

#### Scenario: The arm64 deb self-tests on Ubuntu [arm_pkg_deb_smoke]

- **WHEN** the arm64 `.deb` is installed with `apt-get` into a clean arm64
  `ubuntu:24.04` container
- **THEN** its dependencies resolve, the license and metainfo files exist, and
  `pictura --headless --self-test` exits 0

#### Scenario: The aarch64 rpm self-tests on Fedora [arm_pkg_rpm_smoke]

- **WHEN** the aarch64 `.rpm` is installed with `dnf` into a clean arm64
  `fedora:latest` container
- **THEN** its dependencies resolve, the license and metainfo files exist, and
  `pictura --headless --self-test` exits 0
