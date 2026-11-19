# Proposal

## Why

Every Linux release artifact is x86_64-only. ARM64 Linux machines (Asahi Macs,
Snapdragon laptops, Raspberry Pi 4/5 on a 64-bit OS, ARM workstations) have
nothing to run, and users have asked for equivalent ARM builds (#1419). The
AppImage is the self-contained, distribution-neutral artifact, so it is the
first one to bring to aarch64. The arm64 `.deb`/`.rpm` follow in #1433.

## What Changes

- `appimage.yml` builds a matrix of two jobs: x86_64 on `ubuntu-24.04` and
  aarch64 on GitHub's native `ubuntu-24.04-arm` runner. Each job packs
  `KookaPictura-<version>-<arch>.AppImage`, smoke-tests it in a clean
  `ubuntu:24.04` container of the same architecture, and keeps it as a
  per-architecture workflow artifact. The upload job attaches both to the
  release. Triggers (release tag, manual dispatch, Release-PR dry run) are
  unchanged.
- The `linux-bundle` composite action becomes architecture-aware. It downloads
  the linuxdeploy and linuxdeploy-plugin-qt builds for the runner's machine at
  the same pinned releases, verifies each against its own pinned SHA-256, and
  installs Qt 6.11.1 for `linux_gcc_arm64` on an ARM64 runner.
  `linux-packages.yml` continues to run only on x86_64.
- The in-app updater selects `KookaPictura-<version>-aarch64.AppImage` when an
  AppImage runs on aarch64/arm64, so ARM AppImage users self-update like x86_64
  users. Every other package method keeps its x86_64-only asset names.
- The site's download picker (`site/download.js`) lists the x86_64 AppImage and
  the ARM64 AppImage as separate entries. Otherwise its first-match lookup
  would offer whichever sorts first, which is `-aarch64`.
- `DEVELOPING.md` and `README.md` name the aarch64 AppImage. `ROADMAP.md`
  ticks it and lists the arm64 deb/rpm follow-up (#1433).
- **Non-goals**: 32-bit ARM (armhf), and arm64 `.deb`/`.rpm`/snap/Flatpak.

## Capabilities

### New Capabilities

- `meta/aarch64-appimage-release-build`: the release pipeline builds,
  smoke-tests, and attaches an aarch64 AppImage next to the x86_64 one.

### Modified Capabilities

- `interop/app-updater`: asset selection matches the aarch64 AppImage on an
  ARM64 host.

## Impact

- **CI**: `.github/workflows/appimage.yml`, `.github/actions/linux-bundle/action.yml`.
  This is the first use of a GitHub-hosted ARM runner (`ubuntu-24.04-arm`).
  It needs no new third-party action or dependency.
- **App**: `crates/pictura-app/cpp/update_package.cpp` (asset suffix).
- **Site**: `site/download.js`, covered by `scripts/site-download.test.mjs`.
- **Tests**: `tst_app_updater` gains an aarch64 selection case. The shared
  release fixture now carries the aarch64 AppImage, which a real release will
  have, so the missing-asset case uses a method/architecture pair that still
  has no asset. `checkOnLaunchSendsOneRequest` expects the asset for the host
  CPU, which also stops it from failing on aarch64 development hosts.
