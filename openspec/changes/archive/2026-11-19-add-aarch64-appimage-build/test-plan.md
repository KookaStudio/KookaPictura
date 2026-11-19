# Test Plan

Covers every scenario in this change's two deltas. The `meta/` rows are
CI/packaging behavior that `scripts/check-test-plan.py` cannot name, so they
read `manual:` and name the CI check or artifact that proves them. The
`interop/app-updater` rows are Qt Test cases.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `arm_appimage_release` | Merge the release-please Release PR so tag `vX.Y.Z` is created and `appimage.yml` runs | The release lists `KookaPictura-X.Y.Z-x86_64.AppImage` and `KookaPictura-X.Y.Z-aarch64.AppImage` | manual: release-asset check on the GitHub Release (needs a real tag) |
| `arm_appimage_rerun` | `gh workflow run appimage.yml -f tag=vX.Y.Z` | Both matrix legs pass, and the upload job replaces both assets (`--clobber`) | manual: dispatch run against an existing release |
| `arm_appimage_dry_run` | Open or update the release-please Release PR | The `appimage (x86_64)` and `appimage (aarch64)` jobs both pass, artifacts `appimage-x86_64` and `appimage-aarch64` exist, and the upload job is skipped | manual: Release-PR workflow run (feature PRs do not run it) |
| `arm_appimage_pins` | Inspect the Deploy step log of the aarch64 leg | `sha256sum -c` prints `OK` for both `linuxdeploy-aarch64.AppImage` and `linuxdeploy-plugin-qt-aarch64.AppImage`, and an unknown `uname -m` exits before any download | manual: CI log review |
| `arm_appimage_smoke` | The aarch64 leg's smoke-check step runs the AppImage in `ubuntu:24.04` (arm64) | It prints `pictura self-test: SUMMARY ... failed=0` and exits 0 | manual: the appimage workflow's smoke-check step on the arm runner |
| `au_select_appimage` | Select an AppImage asset on `x86_64` from the fixture, which also holds the aarch64 AppImage | Returns `KookaPictura-0.4.0-x86_64.AppImage` | `tst_app_updater::selectsAppImageAsset` |
| `au_select_appimage_aarch64` | Select an AppImage asset on `arm64` and on `aarch64` | Both return `KookaPictura-0.4.0-aarch64.AppImage` | `tst_app_updater::selectsAarch64AppImageAsset` |
| `au_select_windows` | Select a Windows setup asset on `x64` | Returns the `-windows-x64-setup.exe` asset | `tst_app_updater::selectsWindowsSetupAsset` |
| `au_select_macos` | Select a macOS DMG asset | Returns the `-macOS.dmg` asset | `tst_app_updater::selectsMacDmgAsset` |
| `au_select_missing` | Select deb on `aarch64`, AppImage on `riscv64`, and a development build | No asset is returned in any case | `tst_app_updater::missingAssetFallsBackToReleasePage` |
