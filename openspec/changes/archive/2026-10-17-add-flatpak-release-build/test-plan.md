# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

Everything here is CI/package behaviour: there is no Rust or Qt unit under test,
so the rows are `manual:` with the CI job that exercises them. The one runtime
check is the application self-test, run from the installed Flatpak.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A release is cut | Merge the release-please Release PR; inspect the created GitHub Release | The release carries `KookaPictura-X.Y.Z-x86_64.flatpak` | manual: tag-time release asset; no local oracle |
| The build is re-run for an existing tag | `gh workflow run flatpak.yml -f tag=vX.Y.Z`; inspect the release | The bundle asset is rebuilt and replaced (`--clobber`) | manual: `workflow_dispatch` path |
| The application builds from manifest sources only | Run the `flatpak` build job with the build sandbox network disabled | `pictura` is produced; no network I/O during the build | manual: exercised by the `flatpak.yml` build job in CI |
| CMake fetches nothing over the network | Inspect the configure log of the build job | `CxxQt` and `Corrosion` resolve from the vendored sources; no clone of `cxx-qt-cmake` or `corrosion` | manual: configure-log check in the `flatpak.yml` build job |
| The self-test passes from the bundle | `flatpak install --user` the bundle, then `flatpak run io.github.KookaStudio.KookaPictura --headless --self-test` | Exit 0, `pictura self-test: SUMMARY passed=<n> failed=0` | manual: the `flatpak.yml` build job's smoke step runs this; no named test hook |
| The application is registered on install | Install the bundle; inspect the installed tree and exported files | The app-id is listed; metainfo, desktop entry, and icon are installed under the app-id | manual: install-tree inspection in the build job |
| The bundle records its runtime repository | Inspect the built bundle's metadata | It records the repository supplying `org.kde.Platform//6.11` | manual: bundle metadata inspection in the build job |
| The declared permissions are minimal | Inspect the manifest `finish-args` | `--share=ipc`, `--socket=wayland`, `--socket=fallback-x11`, `--device=dri`, `--socket=cups`; no `--filesystem` and no `--socket=session-bus` | manual: manifest inspection; `flatpak-builder-lint manifest` flags violations |
| release-please triggers the build | Cut a release; watch `release-please.yml` | The `flatpak` job is called with the release tag | manual: workflow-run inspection |
| The Release PR dry-runs | Open/update the release-please Release PR | The build runs, the bundle is kept as a workflow artifact, nothing is uploaded | manual: workflow-run inspection |
| A feature pull request lints but does not build | Open a PR touching `flatpak/**` | The lint job runs; the build job is skipped | manual: workflow-run inspection |
| A maintainer reads the docs | Read the Flatpak section of `DEVELOPING.md` | Artifact/app-id, runtime/arch, triggers, re-run command, and the no-embedded-runtime caveat are present | manual: documentation review |
