# Test Plan

Covers every scenario in the `meta/release-build-hardening` delta. The
capability is CI/packaging behavior that `scripts/check-test-plan.py` cannot
name (its tokens are `tst_<suite>::<case>` and `<crate>::<fn>`), so every row
reads `manual:` naming the dry run, script, or artifact assertion. The new
Flatpak vendoring check is a standalone script; its row names the script and
its `--self-test` case.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `rb_concurrency_unique` | Inspect the six workflow `concurrency.group` values during the PR dry run | Each names its own workflow prefix; none is `release-please-*` | manual: GitHub Actions dry run, workflow run list |
| `rb_fork_gate` | Inspect the six job `if:` expressions and the PR-run job list | Each requires `github.event.pull_request.head.repo.full_name == github.repository` | manual: GitHub Actions dry run, workflow run list |
| `rb_pr_read_only` | Inspect the PR-run workflow permissions and jobs | Workflow token is `contents: read`; the upload job is skipped | manual: GitHub Actions dry run, run permissions page |
| `rb_actions_pinned` | `grep -n 'uses:' .github/workflows/{appimage,linux-packages,snap,flatpak,windows,macos,release-please}.yml .github/actions/*/action.yml` | Every third-party `uses:` is a 40-character SHA with a ref comment | manual: source inspection (no executable check) |
| `rb_linuxdeploy_pinned` | Read the `linux-bundle` download step; run the dry run | Downloads use release-tagged URLs and `sha256sum -c` aborts on a mismatch | manual: GitHub Actions dry run, `appimage`/`linux-packages` logs |
| `rb_win_marker` | Silent-install into a pre-populated temp directory with `setup.exe /S /D=<tmp>` | The installer rejects the non-empty unmarked directory and leaves its files | manual: GitHub Actions dry run, `windows` installer step |
| `rb_win_uninstall` | Run `uninstall.exe /S` in a directory without the marker | It reports nothing was removed, then exits before deleting registry keys | manual: GitHub Actions dry run, `windows` installer step |
| `rb_win_zip` | Extract the built portable zip and run `pictura.exe --headless --self-test` | Exit 0 with `SUMMARY passed=<n> failed=0` | manual: GitHub Actions dry run, `windows` zip smoke step |
| `rb_win_installer` | Silent-install `setup.exe`, self-test, `uninstall.exe /S`, check the directory | Self-test passes and the temporary install directory is gone | manual: GitHub Actions dry run, `windows` installer round-trip |
| `rb_win_sccache` | Run `sccache --show-stats --stats-format json` after the build | Non-zero cache writes or hits and zero cache errors | manual: GitHub Actions dry run, `windows` cache-stats step |
| `rb_macos_target` | Inspect the build log and `otool -l` minos assertion | No `built for newer 'macOS'` line; minos is 13.0 | manual: GitHub Actions dry run, `macos` build log |
| `rb_macos_offscreen` | Inspect `otool -L` on the deployed `libqoffscreen.dylib` | Qt references resolve inside the bundle | manual: GitHub Actions dry run, `macos` bundle assertion |
| `rb_macos_dmg` | Mount the built DMG and self-test the bundle from the volume | Exit 0 with a passing summary; `qt.conf` present; no runner Qt path | manual: GitHub Actions dry run, `macos` DMG smoke step |
| `rb_appimage_isolated` | Run the AppImage in `ubuntu:24.04` with `APPIMAGE_EXTRACT_AND_RUN=1` | Exit 0 with a passing summary | manual: GitHub Actions dry run, `appimage` container smoke step |
| `rb_pkg_license` | Install the `.deb` and `.rpm` in their distro containers and list `/usr/share/doc/kooka-pictura/` | `copyright` and the third-party license text are present | manual: GitHub Actions dry run, `linux-packages` smoke steps |
| `rb_pkg_appstream` | Inspect the installed `/usr/share/metainfo/pictura.metainfo.xml` | File is present and its launchable is `pictura.desktop` | manual: GitHub Actions dry run, `linux-packages` artifacts |
| `rb_pkg_version` | Compare `dpkg-deb -f` and `rpm -qp` versions for tag `vX.Y.Z` | Both report `X.Y.Z` | manual: GitHub Actions dry run, `linux-packages` artifacts |
| `rb_flatpak_metainfo` | Inspect the release-please `extra-files` and the metainfo version line | The metainfo path is listed and the line carries the version marker | manual: source inspection and release-please config review |
| `rb_flatpak_vendoring` | `python3 scripts/check-flatpak-cargo-sources.py --self-test` then the script | Self-test passes; a stale `Cargo.lock` entry makes the script fail and name it | manual: scripts/check-flatpak-cargo-sources.py stale-entry case |
| `rb_flatpak_portal` | Read XC-7 and the Flatpak workflow comment | XC-7 is PARTIAL and both name the unproven portal path (`#647`) | manual: docs review |
| `rb_snap_version` | Dispatch the snap workflow with a mismatched tag | The version assertion fails before build or upload | manual: GitHub Actions workflow_dispatch (ask first: creates no release) |
| `rb_snap_metadata` | Inspect `snap/snapcraft.yaml` | `license`, `contact`, `website`, `source-code`, and `issues` are declared | manual: source inspection |
| `rb_tag_normalization` | Dispatch a packager with `-f tag=X.Y.Z` | Checkout resolves `vX.Y.Z` and upload targets `vX.Y.Z` | manual: GitHub Actions workflow_dispatch (ask first: uploads to a release) |
| `rb_dry_run_version` | Compare the six dry-run artifact names | Each uses `0.0.0~pr<N>` | manual: GitHub Actions dry run, artifact list |
| `rb_docs_workflows` | Read "Cutting a release" in `DEVELOPING.md` | All six packagers and their artifacts are named | manual: docs review |
| `rb_docs_macos` | Read the macOS section | macOS 13 floor, unsigned artifact, and Open Anyway path are present | manual: docs review |
| `rb_docs_flatpak` | Read the Flatpak section | No "no network I/O" claim; vendored sources and regeneration are named | manual: docs review |
