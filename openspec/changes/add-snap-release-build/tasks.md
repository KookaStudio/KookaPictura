# Tasks

## 1. Test plan

- [x] 1.1 Write `test-plan.md` with one row per scenario in
  `specs/meta/snap-release-build/spec.md`, its Coverage column reading
  `manual:` with the concrete CI check (CI/packaging scenarios cannot be named
  by `check-test-plan.py`'s `tst_*`/`<crate>::fn` tokens); verify
  `python3 scripts/check-test-plan.py` passes.

## 2. Snap packaging

- [x] 2.1 Add `snap/snapcraft.yaml`: `name: kooka-pictura`, `base: core24`,
  `confinement: strict`, `extensions: [kde-neon-6]`, a `cmake` part building the
  app from source with `adopt-info` for the version, the `pictura` app reusing
  `packaging/pictura.desktop` with `icon: assets/icons/app.png`, and
  `plugs: [home, removable-media, cups]`; verify it parses as YAML locally and
  is exercised by the pull-request dry run (task 5.1).
- [x] 2.2 Install the pinned Rust toolchain (rustup, honouring
  `rust-toolchain.toml`) and the C++ build deps (`lld`, `ninja-build`,
  `liblcms2-dev`, `libvulkan-dev`) in the snap build environment (part
  `override-build` + `build-packages`); the pull-request dry run compiles the
  Rust static library and the C++ shell (task 5.1).

## 3. CI wiring

- [x] 3.1 Add `.github/workflows/snap.yml` modelled on `appimage.yml`:
  `workflow_call` + `workflow_dispatch` for a tag, a pull-request dry run gated
  by `paths: ['snap/**', '.github/workflows/snap.yml']`, a `concurrency` group,
  `snapcore/action-build@v1`, a `timeout-minutes` and disk-headroom step, a
  smoke-check installing the `.snap` (`--dangerous`, devmode fallback) and
  running `pictura --headless --self-test`, artifact upload on PR and
  `gh release upload ... --clobber` on tag; the pull-request dry run runs it and
  produces a `kooka-pictura_<version>_amd64.snap` (task 5.1).
- [x] 3.2 Add a `snap` job to `.github/workflows/release-please.yml` mirroring
  the `appimage` job (`needs: release-please`, `if: release_created == 'true'`,
  `uses: ./.github/workflows/snap.yml`, `with: tag: ...`); verify the file
  parses as valid workflow YAML.

## 4. Documentation

- [x] 4.1 Add a "Snap packages" section to `DEVELOPING.md` after the macOS
  section: the artifact and `strict` confinement, the release-asset publish and
  the Snap Store non-goal, the dry-run trigger, and the
  `gh workflow run snap.yml -f tag=vX.Y.Z` re-run command; also update the
  "Cutting a release" step that enumerates the packaging workflows to list all
  of them (snap, plus the currently-missing macOS/Windows); verify it names all
  five points and the enumeration is complete.

## 5. Integration verification

- [ ] 5.1 Confirm on this pull request that the dry run keeps the `.snap` as a
  workflow artifact and uploads nothing to a release.
- [x] 5.2 `openspec validate --all --strict` passes.
- [ ] 5.3 Open the phase-2 follow-up issue ("gate the snap dry-run to the
  release-please Release PR"); verify it exists and references this change.
- [ ] 5.4 Open a follow-up issue for snap state persistence under strict
  confinement (the `home` interface does not grant the app's default
  `$HOME/.local/state`), noting the snap-aware state root or `personal-files`
  options; verify it exists.
