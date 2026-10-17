# Proposal

## Why

The release pipeline ships Linux (AppImage, `.deb`, `.rpm`), macOS (DMG), and
Windows (portable zip, NSIS installer) artifacts, but there is no snap — even
though `meta/release-please` already names snap as a packaging target. Snap is a
first-class install channel on Ubuntu, the primary Linux distribution this
project builds on, and users currently have nothing to `snap install`.

The codebase has also never been built against a snap base and the KDE Qt6
extension, so a snap packaging break would stay invisible until a release is
attempted. The release-please Release PR builds it as a dry run, so a break
surfaces before the tag is cut.

## What Changes

- Add `snap/snapcraft.yaml`: base `core24`, the `kde-neon-6` extension (Qt 6.11
  from the KDE SDK/content snaps, matching the app's CI Qt), `strict`
  confinement, and a `cmake` part that builds the application from source inside
  snapcraft. The version comes from `version.txt` via `adopt-info`; the
  desktop entry (`packaging/pictura.desktop`) and the application icon are
  reused. Plugs add `home`, `removable-media`, and `cups` on top of the desktop
  plugs the extension supplies.
- Add `.github/workflows/snap.yml`, modelled on `appimage.yml`: callable by
  `release-please.yml` for a release tag, runnable by hand for an existing tag,
  and run by the release-please Release PR as a dry run that keeps the `.snap`
  as a workflow artifact instead of uploading it. Feature PRs never run it. The
  tag build attaches the `.snap` to the GitHub Release as a release asset.
- Wire a `snap` job into `.github/workflows/release-please.yml`, mirroring the
  `appimage`/`linux-packages` jobs.
- Add a "Snap packages" section to `DEVELOPING.md`.
- **Non-goal**: publishing to the Snap Store or other package managers. The
  `.snap` is an unsigned release asset for now; store publishing (credentials,
  name registration, channel) is a future change.
- No BREAKING changes; the existing Linux, macOS, and Windows artifacts are
  unaffected.

## Capabilities

### New Capabilities

- `meta/snap-release-build`: the repository builds a strictly-confined snap of
  the application from a release tag with the KDE Qt6 extension, attaches it to
  the GitHub Release, and dry-runs the build from the release-please Release PR.

### Modified Capabilities

<!-- None: meta/release-please already names snap as a packaging target and its
     "tag is the packaging trigger" requirement does not change. The snap
     build's enabling changes have no existing dedicated behavior spec. -->

## Impact

- **Packaging**: new `snap/snapcraft.yaml`.
- **CI**: new `.github/workflows/snap.yml`, a new job in
  `.github/workflows/release-please.yml`, and one new third-party action,
  `snapcore/action-build` — required because a snap build needs snapd + LXD and
  the action provisions that build environment on the runner; no already-present
  action does this.
- **Assets**: none new; the snap reuses `assets/icons/app.png` (512×512, within
  snapcraft's accepted 40–512 icon range).
- **Docs**: `DEVELOPING.md` gains a snap packaging subsection.
- **Dependencies**: no new Rust or C++ dependencies. The snap build uses the
  `kde-neon-6` extension's Qt (6.11) rather than the `install-qt-action` copy
  used by the other Linux packages, so it does **not** consume the
  `linux-bundle` AppDir.
