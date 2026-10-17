# Design

## Context

See `proposal.md` for motivation. Constraints that shape the approach:

- The release pipeline already has a shape to copy: `appimage.yml` and
  `linux-packages.yml` are `workflow_call` + `workflow_dispatch` + a
  pull-request dry run, and `release-please.yml` calls them by tag. The snap is
  a fifth such workflow.
- The repository pins **Qt 6.11.1** in CI and links Qt modules
  Core/Gui/Widgets/Svg/Network/PrintSupport/GuiPrivate; the rendering path uses
  QRhi (Vulkan/OpenGL). The app already detects `SNAP`/`SNAP_NAME` and switches
  to xdg-desktop-portal file dialogs when sandboxed.
- The existing Linux packages build with `linux-bundle` into an `AppDir` and
  deploy Qt with `linuxdeploy`. That tree is not snap-safe (see Decisions).
- The version of record is `version.txt`, bumped by release-please.

## Goals / Non-Goals

**Goals:**

- A strictly-confined, x86_64 snap built from the release tag and attached to
  the GitHub Release as a (unsigned) asset.
- A Release-PR dry run, with the same keep-artifact behavior as the other
  packagers.

**Non-Goals:**

- Snap Store publishing, channel management, and store credentials.
- Other package managers (Flatpak, etc.).
- Reusing the `linux-bundle` AppDir or the `install-qt-action` Qt.

## Decisions

**Base and extension: `core24` + `kde-neon-6`, `strict`.** On `core24` the
extension's build SDK and runtime content snap both carry Qt **6.11.1**, an
exact match for the app's CI Qt. Alternatives rejected: `core22` (extension pins
Qt 6.8), `core26` (the extension does not support it yet), the `gnome` extension
(GTK, not Qt), and `qt-framework` (not a Snapcraft extension — it is abandoned
content snaps from a never-merged PR). Strict confinement matches every
comparable image editor snap (GIMP, Krita, Kdenlive, Inkscape).

**Build from source inside snapcraft; do not reuse the AppDir.** The extension
supplies Qt through the `kde-qt6-core24-sdk` build snap and the `kf6-core24`
runtime content; a snap that also bundles the linuxdeploy Qt stack mixes two Qt
copies and fails at runtime on base-vs-bundled library conflicts. The `dump`
plugin could prime a prebuilt tree, but only after stripping Qt and re-targeting
it at the base — i.e. rebuilding. So the snap gets its own build path parallel
to `linux-bundle`, reusing only the non-Qt bits (`packaging/pictura.desktop`,
the icon).

**The build environment installs the pinned Rust toolchain and C++ deps.**
Ubuntu 24.04's distro Rust is older than the pinned 1.98, so the build part
installs rustup (which honours `rust-toolchain.toml`) and adds `lld`,
`ninja-build`, `liblcms2-dev`, and `libvulkan-dev` as `build-packages`. Without
this the Corrosion/Cargo build fails inside the snap.

**Confinement plugs: extension defaults + `home`, `removable-media`, `cups`.**
`kde-neon-6` auto-adds `desktop, desktop-legacy, opengl, wayland, x11,
audio-playback, unity7, network, network-bind` and pulls the `gpu-2404` content
snap, which covers Vulkan. The app adds `home` and `removable-media` because
portal remapping does not cover drag-and-drop, command-line arguments, or
"open with"; `cups` for printing. `removable-media` may need store approval
before it auto-connects, which only matters once the store is wired.

**Version: `adopt-info` on the build part.** The part runs `craftctl set version=`
reading `version.txt` (the release-please version file). The top-level
`version-script` key is deprecated in current Snapcraft, so `adopt-info` is the
supported mechanism; a literal `version:` cannot track the release bump.

**Build tool: `snapcore/action-build`.** It provisions snapd + LXD + the
snapcraft snap and runs the build in an LXD container; the workflow otherwise
has to reproduce that environment. It never publishes, so the dry run and the
tag build share it. This is the one new third-party action. It requires a VM
runner (`ubuntu-24.04` is one) and 14 GB of disk, which the job must watch.

**The pull-request trigger is gated to the Release PR, like the other packagers.**
GitHub has no head-branch filter on `pull_request`, so gating is two-part:
`on.pull_request.paths` lists the five files release-please bumps in every
Release PR (`Cargo.toml`, `CMakeLists.txt`, `CHANGELOG.md`, `version.txt`,
`.release-please-manifest.json`), and a job-level `if` accepts only the Release
PR head (`github.head_ref == 'release-please--branches--master'`). Feature PRs
never schedule a snap runner. This matches `appimage.yml` and
`linux-packages.yml`; the snap build was exercised on a temporary broad trigger
while it was being brought up, and the trigger was narrowed once it went green.

**Smoke-check by installing the built snap.** Install the `.snap` with
`snap install --dangerous` and run the headless self-test under strict
confinement. A strict failure is re-run in `--devmode` only to distinguish a
broken build from a confinement gap, then the step still fails: the strict
self-test is the contract, not an optional fallback.

## Risks / Trade-offs

- **LXD fragility / disk on hosted runners** → set `timeout-minutes`, and add a
  step that frees the preinstalled toolchains (`/opt/hostedtoolcache`, etc.)
  before building if the disk fills. Pin `snapcore/action-build@v1`.
- **Strict confinement + headless self-test on the runner** → the devmode /
  pack-only fallback above; the app already handles `SNAP`/`SNAP_NAME`.
- **The SDK's Qt Gui config needs a system dev header** → `Qt6GuiPrivate`
  requires `libxkbcommon-dev`, which the SDK does not carry; it is added as a
  `build-package`.
- **`qt-build-utils` probes qmake with a cleared environment** → it strips the
  SDK's `LD_LIBRARY_PATH`, so the SDK's qmake cannot load its Qt; the build adds
  the SDK library dirs to the loader cache (`ldconfig`) so the probe works
  without the environment.
- **Distro Rust is older than the pinned toolchain** → the build part installs
  rustup and reads `rust-toolchain.toml`; the C++ deps are `build-packages`.
- **The `kf6-core24` content snap auto-updates independently** → only forward
  compatibility (newer Qt) is guaranteed; acceptable for an unsigned asset.
- **New third-party action** → `snapcore/action-build`, justified in the
  proposal; no already-present action provisions an LXD snap build.
- **Session/workspace state under strict confinement** → the app stores state at
  `$XDG_STATE_HOME/kooka-pictura`, defaulting to `$HOME/.local/state`, which is a
  hidden path the `home` interface does not grant. Persistence across restarts
  is a separate concern (a snap-aware state root or a `personal-files` plug) and
  is tracked as a follow-up; the self-test isolates `XDG_STATE_HOME`, so the
  snap builds and self-tests regardless.

## Migration Plan

1. Land `snap/snapcraft.yaml`, `snap.yml`, the `release-please.yml` job, and the
   `DEVELOPING.md` section.
2. The pull-request dry run was exercised on a temporary broad trigger while the
   build was brought up; the trigger is now gated to the release-please Release
   PR, matching the other packagers.
3. Future change: register the snap name, add `SNAPCRAFT_STORE_CREDENTIALS`,
   and publish with `snapcore/action-publish`.
