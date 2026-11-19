# Design

## Context

`linux-packages.yml` runs one `ubuntu-24.04` job. It calls `linux-bundle` to
fill `./AppDir`, then `packaging/build-packages.sh` repackages that AppDir as a
deb and an rpm. Since #1419, `linux-bundle` picks its linuxdeploy builds by
`uname -m` and installs Qt as `linux_gcc_arm64` on ARM, so it already works on
`ubuntu-24.04-arm`.

## Decisions

- **The same matrix as `appimage.yml`.** A `matrix.include` of `{arch, runner}`
  with `fail-fast: false`. Every step stays the same. Docker on a native
  runner pulls the runner's architecture of `ubuntu:24.04` and `fedora:latest`,
  so the smoke checks need no `--platform`.
- **The packaging script asks the host for its architecture.** The deb uses
  `dpkg --print-architecture` (`amd64` / `arm64`) and the rpm uses `uname -m`
  (`x86_64` / `aarch64`). These are the strings each format uses natively, so
  the file names, the deb `Architecture:` field, the rpm `ExclusiveArch`, and
  the rpmbuild output directory all follow from them. The script cannot
  cross-build: it repackages whatever the AppDir on that host contains.
- **Per-architecture artifacts.** The artifacts are named
  `linux-packages-<arch>`. The upload job downloads `linux-packages-*` with
  `merge-multiple` and uploads with `--clobber`, as `appimage.yml` does.
- **One source for package file names.** `packageUpdateCommand` gains an
  architecture argument and builds the file name from `packageAssetSuffix`.
  The command and the selected asset therefore cannot disagree, and a host with
  no package (riscv64, armhf) gets no command. The banner then offers only the
  release page. The caller passes `QSysInfo::currentCpuArchitecture()`, the same
  value the checker uses to select the asset.

## Risks

- The deb's `Depends` comes from `dpkg-shlibdeps` on the arm runner. A library
  name that differs between arm64 and amd64 is resolved by the arm64 dpkg
  database, and the clean-container install proves the list resolves.
- `fedora:latest` arm64 is a multi-arch manifest entry like the x86_64 one. If
  it drifts, the arm leg of the Release-PR dry run fails before a tag is cut.
