# Proposal

## Why

Flatpak is the project's documented primary Linux distribution target
(`docs/GLOSSARY.md`, `docs/11-cross-cutting/gap-analysis.md`), but there is no
manifest, no CI job, and no release artifact: issue #196 asks for a Flatpak
published on release tags. This change ships the first half — a `.flatpak`
bundle built on the release tag and attached to the GitHub Release — and leaves
the store/package-manager wiring (Flathub submission, AppStream release notes
from the tag) to a later change. The feasibility unknowns that blocked a
proposal (private Qt headers, Rust toolchain vs the SDK extension, offline
cargo) are now resolved: `org.kde.Sdk//6.11` ships `Qt6::GuiPrivate` (proven by
Mixxx building on Flathub), its freedesktop base tracks 25.08 whose
`rust-stable` extension carries rustc 1.98.1, and offline cargo vendoring is a
documented pattern used by KDE's own Rust guide.

## What Changes

- Add a Flatpak manifest `flatpak/io.github.KookaStudio.KookaPictura.yml` on the
  KDE runtime (`org.kde.Platform`/`Sdk` 6.11), with the rust-stable SDK
  extension, the `cxx-qt-cmake` and `corrosion` dependencies vendored for
  `FetchContent`, and the app built `cmake-ninja`.
- Add the generated `flatpak/cargo-sources.json` so the Rust build is offline.
- Add `.github/workflows/flatpak.yml`: a cheap `lint` job
  (`flatpak-builder-lint manifest`) that runs on manifest pull requests, an
  expensive `flatpak` build job that dry-runs on the release-please Release PR
  (kept as a workflow artifact) and produces
  `KookaPictura-<version>-x86_64.flatpak` on the release tag, and a host-runner
  job that uploads the bundle to the release. The build job runs on the
  development PR while the manifest is being made green, then is gated to the
  Release PR alongside the other packaging workflows.
- Add a minimal AppStream metainfo file (`flatpak/io.github.KookaStudio.KookaPictura.metainfo.xml`)
  with the identity fields and one `<release>` entry; the metainfo, desktop
  entry, and icon are installed under the app-id so the app registers under
  Flatpak.
- Wire the tag build into `.github/workflows/release-please.yml`.
- Document the Flatpak packaging flow in `DEVELOPING.md`.

## Capabilities

### New Capabilities

- `meta/flatpak-release-build`: how the repository builds and publishes the
  x86_64 Flatpak bundle from a release tag — the artifact and its release
  attachment, the KDE-runtime build and its self-test, the sandbox `finish-args`,
  the release-keyed workflow with its Release-PR dry run, and the documentation
  requirement.

### Modified Capabilities

<!-- None. `meta/release-please` already names flatpak as a packaging trigger;
     this change does not alter its requirements. -->

## Impact

- **CI**: one new workflow; a new job in `release-please.yml`. New third-party
  dependency: `flatpak/flatpak-github-actions/flatpak-builder@v6` (pinned to the
  v6 major) run inside the Flathub container image
  `ghcr.io/flathub-infra/flatpak-github-actions:kde-6.11`, and
  `flathub-infra/flatpak-builder-lint` for the lint job. Stated because AGENTS
  rule 4 requires a reason: no existing action can run `flatpak-builder`, and
  the container supplies the runtime/SDK that the action deliberately does not
  install itself.
- **Packaging**: new `flatpak/` directory (manifest, generated cargo sources,
  metainfo). No application code changes; the private-header and Rust-toolchain
  concerns the offline memo raised are handled by the SDK and the existing
  `Qt6::GuiPrivate` link.
- **Docs**: a "Flatpak packages" section in `DEVELOPING.md`.
- **Specs**: `meta/flatpak-release-build` added; `meta/release-please` untouched.
- **Dependencies / app code**: none.
- **Out of scope**: Flathub submission and tag-derived AppStream release notes
  (the "store wiring"), and aligning the CMake Windows/macOS bundle identifier
  (`io.github.KookaPictura.KookaPictura`, not reachable as a code-hosting id)
  with the Flatpak app-id `io.github.KookaStudio.KookaPictura`.
