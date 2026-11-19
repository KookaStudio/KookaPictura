# Tasks

## 1. Test plan

- [x] 1.1 Write `test-plan.md` with one row per scenario in both deltas.

## 2. CI

- [x] 2.1 `packaging/build-packages.sh` derives the deb and rpm architecture
  from the host.
- [x] 2.2 Turn `linux-packages.yml` into an x86_64/aarch64 matrix
  (`ubuntu-24.04` / `ubuntu-24.04-arm`) with per-architecture artifacts, and make
  the upload job upload all of them.

## 3. Updater

- [x] 3.1 Map `arm64`/`aarch64` to `_arm64.deb` and `-1.aarch64.rpm` in
  `packageAssetSuffix`.
- [x] 3.2 `packageUpdateCommand` takes the architecture and builds the file name
  from the asset suffix. `frame_update.cpp` passes the host CPU.
- [x] 3.3 Add the arm64 packages to the `tst_app_updater` fixture, add
  `selectsArm64DebAndRpmAssets` and arm64 command cases, and keep
  `missingAssetFallsBackToReleasePage` on pairs that have no asset.

## 4. Site

- [x] 4.1 `site/download.js` lists `deb (ARM64)` and `rpm (ARM64)` separately;
  `node --test scripts/site-download.test.mjs` covers it.

## 5. Docs

- [x] 5.1 `DEVELOPING.md` and `README.md` name the arm64 packages. `ROADMAP.md`
  ticks #1433.
