# Tasks

## 1. Build the app as a macOS bundle

- [x] 1.1 Add `packaging/macos/Info.plist.in` with `CFBundleExecutable`,
  `CFBundleIdentifier`, `CFBundleName`, `CFBundlePackageType`, the
  `MACOSX_BUNDLE_*` version keys, `CFBundleIconFile`, `LSMinimumSystemVersion`
  13.0, `NSHighResolutionCapable`, and `NSPrincipalClass`.
- [x] 1.2 In `CMakeLists.txt`, mark `pictura` as `MACOSX_BUNDLE` on `APPLE`,
  point `MACOSX_BUNDLE_INFO_PLIST` at the template, set the bundle name,
  identifier (`io.github.KookaPictura.KookaPictura`, matching Windows), and icon
  file, add `assets/icons/app.icns` with `MACOSX_PACKAGE_LOCATION Resources`,
  and give the install rule a `BUNDLE DESTINATION`. Verify a Linux configure
  and build are unaffected (all additions Apple-gated).

## 2. Make Little CMS static on macOS

- [x] 2.1 In `crates/pictura-color/Cargo.toml`, add a
  `[target.'cfg(target_os = "macos")'.dependencies]` edge for `lcms2` with
  `features = ["static"]`; verify with `cargo metadata` that the `static`
  feature is on for `aarch64-apple-darwin` and off for
  `x86_64-unknown-linux-gnu`.
- [x] 2.2 Narrow the CMake lcms2 guard to `UNIX AND NOT APPLE` and update the
  adjacent comment (Linux links the system library; macOS and Windows embed the
  vendored static copy). Verify the Linux configure + build still succeeds.

## 3. Add the macOS icon asset

- [x] 3.1 Generate `assets/icons/app.icns` (128/256/512/1024 px PNG-backed
  entries) from `assets/icons/app.png`; verify it is a valid ICNS.
- [x] 3.2 Add the `app.icns` entry to `assets/PROVENANCE.md` under raster
  artwork with the same CC0-1.0 / MIT license as `app.png`.

## 4. Add the macOS bundle composite action

- [x] 4.1 Create `.github/actions/macos-bundle/action.yml`: install the pinned
  Rust toolchain, Qt 6.11.1 via `jurplel/install-qt-action` (no `arch`, so the
  macOS default applies), `mozilla-actions/sccache-action`, and
  `Swatinem/rust-cache`; configure Ninja/Release with `-DBUILD_TESTING=OFF`,
  `-DCMAKE_OSX_DEPLOYMENT_TARGET=13.0`, and
  `-DCMAKE_CXX_COMPILER_LAUNCHER=sccache` (no `-fuse-ld=lld`); build; run
  `macdeployqt build/pictura.app`. Verify the YAML parses (actionlint or a YAML
  load).

## 5. Add the macOS release workflow

- [x] 5.1 Create `.github/workflows/macos.yml` with `pull_request` (paths: the
  five release-please files, gated to the Release PR), `workflow_call`, and
  `workflow_dispatch` (tag input) triggers, the same `concurrency` block as the
  other packaging workflows, `VERSION` from the tag or the PR number, and the
  sccache env.
- [x] 5.2 Add the smoke check
  `pictura.app/Contents/MacOS/pictura --headless --self-test` with
  `QT_PLUGIN_PATH`/`QML2_IMPORT_PATH` unset, asserting exit 0 and
  `SUMMARY ... failed=0`.
- [x] 5.3 Install `create-dmg` with Homebrew, stage the bundle, and pack
  `KookaPictura-<version>-macOS.dmg` with the volume name, volume icon, and
  Applications drop link, `--skip-jenkins`, and no signing options; add a
  `ponytail:` comment naming the unsigned ceiling.
- [x] 5.4 Keep the DMG as a workflow artifact on pull requests and
  `gh release upload "$TAG" ... --clobber` otherwise.

## 6. Wire the workflow into release-please

- [x] 6.1 Add a `macos` job to `.github/workflows/release-please.yml` with
  `needs: release-please`, `if: needs.release-please.outputs.release_created ==
  'true'`, calling `./.github/workflows/macos.yml` with the tag, mirroring the
  `appimage`/`windows` jobs.

## 7. Document macOS packaging

- [x] 7.1 Add a "macOS packages" subsection to `DEVELOPING.md` next to the
  Windows one: the artifact, Apple Silicon and macOS 13+, the
  release/dispatch/Release-PR triggers, the re-run command, the `macdeployqt`
  and `create-dmg` flow, and the unsigned/Gatekeeper caveat. `DEVELOPING.md` is
  outside `docs/`, so no `TASK-ALLOWS-DOCS` marker is required.

## 8. Validation and verification

- [x] 8.1 `openspec validate --all --strict` passes; `cargo fmt --all --check`
  and `cargo clippy --workspace --all-targets -- -D warnings` pass (the Cargo
  change is macOS-only).
- [x] 8.2 Configure + build the Linux tree to prove the CMake/lcms2 guards are
  behavior-preserving.
- [ ] 8.3 Push the branch and dispatch (or let the Release PR dry-run) the
  macOS workflow; confirm the bundle smoke check passes, the DMG is produced
  and kept as an artifact, and no release is touched.
- [ ] 8.4 After a release, confirm `KookaPictura-X.Y.Z-macOS.dmg` attaches to
  the GitHub Release and opens on an Apple Silicon Mac running macOS 13 or
  later with the self-test passing (the runner cannot cover the macOS 13
  floor).
