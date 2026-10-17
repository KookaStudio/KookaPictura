# Proposal

## Why

#703/#710 hardened the six release packagers and updated `README.md`,
`DEVELOPING.md`, and the parity register, but a few documentation surfaces still
describe the pre-#703 state. The Flatpak capability spec also claims the build
performs no network I/O at all, which contradicts how `flatpak-builder` fetches
its declared manifest sources before the sandboxed compile runs offline.

## What Changes

- Reword `meta/flatpak-release-build` requirement "The application builds
  offline against the KDE runtime": `flatpak-builder` fetches the sources
  declared in the manifest, and only the compile phase is offline. Update the
  matching scenario.
- Check off the stale `ROADMAP.md` installers item and name the shipped
  artifacts: AppImage, `.deb`/`.rpm`, snap, Flatpak, Windows portable/installer,
  and macOS DMG.
- Add the #703/#710 release-build-hardening entry to `docs/dev/STATE.md`,
  including the archived `release-build-hardening` change and the temporary
  dry-run widening that was reverted before merge.
- Mark `docs/dev/flatpak-offline-memo.md` superseded by the shipped manifest and
  the packaging sections of `DEVELOPING.md`.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `meta/flatpak-release-build`: the offline-build requirement and its scenario
  now distinguish the manifest fetch from the offline compile.

## Impact

- **Specs**: `openspec/specs/meta/flatpak-release-build/spec.md` (via this
  change's delta).
- **Docs**: `ROADMAP.md`, `docs/dev/STATE.md`, `docs/dev/flatpak-offline-memo.md`.
- **Code/CI**: none. This is a docs and spec wording correction; the shipped
  workflows, manifest, and packaging scripts are unchanged.
