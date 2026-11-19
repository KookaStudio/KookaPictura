# Tasks

## 1. Test plan

- [x] 1.1 Write `test-plan.md` with one row per scenario in both deltas.

## 2. CI

- [x] 2.1 Make `linux-bundle` architecture-aware: `uname -m` selects the
  linuxdeploy and Qt-plugin assets and their pinned SHA-256s, and Qt installs as
  `linux_gcc_arm64` on an ARM64 runner.
- [x] 2.2 Turn `appimage.yml` into an x86_64/aarch64 matrix (`ubuntu-24.04` /
  `ubuntu-24.04-arm`) with per-architecture artifacts, and make the upload job
  upload both.

## 3. Updater

- [x] 3.1 Map `arm64`/`aarch64` to `-aarch64.AppImage` in `packageAssetSuffix`.
- [x] 3.2 Add the aarch64 AppImage to the `tst_app_updater` release fixture,
  add `selectsAarch64AppImageAsset`, and keep `missingAssetFallsBackToReleasePage`
  on method/architecture pairs that have no asset.

## 4. Site

- [x] 4.1 `site/download.js` lists `AppImage` (x86_64) and `AppImage (ARM64)`
  separately; `node --test scripts/site-download.test.mjs` covers it.

## 5. Docs

- [x] 5.1 `DEVELOPING.md` and `README.md` name the aarch64 AppImage. `ROADMAP.md`
  ticks it and links #1433 for arm64 deb/rpm.
