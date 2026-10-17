# Tasks

## 1. Offline dependency sources

- [x] 1.1 Generate `flatpak/cargo-sources.json` by running
  `flatpak-cargo-generator.py Cargo.lock -o flatpak/cargo-sources.json`
  (`flatpak/flatpak-builder-tools`) and commit it. Verify it is valid JSON that
  lists the workspace's crates (for example `grep -q cxx-qt flatpak/cargo-sources.json`)
  and is regenerated only from the committed `Cargo.lock`.
- [x] 1.2 Pin both CMake-fetched dependencies as vendored sources: record the
  `cxx-qt-cmake` 0.10.0 download URL and `sha256` (matching the
  `GIT_TAG 0.10.0` in `CMakeLists.txt:60`) and the `corrosion` 0.5.2 source that
  `cxx-qt-cmake` fetches (upstream tag plus commit). Verify both are present as
  manifest sources in task 2.2 and neither is left to `FetchContent`'s network
  path.

## 2. Flatpak manifest

- [x] 2.1 Add `flatpak/io.github.KookaStudio.KookaPictura.yml`: app-id, the
  `org.kde.Platform`/`org.kde.Sdk` 6.11 runtime, the
  `org.freedesktop.Sdk.Extension.rust-stable` SDK extension with
  `append-path: /usr/lib/sdk/rust-stable/bin`, `command: pictura`, and the
  minimal `finish-args` (`--share=ipc`, `--socket=wayland`,
  `--socket=fallback-x11`, `--device=dri`, `--socket=cups`; no `--filesystem`
  and no `--socket=session-bus`). Verify
  `flatpak-builder-lint manifest` reports no never-granted violations.
- [x] 2.2 Add the `pictura` module: `buildsystem: cmake-ninja`; `sources` =
  `cargo-sources.json`, the `cxx-qt-cmake` archive with `dest:`, the `corrosion`
  archive with `dest:`, and `type: dir path: ..` (the app source tree, since the
  manifest sits in `flatpak/`); `config-opts`
  `-DFETCHCONTENT_SOURCE_DIR_CXXQT=<dest>`,
  `-DFETCHCONTENT_SOURCE_DIR_CORROSION=<dest>`, and
  `-DFETCHCONTENT_FULLY_DISCONNECTED=ON`; `build-options.env`
  `CARGO_HOME: /run/build/pictura/cargo` and `CARGO_NET_OFFLINE: "true"`. Verify
  the module references every source and that both `SOURCE_DIR` paths point at
  directories that exist in the module build dir.
- [x] 2.3 Ensure cargo finds the generated configuration under the Corrosion
  build: append the generator's `cargo/config` to `.cargo/config.toml` in the
  source tree (Corrosion may not pick up `CARGO_HOME`), preserving the
  repository's fast-linker rustflags. Verify the dry-run build (task 4.2)
  configures and compiles with no network access, and the configure log shows no
  `FetchContent` clone.

## 3. Application registration

- [x] 3.1 Add `flatpak/io.github.KookaStudio.KookaPictura.metainfo.xml` with the
  identity fields (`id` equal to the app-id, `name`, `summary`,
  `metadata_license`, `project_license`, `launchable` pointing at the app-id
  desktop entry) and one `<release>` entry carrying the current version and a
  date. Verify `flatpak-builder-lint appstream flatpak/io.github.KookaStudio.KookaPictura.metainfo.xml`
  passes.
- [x] 3.2 Install the app-id-named registration files into the bundle: the
  metainfo under `${FLATPAK_DEST}/share/metainfo/`, and app-id-named copies of
  the CMake-installed `pictura.desktop` and `pictura.png` (with `Icon=`/`Name`
  matching the app-id) under `share/applications/` and the hicolor icon dir,
  without changing the shared `packaging/pictura.desktop` used by the native
  packages. Verify by inspecting the installed tree in the dry run
  (`/app/share/metainfo/io.github.KookaStudio.KookaPictura.metainfo.xml`,
  `/app/share/applications/io.github.KookaStudio.KookaPictura.desktop`, and an
  app-id-named icon all exist).

## 4. Flatpak CI workflow

- [x] 4.1 Add `.github/workflows/flatpak.yml` with the `lint` job: run
  flathub's `flatpak-builder-lint` (container
  `ghcr.io/flathub/flatpak-builder-lint`, digest-pinned) over `manifest` and
  `appstream`, triggered by `on.pull_request.paths` covering the Flatpak files.
  Verify the lint job passes on this PR.
- [x] 4.2 Add the `flatpak` build job: `flatpak/flatpak-github-actions/flatpak-builder@v6`
  in `ghcr.io/flathub-infra/flatpak-github-actions:kde-6.11` with
  `options: --privileged`, `manifest-path` the manifest, `build-bundle: true`,
  `upload-artifact: false`; a following step renames the bundle to
  `KookaPictura-<version>-x86_64.flatpak` (`VERSION` derived as in
  `linux-packages.yml:43`); then a smoke step
  `flatpak install --user -y --noninteractive` the bundle and
  `dbus-run-session -- flatpak run io.github.KookaStudio.KookaPictura --headless --self-test`.
  Add `workflow_call`/`workflow_dispatch` with the `tag` input, the
  `concurrency` group, `permissions: contents: write`, and `timeout-minutes`.
  Verify the build job is green on this PR and its self-test passes.
- [x] 4.3 Add a `runs-on: ubuntu-24.04` upload job that `needs` the build job,
  runs only when the event is not a pull request, downloads the bundle artifact,
  and runs `gh release upload "$TAG" KookaPictura-*-x86_64.flatpak --clobber`
  with `GH_REPO` set. Verify the job is skipped on the PR and its command matches
  the artifact name from 4.2.

## 5. Documentation

- [x] 5.1 Add a "Flatpak packages" section to `DEVELOPING.md`: the artifact
  name and app-id, the KDE 6.11 runtime and x86_64 architecture, the trigger
  paths (release, dispatch, Release-PR dry run, PR lint), the re-run command for
  an existing tag, the no-embedded-runtime caveat, and that Flathub and
  release-notes-from-tag are not yet wired. Verify the documented commands and
  paths match the workflow.

## 6. Gate the dry run to the Release PR

- [x] 6.1 Once the lint and build jobs are green, gate the build job:
  `if: github.event_name != 'pull_request' || github.head_ref == 'release-please--branches--master'`,
  and extend `on.pull_request.paths` with the five release-please files
  (`Cargo.toml`, `CMakeLists.txt`, `CHANGELOG.md`, `version.txt`,
  `.release-please-manifest.json`) so the Release PR schedules the workflow.
  Verify the workflow YAML parses and matches the `appimage.yml` gate.
- [x] 6.2 Wire the tag build into `.github/workflows/release-please.yml`: add a
  `flatpak` job that `uses: ./.github/workflows/flatpak.yml` with
  `tag: ${{ needs.release-please.outputs.tag_name }}`, gated on
  `needs.release-please.outputs.release_created == 'true'`. Verify
  `openspec validate --all --strict` and the guard scripts pass.
- [x] 6.3 Archive the change with `openspec archive add-flatpak-release-build`
  before the PR ships (zero-open rule). Verify `scripts/check-openspec-archived.sh`
  passes.
