# Tasks

## 1. Test plan

- [ ] 1.1 Write `test-plan.md` with one row per scenario in
  `specs/meta/release-build-hardening/spec.md`; CI/packaging scenarios read
  `manual:` naming the concrete dry run or check, because
  `check-test-plan.py` only names Qt/Rust tests; verify
  `python3 scripts/check-test-plan.py` passes.

## 2. Workflow safety and pins

- [ ] 2.1 Give all six packagers a unique `concurrency.group` and a fork gate
  that requires the pull request head repository to be this repository.
- [ ] 2.2 Move `contents: write` off the pull-request path: workflow-level
  `contents: read` plus a separate upload job gated to non-pull-request runs.
- [ ] 2.3 Extend the pull-request `paths` filters with the workflow, composite
  action, packaging files, and `Cargo.lock` where relevant.
- [ ] 2.4 Pin every third-party action in the release pipeline to a commit SHA,
  and pin the linuxdeploy/Qt-plugin downloads with SHA256, sccache v0.18.0, and
  `create-dmg` v1.3.0.
- [ ] 2.5 Normalize the tag input (with or without `v`) in all six workflows and
  use the `0.0.0~pr<N>` dry-run fallback consistently.

## 3. Windows

- [ ] 3.1 Replace the sentinel with an install marker, reject non-empty
  unmarked install directories, and quit before `remove_reg` when the marker is
  absent; add VERSIONINFO.
- [ ] 3.2 Remove the duplicate VC runtime copy, stop swallowing the offscreen
  plugin copy error, and add `if-no-files-found: error`.
- [ ] 3.3 Extract the portable zip and silently install/uninstall `setup.exe`,
  running the self-test from each and clearing Qt plugin variables.

## 4. macOS

- [ ] 4.1 Export `MACOSX_DEPLOYMENT_TARGET=13.0`, fail on the newer-macOS
  linker warning, assert `otool -l` minos 13.0, deploy the offscreen plugin
  with rewritten references, and ad-hoc sign the bundle.
- [ ] 4.2 Build the DMG first, mount it, self-test from the volume with
  `DYLD_*` unset, assert `qt.conf` and bundle-relative `otool -L`, and detach;
  remove the dead `--app-drop-link` and pin `create-dmg`.

## 5. Linux

- [ ] 5.1 Run the AppImage self-test inside a clean `ubuntu:24.04` container.
- [ ] 5.2 Add license/third-party files and native AppStream metainfo to
  `.deb`/`.rpm`, and remove the RPM version rewrite.
- [ ] 5.3 Guard the desktop/icon install rules with `if(NOT WIN32)`.

## 6. Flatpak and snap

- [ ] 6.1 Add the Flatpak metainfo to release-please `extra-files` and mark the
  version line.
- [ ] 6.2 Add `scripts/check-flatpak-cargo-sources.py` (with `--self-test`) and
  run it in the Flatpak lint job; reference `#647` for the unproven portal.
- [ ] 6.3 Assert the snap tag matches `version.txt`, allow `KOOKA_VERSION`, and
  add license/contact/website/source-code/issues metadata.

## 7. Documentation and verification

- [ ] 7.1 Update `DEVELOPING.md` (all six packagers and artifacts, macOS 13 and
  unsigned Open Anyway path, Flatpak network wording) and
  `docs/dev/doc-code-parity-gaps.md` XC-7.
- [ ] 7.2 Run all six packagers as pull-request dry runs, then revert only the
  temporary trigger widening.
- [ ] 7.3 `openspec validate --all --strict`, `scripts/check-openspec-archived.sh`,
  `scripts/guard.sh`, `scripts/check-test-plan.py`, and the file-size gate pass.
- [ ] 7.4 Archive `release-build-hardening`.
