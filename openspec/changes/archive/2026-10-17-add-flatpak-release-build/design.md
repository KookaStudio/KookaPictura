# Design

## Context

See `proposal.md` - Why. The constraints that shape the approach:

- The repo already builds every other release artifact from a tag through a
  small workflow (`appimage.yml`, `linux-packages.yml`, `windows.yml`,
  `macos.yml`), each keyed off the release by `release-please.yml`, runnable by
  `workflow_dispatch`, and dry-run on the release-please Release PR. Flatpak
  should look like them.
- `CMakeLists.txt` already requires `Qt6::GuiPrivate` (`:49`) and links it
  (`:470`), and fetches `cxx-qt-cmake` 0.10.0 through `FetchContent` when
  `CxxQt` is not found (`:54-63`). The memo's private-header fallback is
  therefore unnecessary. There are **three** network fetches the manifest must
  satisfy: the crates, `cxx-qt-cmake`, and `corrosion-rs/corrosion` 0.5.2,
  which `cxx-qt-cmake` itself fetches.
- `org.kde.Sdk//6.11` tracks freedesktop-sdk 25.08 and retains Qt's private
  headers and CMake target files (the Platform strips them). Its
  `org.freedesktop.Sdk.Extension.rust-stable` branch carries rustc 1.98.1,
  satisfying the workspace's `rust-version = "1.98"`. The extension ships plain
  `cargo` and ignores `rust-toolchain.toml`; that is acceptable here.
- `flatpak/flatpak-github-actions` publishes only the `flatpak-builder` and
  `flat-manager` sub-actions; there is no lint action. It deliberately does not
  install flatpak, flatpak-builder, the runtime, or `gh`, so the build runs
  inside the Flathub container image and the release upload is a separate
  host-runner job.

## Goals / Non-Goals

**Goals:**
- A reproducible `.flatpak` release asset, built offline against the KDE
  runtime, with the existing self-test as its correctness check.
- A CI shape that mirrors the other packaging workflows: cheap validation on
  manifest pull requests, a full dry run on the Release PR, an upload on the
  tag.
- A manifest already shaped for a later Flathub submission (valid code-hosting
  app-id, minimal `finish-args`, metainfo present) without paying the
  store-wiring cost now.

**Non-Goals:**
- Submitting to Flathub, `flathub.json`, screenshots, or generating AppStream
  release notes from the tag.
- Multi-architecture (aarch64) builds.
- Changing the CMake Windows/macOS bundle identifier.

## Decisions

**Bundle as a release asset, not Flathub yet.** Flathub builds server-side, so
wiring it would remove the CI build entirely; the bundle path exercises the
whole offline build now and matches the other four workflows. Flathub becomes a
separate change.

**App-id `io.github.KookaStudio.KookaPictura`.** Flathub requires a code-hosting
id of the form `io.github.<owner>.<repo>` and its linter's
`appid-url-not-reachable` check is a never-granted exception; the id must derive
`https://github.com/KookaStudio/KookaPictura`. The existing CMake value
`io.github.KookaPictura.KookaPictura` derives an unreachable URL and is left
untouched (it is a private bundle identifier with no Flathub role); aligning it
is a separate concern.

**Manifest in `flatpak/`, offline sources committed.** `flatpak/io.github.KookaStudio.KookaPictura.yml`
plus `flatpak/cargo-sources.json` from `flatpak-cargo-generator.py` over the
committed `Cargo.lock`. Because the manifest sits in `flatpak/`, the app's
`type: dir` source uses `path: ..` (a `dir` source path is resolved relative to
the manifest root). The app module uses `cmake-ninja` and declares three
sources: `cargo-sources.json`, a vendored `cxx-qt-cmake` 0.10.0 archive, and a
vendored `corrosion` 0.5.2 archive, each with `dest:` into the module build
directory. The app passes `-DFETCHCONTENT_SOURCE_DIR_CXXQT=<dir>`,
`-DFETCHCONTENT_SOURCE_DIR_CORROSION=<dir>`, and
`-DFETCHCONTENT_FULLY_DISCONNECTED=ON`, so neither `FetchContent` call can reach
the network. Alternative considered: install `cxx-qt-cmake`/`Corrosion` so
`find_package` succeeds — rejected because vendoring keeps the manifest close to
the existing build and needs no CMake changes.

**The artifact name is produced, not assumed.** The `flatpak-builder` action's
`bundle` input defaults to `app.flatpak`, so the build runs with
`upload-artifact: false` and a following step renames the bundle to
`KookaPictura-<version>-x86_64.flatpak` before any upload, using the same
`VERSION` derivation as `linux-packages.yml`.

