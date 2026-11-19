# Test Plan

Covers every scenario in this change's two deltas. The `meta/` rows are
CI/packaging behavior that only runs on the release-please Release PR, a tag, or
a manual dispatch, so they read `manual:` and name the CI check or artifact that
proves them. The `interop/app-updater` rows are Qt Test cases.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `arm_pkg_release` | Merge the release-please Release PR so tag `vX.Y.Z` is created and `linux-packages.yml` runs | The release lists the amd64 and arm64 `.deb` and the x86_64 and aarch64 `.rpm` | manual: release-asset check on the GitHub Release (needs a real tag) |
| `arm_pkg_rerun` | `gh workflow run linux-packages.yml -f tag=vX.Y.Z` | Both matrix legs pass, and the upload job replaces all four assets (`--clobber`) | manual: dispatch run against an existing release |
| `arm_pkg_dry_run` | Open or update the release-please Release PR | `packages (x86_64)` and `packages (aarch64)` both pass, artifacts `linux-packages-x86_64` and `linux-packages-aarch64` exist, and the upload job is skipped | manual: Release-PR workflow run (feature PRs do not run it) |
| `arm_pkg_deb_smoke` | The aarch64 leg installs the `_arm64.deb` in `ubuntu:24.04` (arm64) | `apt-get` resolves the dependencies, the license and metainfo files exist, and the self-test prints `SUMMARY ... failed=0` | manual: the linux-packages workflow's Ubuntu smoke-check step on the arm runner |
| `arm_pkg_rpm_smoke` | The aarch64 leg installs the `-1.aarch64.rpm` in `fedora:latest` (arm64) | `dnf` resolves the dependencies, the license and metainfo files exist, and the self-test prints `SUMMARY ... failed=0` | manual: the linux-packages workflow's Fedora smoke-check step on the arm runner |
| `au_select_appimage` | Select an AppImage asset on `x86_64` from the fixture, which also holds the aarch64 AppImage | Returns `KookaPictura-0.4.0-x86_64.AppImage` | `tst_app_updater::selectsAppImageAsset` |
| `au_select_appimage_aarch64` | Select an AppImage asset on `arm64` and on `aarch64` | Both return `KookaPictura-0.4.0-aarch64.AppImage` | `tst_app_updater::selectsAarch64AppImageAsset` |
| `au_select_deb_rpm_arm64` | Select deb and rpm assets on `arm64` and on `aarch64` from a fixture that holds both architectures | deb returns `KookaPictura_0.4.0_arm64.deb` and rpm returns `KookaPictura-0.4.0-1.aarch64.rpm` | `tst_app_updater::selectsArm64DebAndRpmAssets` |
| `au_select_windows` | Select a Windows setup asset on `x64` | Returns the `-windows-x64-setup.exe` asset | `tst_app_updater::selectsWindowsSetupAsset` |
| `au_select_macos` | Select a macOS DMG asset | Returns the `-macOS.dmg` asset | `tst_app_updater::selectsMacDmgAsset` |
| `au_select_missing` | Select deb on `riscv64`, AppImage on `riscv64`, and a development build | No asset is returned in any case | `tst_app_updater::missingAssetFallsBackToReleasePage` |
| `au_command_deb_rpm` | Ask for the deb and rpm command on `x86_64` for `v0.4.0` / `0.4.0` | The amd64 deb and x86_64 rpm `apt`/`dnf` commands | `tst_app_updater::packageCommands` |
| `au_command_deb_rpm_arm64` | Ask for the deb and rpm command on `arm64` and on `aarch64` | The arm64 deb and aarch64 rpm `apt`/`dnf` commands | `tst_app_updater::arm64PackageCommands` |
| `au_command_missing_arch` | Ask for the deb and rpm command on `riscv64` | Both commands are empty | `tst_app_updater::arm64PackageCommands` |
