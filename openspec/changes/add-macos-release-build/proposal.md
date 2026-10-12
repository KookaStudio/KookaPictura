# Proposal

## Why

The release pipeline ships Linux (AppImage, `.deb`, `.rpm`) and Windows
(portable zip, NSIS installer) artifacts, but macOS users have nothing to
download even though `meta/release-please` names DMG as a packaging target. The
codebase has also never been built with the Apple toolchain, so macOS build
breaks stay invisible until a release is attempted.

## What Changes

- Add a macOS release build that produces
  `KookaPictura-<version>-macOS.dmg` for Apple Silicon (arm64) from a release
  tag: an application bundle carrying the Qt frameworks and plugins, packed as
  a DMG.
- Make the CMake build produce a proper macOS application bundle on `APPLE`:
  `MACOSX_BUNDLE` with a committed `Info.plist` template (bundle identifier,
  name, versions, icon, minimum macOS 13), the application icon
  (`assets/icons/app.icns`) in `Contents/Resources`, and the bundle covered by
  the existing install rules.
- Make Little CMS static on macOS so the DMG needs no external dylib:
  `pictura-color` enables `lcms2`'s `static` feature for
  `cfg(target_os = "macos")` (the same vendored copy Windows already compiles)
  and the CMake system-lcms2 link guard narrows to `UNIX AND NOT APPLE`.
- Add a composite action `.github/actions/macos-bundle` that builds a
  Ninja/Release bundle on `macos-15` with Qt 6.11.1
  (`jurplel/install-qt-action`) and deploys Qt into it with `macdeployqt`.
- Add `.github/workflows/macos.yml` (`workflow_call` + `workflow_dispatch` +
  release-please Release PR dry run) that smoke-checks the deployed bundle with
  `--headless --self-test`, packs the DMG with `create-dmg`, and attaches it to
  the release; wire it into `release-please.yml`.
- Leave code signing and notarization out until an Apple Developer account
  exists: the artifacts are unsigned, with a `ponytail:` marker naming the
  ceiling and a `DEVELOPING.md` note that Gatekeeper needs a right-click open.
- Add `assets/icons/app.icns` (converted from `app.png`) and record it in
  `assets/PROVENANCE.md`; add a "macOS packages" section to `DEVELOPING.md`.
- No BREAKING changes; the Linux and Windows builds and artifacts are
  unaffected.

## Capabilities

### New Capabilities

- `meta/macos-release-build`: the repository builds a macOS application bundle
  and publishes it as a DMG from a release tag, deploying Qt into the bundle
  with `macdeployqt`, keyed off the release event like the other packaging
  workflows, and dry-runnable from the release-please Release PR.

### Modified Capabilities

<!-- None: the release-trigger contract in meta/release-please already covers
     packaging generically and its requirements do not change. The macOS
     lcms2/bundle build changes are packaging-enabling changes with no existing
     dedicated behavior spec. -->

## Impact

- **Build**: `CMakeLists.txt` gains Apple-only bundle properties and install
  destinations; the lcms2 link guard narrows to `UNIX AND NOT APPLE`.
- **Rust**: `crates/pictura-color/Cargo.toml` gains a macOS-only dependency
  edge enabling `lcms2/static`; no lockfile change and no Linux/Windows
  behavior change.
- **CI**: new `.github/actions/macos-bundle`, new `.github/workflows/macos.yml`,
  and a new job in `.github/workflows/release-please.yml`; no new third-party
  actions (`jurplel/install-qt-action`, `mozilla-actions/sccache-action`,
  `Swatinem/rust-cache` are already used).
- **Packaging**: new `packaging/macos/Info.plist.in`.
- **Assets**: new `assets/icons/app.icns` plus a `PROVENANCE.md` entry.
- **Docs**: `DEVELOPING.md` gains a macOS packaging subsection.
- **Dependencies**: no new Rust or C++ dependencies; `lcms2-sys` already
  vendors its source for the static build.