**One workflow, two jobs, plus a host upload job.** `flatpak.yml` has a cheap
`lint` job (`flatpak-builder-lint`'s `manifest` and `appstream` checks, run
through the linter's digest-pinned container image) and an expensive `flatpak`
build job. Unlike the
other packaging workflows, the workflow's `paths` are not narrowed to the five
release-please files: the lint job must run on manifest pull requests, so only
the build job carries the head-branch gate:

```yaml
if: github.event_name != 'pull_request'
    || github.head_ref == 'release-please--branches--master'
```

The five release-please files are still added to `paths` so the Release PR
schedules the workflow and the dry run fires. The upload to the release is a
separate `runs-on: ubuntu-24.04` job that downloads the build artifact and runs
`gh release upload "$TAG" ... --clobber`, because the container image has no
`gh`; the workflow sets `permissions: contents: write`.

**Development order: build ungated, then gate.** The manifest can only be
validated by a real build, and `workflow_dispatch --ref <branch>` checks out the
tag's tree, so the build job runs ungated on this development PR until it is
green; the final commit applies the gate above. The gated config is identical to
`appimage.yml`'s, so the only untested element is the gate expression itself.

**Action, container, cache.** `flatpak/flatpak-github-actions/flatpak-builder@v6`
in `ghcr.io/flathub-infra/flatpak-github-actions:kde-6.11` with
`options: --privileged`; the image pre-adds the Flathub remote and the action
installs the runtime from the manifest. Cache uses the action default (keyed on
the manifest, never `github.sha`); `save-cache: false` is the lever if
development-run caches churn the 10 GB store. Both jobs set `timeout-minutes`.

**Self-test as the check.** After the build the job runs
`flatpak install --user -y --noninteractive` on the produced bundle and then
`dbus-run-session -- flatpak run --env=QT_QPA_PLATFORM=offscreen io.github.KookaStudio.KookaPictura --headless --self-test`.
The action's `run-tests` input is **not** used: it rewrites the manifest to grant
network and X11, contradicting the no-network and minimal-sandbox requirements.

**Minimal static metainfo, installed.** An
`io.github.KookaStudio.KookaPictura.metainfo.xml` with identity fields, a
`launchable`, and one `<release>` entry for the current version, installed into
`/app/share/metainfo/` by the manifest (the CMake install rules do not carry
it). Release-note generation from the tag is deferred with the Flathub work; the
static `<release>` is bumped by hand until then.

**Sandbox permissions** are display, GPU, and CUPS only: `--share=ipc`,
`--socket=wayland`, `--socket=fallback-x11`, `--device=dri`, `--socket=cups`.
No `--filesystem=xdg-*` grants are added: the sandbox already gives the app
writable per-app XDG config/cache/data directories, so those grants would widen
access to the host directories, not the per-app ones. This narrows the proposal
in `docs/11-cross-cutting/security-and-sandboxing.md`; nothing broader is
requested.

**Offline enforcement is inherent.** flatpak-builder fetches the manifest's
declared sources before the build and then runs the build sandbox without
network, so an undeclared fetch (for example a `FetchContent` clone) fails the
build; the `SOURCE_DIR` overrides remove the two CMake fetches from that path.

## Risks / Trade-offs

- **Three vendored fetches must all be satisfied.** → Vendor `cxx-qt-cmake` and
  `corrosion` as module sources with pinned `sha256` and point both
  `FETCHCONTENT_SOURCE_DIR_*` variables at them; the first dry run confirms
  configure finds neither the network nor a missing source.
- **`Qt6::GuiPrivate` is a hard requirement** (`CMakeLists.txt:49`), so a SDK
  without it fails configure outright, unlike the qmake/lcms2 cases. → The
  first dry run confirms the SDK ships it (Mixxx builds against it on Flathub);
  if not, the fallback is to install Qt's private dev module into the SDK, or
  gate `interop.cpp` per the memo's option 2. Decision recorded, not guessed.
- **`qmake6` may not be on the SDK PATH.** → The first dry run surfaces it;
  fallback is to set `QMAKE` explicitly or install the Qt tools package.
- **`lcms2` may not be found by `find_library`.** → The SDK ships Little CMS;
  if the lookup misses, add a small module. Dry run confirms.
- **`Cargo.lock` lags the Release PR's version bump**, so `cargo-sources.json`
  and the lock may disagree at the tag. → The build must not assume `--locked`
  (consistent with the other packaging workflows); the Release-PR dry run is the
  check.
- **Third-party action / container dependency.** → Pin the action to the `v6`
  major and the container to `kde-6.11`; the dependency is stated in the
  proposal per AGENTS rule 4.
- **The static `<release>` version drifts** because it is not one of
  release-please's version files. → Documented as manual until the
  release-notes-from-tag change.

## Migration Plan

1. Land the manifest, sources, metainfo, and workflow with the build job
   ungated; iterate on the development PR until the lint job and the dry-run
   build are green.
2. Apply the build-job gate, the release-please wiring, and the upload job; the
   config then matches the other packaging workflows.
3. Archive the change (`openspec archive add-flatpak-release-build`) before the
   PR ships, per the zero-open rule.
4. Rollback is deleting the workflow's jobs (or reverting the change); no
   application code depends on it.
