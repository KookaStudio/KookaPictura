# Design

## Context

`appimage.yml` runs one `ubuntu-24.04` job. That job calls the `linux-bundle`
composite action, which does the Release CMake build, the Qt install
(`jurplel/install-qt-action`, Qt 6.11.1), and the linuxdeploy and Qt-plugin
deploy into `./AppDir`. It then packs the AppImage and self-tests it in a clean
container. `linux-packages.yml` reuses the same action for `.deb`/`.rpm`.

## Decisions

- **Native ARM runner, not QEMU.** `ubuntu-24.04-arm` is a GitHub-hosted
  aarch64 runner that is free for public repositories. Every step (apt, rustup,
  sccache, the Qt install, linuxdeploy, docker) runs natively, so the arm job is
  the x86_64 job with a different `runs-on`. A Rust and Qt release build under
  QEMU emulation would take hours.
- **One matrix job, not a second workflow.** Both architectures share every
  step. A `matrix.include` of `{runner, arch}` keeps them in one place, and
  `fail-fast: false` keeps one architecture's failure from cancelling the
  other's dry run.
- **The bundle action picks the architecture from the machine.** It reads
  `uname -m` (`x86_64` or `aarch64`), which matches linuxdeploy's asset names
  and the AppImage name, so callers pass nothing new. The SHA-256 pins are
  selected by that name, and an unknown machine fails the step. The
  linuxdeploy release tags do not change. The aarch64 pins were hashed from
  the same releases as the x86_64 ones, and the x86_64 hash re-verified, on
  2026-11-19.
- **Qt arch is explicit on ARM.** install-qt-action detects the
  `linux_arm64` host by itself but leaves the arch empty for aqtinstall's
  default, so the action passes `linux_gcc_arm64` on an `ARM64` runner and
  leaves the arch empty on x86_64, which is today's behavior.
- **Per-architecture artifacts.** The artifacts are named `appimage-<arch>`.
  The upload job downloads `appimage-*` with `merge-multiple` and uploads every
  `KookaPictura-*.AppImage` with `--clobber`, so re-running one tag replaces
  both assets.
- **Updater suffix.** `QSysInfo::currentCpuArchitecture()` reports `arm64` on
  aarch64 Linux, and some environments report `aarch64`. Both map to
  `-aarch64.AppImage` for the AppImage method only. Methods with no ARM
  artifact still return no suffix, so the UI keeps offering the release page.

## Risks

- `ubuntu-24.04-arm` image drift: it is a partner image. If rustup or docker
  goes missing there, the arm leg of the Release-PR dry run fails visibly
  before any tag is cut.
- linuxdeploy-plugin-qt on aarch64 is less exercised upstream. The clean
  container self-test is the guard that catches a library it failed to bundle.
