# Proposal

## Why

#1419 brought the AppImage to aarch64. The `.deb` and `.rpm` are still
x86_64-only, so arm64 users on Debian/Ubuntu or Fedora have to use the AppImage
and lose package-manager integration (#1433).

## What Changes

- `linux-packages.yml` becomes a two-leg matrix: x86_64 on `ubuntu-24.04` and
  aarch64 on the native `ubuntu-24.04-arm` runner. Both legs reuse the
  architecture-aware `linux-bundle` action from #1419. Each leg builds its
  packages, installs each one into a clean `ubuntu:24.04` / `fedora:latest`
  container of its own architecture, self-tests it, and keeps the packages as a
  per-architecture workflow artifact. The upload job attaches all four. The
  triggers (release tag, manual dispatch, Release-PR dry run) do not change.
- `packaging/build-packages.sh` takes the package architecture from the build
  host (`dpkg --print-architecture` for the deb, `uname -m` for the rpm) instead
  of hard-coding `amd64` / `x86_64`. On ARM it produces
  `KookaPictura_<ver>_arm64.deb` and `KookaPictura-<ver>-1.aarch64.rpm`.
- The in-app updater selects the arm64 deb and the aarch64 rpm on an ARM64
  host. The manual update command it shows names the package file for the
  host's architecture, and shows nothing when the host has no package.
- The site's download picker lists the x86_64 and ARM64 `.deb`/`.rpm`
  separately. Its first-match lookup would otherwise offer
  `-1.aarch64.rpm` to everyone, because that name sorts before `-1.x86_64.rpm`.
- `DEVELOPING.md` and `README.md` name the arm64 packages, and `ROADMAP.md`
  ticks #1433.
- **Non-goals**: arm64 snap and Flatpak, and 32-bit armhf.

## Capabilities

### New Capabilities

- `meta/aarch64-linux-packages-release-build`: the release pipeline builds,
  smoke-tests, and attaches an arm64 `.deb` and an aarch64 `.rpm` next to the
  x86_64 ones.

### Modified Capabilities

- `interop/app-updater`: asset selection and the manual update command cover
  the arm64 deb and the aarch64 rpm.

## Impact

- **CI**: `.github/workflows/linux-packages.yml`; `packaging/build-packages.sh`.
  No new action or dependency.
- **App**: `update_package.{h,cpp}` (suffix and command, which now takes the
  architecture), `frame_update.cpp` (passes the host CPU).
- **Site**: `site/download.js`, covered by `scripts/site-download.test.mjs`.
- **Tests**: `tst_app_updater` gains arm64 deb/rpm selection and command cases.
  The release fixture gains both arm64 packages, so the missing-asset case uses
  pairs that still have no asset.
