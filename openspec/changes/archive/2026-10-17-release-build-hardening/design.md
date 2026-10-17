# Design

## Context

See `proposal.md`. The six packagers already share a shape: a `pull_request`
dry run gated by `paths` and a head-branch `if`, `workflow_call` from
`release-please.yml`, and `workflow_dispatch` for an existing tag. This change
keeps that shape and closes the gaps found by auditing the shell, the packaging
scripts, and the composite actions.

Two constraints drive most decisions:

- There is no Apple Developer account, so no signing or notarization is in
  scope; the DMG is unsigned and must be documented as such.
- Builds are serialized on the developer machine. CI edits are verified by
  running all six packagers as pull-request dry runs, not by a local packaging
  build.

## Goals / Non-Goals

**Goals:**

- Fix the Windows data-loss bug and the macOS first-launch blocker.
- Make every packager's concurrency key, fork gate, and token permissions
  correct.
- Pin every downloaded build input and every third-party action.
- Exercise the artifact users download, not only the build tree.

**Non-Goals:**

- Signing/notarization, additional architectures, store publishing, and
  package-manager manifests.
- Refactoring the workflows into a shared generator. Six small, explicit YAML
  files are easier to audit and to dry-run independently.

## Decisions

**Unique concurrency groups use the workflow file's name.** In a reusable
workflow `github.workflow` is the caller's name, so all six resolved to
`release-please-refs/heads/master`, the same group as `release-please.yml`.
Each workflow now uses a literal prefix (`appimage-`, `linux-packages-`, …) plus
`inputs.tag || github.ref`. This is boring and cannot collide with the caller or
with a sibling packager.

**The fork gate checks the head repository.** `github.head_ref` is
attacker-controlled: a fork can name its branch `release-please--branches--master`.
The job condition now also requires
`github.event.pull_request.head.repo.full_name == github.repository`, so only a
branch in this repository can run a privileged packager.

**PR dry runs are read-only.** `contents: write` is removed from the workflow
level of the five workflows that still had it. Each gets a small `upload` job
that depends on the build job, runs only when `github.event_name != 'pull_request'`,
and declares `permissions: contents: write`. `flatpak.yml` already had this
shape. The build jobs upload their artifacts unconditionally (with
`if-no-files-found: error`) so the upload job can download them; this costs a
few days of artifact retention but keeps the two paths identical.

**Tags are normalized in the checkout expression.** `workflow_dispatch` inputs
are user-entered, so `-f tag=0.2.0` previously failed at checkout. Every
checkout uses
`startsWith(inputs.tag, 'v') && inputs.tag || format('v{0}', inputs.tag)`,
falling back to `github.sha` for a pull request. Upload jobs use the same
expression for `RELEASE_TAG`. The dry-run version is `0.0.0~pr<N>` everywhere;
the snap reads it from `KOOKA_VERSION` when set and otherwise from
`version.txt`.

**Windows uninstall safety is a marker, not a sentinel.** The installer writes
`$INSTDIR\.kooka-pictura-install` after staging the files. The uninstaller
requires that marker before `RMDir /r "$INSTDIR"`. If it is absent it shows the
message and quits, so the uninstall registry keys and shortcuts stay intact.
`.onVerifyInstDir` rejects a non-empty directory unless it already carries the
marker, which also protects an in-place upgrade.

**The installer round-trip is tested.** The workflow extracts the portable zip
and runs the self-test from the extracted folder, then installs `setup.exe`
silently into a temporary directory, runs the self-test there, uninstalls, and
asserts the directory is gone. This exercises the two archives and the
uninstaller, which the staged-folder smoke test never touched.

**macOS uses `-always-overwrite` for the offscreen plugin.** `macdeployqt`
skips existing bundle files, so the pre-copied `libqoffscreen.dylib` kept its
Qt install paths. Passing `-always-overwrite` makes macdeployqt rewrite the
plugin's framework references. The build exports `MACOSX_DEPLOYMENT_TARGET=13.0`
(as well as the CMake variable), fails when the linker warns
`built for newer 'macOS'`, and asserts the main binary's `minos` is 13.0. The
bundle is ad-hoc signed (`codesign --sign -`) after deployment, which is
required on Apple Silicon and does not imply notarization.

**The DMG self-test mounts the image.** The old check ran
`build/pictura.app` before the DMG existed and left `DYLD_*` in the
environment. The new check attaches the built DMG at a fixed mount point, runs
the bundle from the volume with `env -u` clearing `QT_PLUGIN_PATH`,
`QML2_IMPORT_PATH`, and the `DYLD_*` variables, checks `qt.conf`, inspects
`otool -L` for bundle-relative references, and detaches the image.

**The AppImage self-test runs in a clean container.** The build runner has the
`install-qt-action` Qt on `LD_LIBRARY_PATH`, which can mask a missing bundled
library. The AppImage is run inside `ubuntu:24.04` with
`APPIMAGE_EXTRACT_AND_RUN=1` and the artifact directory mounted read-only, so
only the bundled libraries are reachable.

**Flatpak drift is a structural check, not a regeneration.** Regenerating
`cargo-sources.json` needs network and a Python tool that is not vendored. A
small script parses `Cargo.lock` and `cargo-sources.json` and fails when a
locked crate or version is missing from the sources. It runs in the Flatpak
lint job on every pull request that touches the manifest or lockfile. A full
regenerate remains the documented manual step.

**Portal confinement is documented, not faked.** The smoke test cannot prove a
portal-backed dialog without a portal implementation in the runner. The parity
register moves XC-7 to PARTIAL and names the remaining portal gap (`#647`);
the workflow comment records the same ceiling.

**Actions are pinned to commit SHAs.** Tags and branches move. The pins are the
current commit for each tag/branch used before this change, with the human ref
in a comment. `install-qt-action` has no `v4` tag (it is a branch), so its pin
is the branch head at the time of this change.

## Risks / Trade-offs

- **Pinning actions freezes known-good versions** → the pins are updated
  deliberately, not by a moving tag; Dependabot can propose bumps later.
- **The Windows installer runs as admin in CI** → the runner is already
  administrator; the install target is a unique temporary directory so a
  failed run cannot leave a shared path modified.
- **`cache_writes > 0` can fail on a fully cached run** → the assertion also
  accepts cache hits, which proves the backend is reachable; the first run after
  the sccache version pin establishes the cache.
- **Ad-hoc signing is not notarization** → the documented Gatekeeper path is
  still required; signing is `#704`.
- **The vendored-crate check is structural** → it catches a stale name/version,
  not a changed checksum. The documented regenerate step covers checksums.
- **Temporary PR widening** → used only to exercise the six dry runs, then
  reverted in a separate commit before merge.

## Migration Plan

1. Land the workflow, action, packaging, build, and script changes.
2. Land the docs update and the parity-register change.
3. Exercise all six packagers as pull-request dry runs.
4. Revert only the temporary trigger widening, then archive the change.
