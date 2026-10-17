# Proposal

## Why

Six release build workflows (`appimage`, `linux-packages`, `snap`, `flatpak`,
`windows`, `macos`) have only ever run as pull-request dry runs. Before 0.2.0
they must be safe to run for a real tag. An audit found one data-loss bug
(the Windows uninstaller deletes unrelated files), one distribution blocker
(the unsigned macOS DMG is stopped by Gatekeeper with no working documented
path), and systemic gaps: every packager resolves to the caller's concurrency
group, the fork gate accepts a forged head branch, build tools are downloaded
from mutable URLs, and most packaged artifacts are never executed in the form
users receive them.

## What Changes

- **Workflow safety**: unique concurrency groups per packager, a fork gate that
  requires the PR head repository to be this repository, read-only permissions
  on PR dry runs (release uploads move to a separate job with `contents: write`),
  and pull-request path filters that include the workflow, composite action,
  packaging files, and `Cargo.lock` where relevant.
- **Supply chain**: pin third-party actions to commit SHAs, pin the linuxdeploy
  and Qt-plugin downloads to release tags with SHA256 checks, pin sccache to a
  GitHub-Actions-cache-v2-capable release, and pin `create-dmg`.
- **Windows**: write an install marker and require it before removing the
  install directory, reject a non-empty install directory, stop the uninstaller
  from deleting registry state when it refuses to remove files, add VERSIONINFO,
  remove the duplicate VC runtime copy, and test the built `.zip` and the
  silent-installed `setup.exe` end to end.
- **macOS**: export the 13.0 deployment target to Cargo/cxx-qt, fail on the
  newer-macOS linker warning, verify the offscreen plugin's framework
  references, ad-hoc sign the bundle, and run the self-test from the mounted
  DMG. Document the unsigned artifact and the real Gatekeeper bypass path.
- **Linux**: run the AppImage self-test in a clean container, add a
  copyright/license file and AppStream metainfo to the `.deb`/`.rpm`, stop the
  RPM version rewrite, and install the desktop file/icon only on non-Windows.
- **Flatpak**: keep the metainfo release version in sync through
  release-please, add a cheap `Cargo.lock`/`cargo-sources.json` drift check, and
  record that portal confinement remains unproven (`#647` is partial).
- **Snap**: assert the tag matches `version.txt` before upload and declare the
  license, contact, website, and source-code metadata.
- **Versions and docs**: normalize the tag input in all six workflows, use one
  dry-run version fallback, and update `DEVELOPING.md` and the parity register
  to match what ships.

**Non-goals**: code signing and notarization (`#704`), macOS Intel/universal
(`#705`), Linux aarch64 (`#706`), Snap Store publishing (`#707`), Flathub
submission (`#708`), and Homebrew/winget/scoop/AUR (`#709`).

## Capabilities

### New Capabilities

- `meta/release-build-hardening`: the release packagers are safe to re-run for
  a tag, pinned and fork-gated, and each artifact is verified in the form users
  receive it.

### Modified Capabilities

<!-- None: the per-platform capabilities keep their existing requirements. This
     change adds cross-cutting hardening and verification requirements under a
     new capability. -->

## Impact

- **CI**: six workflows, three composite actions, `release-please.yml`,
  `release-please-config.json`.
- **Packaging**: `packaging/windows/installer.nsi`, `packaging/build-packages.sh`,
  a new native AppStream metainfo, `snap/snapcraft.yaml`, the Flatpak metainfo.
- **Build**: `CMakeLists.txt` install rules.
- **Scripts**: one new dependency-free check for the Flatpak vendored crates.
- **Docs**: `DEVELOPING.md`, `docs/dev/doc-code-parity-gaps.md`.
- **Follow-ups**: `#704`–`#709` as listed above; no new dependencies.
