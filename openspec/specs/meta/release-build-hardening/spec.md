# meta/release-build-hardening Specification

## Purpose
Defines how the six release packagers are made safe to re-run, pinned and
fork-gated, and how each shipped artifact is verified in the form users
receive it.

## Requirements

### Requirement: Release workflows are uniquely grouped, fork-gated, and read-only on pull requests

Each packager workflow SHALL use a concurrency group unique to that workflow so
a release does not share a group with `release-please.yml` or a sibling
packager. A pull-request dry run SHALL be accepted only when the pull request
comes from this repository and its head branch is the release-please Release
PR. The pull-request path SHALL NOT hold `contents: write`; release uploads
SHALL run in a separate job that is skipped for pull requests.

#### Scenario: Packagers do not share a concurrency group [rb_concurrency_unique]

- **WHEN** `release-please.yml` calls the six packagers for one tag
- **THEN** each packager resolves to a group that names its own workflow, and
  none resolves to the caller's `release-please-*` group

#### Scenario: A fork cannot run the privileged build [rb_fork_gate]

- **WHEN** a pull request from a fork names its head branch
  `release-please--branches--master`
- **THEN** the packager job is skipped because the head repository is not this
  repository

#### Scenario: A pull-request dry run cannot write [rb_pr_read_only]

- **WHEN** a packager runs for a pull request
- **THEN** its workflow token has `contents: read`, it keeps artifacts as
  workflow artifacts, and no release upload job runs

### Requirement: Downloaded build inputs are pinned

Third-party actions used by the release pipeline SHALL be pinned to commit
SHAs, and the linuxdeploy, linuxdeploy Qt plugin, `create-dmg`, and sccache
downloads SHALL be pinned to immutable releases or commit SHAs. Linux downloads
SHALL be verified against a recorded SHA256 before execution.

#### Scenario: Actions use commit SHAs [rb_actions_pinned]

- **WHEN** the release workflows and composite actions are inspected
- **THEN** every third-party `uses:` entry names a 40-character commit SHA, with
  the human-readable ref in a comment

#### Scenario: Linux bundling tools are verified [rb_linuxdeploy_pinned]

- **WHEN** `linux-bundle` downloads `linuxdeploy` and its Qt plugin
- **THEN** it fetches release-tagged URLs and aborts if either SHA256 differs
  from the recorded value

### Requirement: The Windows installer and uninstaller are data-safe

The installer SHALL write a unique installation marker. The uninstaller SHALL
remove `$INSTDIR` recursively only when that marker is present, and SHALL quit
without deleting the uninstall registry keys when it is absent. The installer
SHALL refuse to install into a non-empty directory that does not already carry
the marker. The built `setup.exe` SHALL carry product version metadata.

#### Scenario: Uninstalling a shared directory keeps unrelated files [rb_win_marker]

- **WHEN** the application is installed into a directory that already contains
  other files
- **THEN** the installer refuses that directory unless it is a previous Kooka
  Pictura install marked by the installer

#### Scenario: A missing marker keeps the uninstall entry [rb_win_uninstall]

- **WHEN** the uninstaller runs in a directory without the installation marker
- **THEN** it reports that nothing was removed and exits before deleting the
  uninstall registry keys or shortcuts

### Requirement: Windows artifacts are exercised as shipped

The Windows workflow SHALL run the self-test from the extracted portable archive
and from a silent installation of `setup.exe`, then uninstall and assert the
install directory is gone. It SHALL clear inherited Qt plugin variables before
each run. The build SHALL report a working sccache backend.

#### Scenario: The portable archive self-tests [rb_win_zip]

- **WHEN** the portable `.zip` is extracted and `pictura.exe --headless
  --self-test` is run from the extracted folder
- **THEN** it exits 0 and prints `SUMMARY passed=<n> failed=0`

#### Scenario: The installer installs, self-tests, and uninstalls [rb_win_installer]

- **WHEN** `setup.exe /S /D=<temporary directory>` is run, followed by the
  installed self-test and `uninstall.exe /S`
- **THEN** the installed binary passes the self-test and the temporary install
  directory no longer exists

#### Scenario: sccache writes to the backend [rb_win_sccache]

- **WHEN** the Windows build finishes and `sccache --show-stats` is read
- **THEN** the cache backend reports writes or hits, with no cache errors

### Requirement: The macOS bundle has a real deployment floor

The macOS build SHALL export the 13.0 deployment target to Cargo and cxx-qt,
SHALL fail when the linker reports a binary built for a newer macOS, and SHALL
assert that the application binary's minimum OS version is 13.0. The offscreen
platform plugin SHALL be deployed with its Qt framework references rewritten to
the bundle and SHALL be ad-hoc signed.

#### Scenario: The deployment target reaches Cargo [rb_macos_target]

- **WHEN** the macOS build compiles the Rust static library and the C++ shell
- **THEN** the link completes without a `built for newer 'macOS'` warning and
  the application binary's `minos` is 13.0

#### Scenario: The offscreen plugin is bundle-relative [rb_macos_offscreen]

- **WHEN** the deployed `libqoffscreen.dylib` is inspected with `otool -L`
- **THEN** its Qt framework references resolve inside the bundle rather than to
  the runner's Qt installation

### Requirement: The macOS DMG is verified from the mounted image

The macOS workflow SHALL build the DMG before smoke-testing, mount it at a fixed
mount point, run the self-test from the bundle on that volume with `DYLD_*` and
Qt plugin variables unset, assert `qt.conf` is present, verify the bundle's
libraries are bundle-relative, and detach the image. It SHALL pin the
`create-dmg` script.

#### Scenario: The self-test runs from the DMG [rb_macos_dmg]

- **WHEN** the built `KookaPictura-X.Y.Z-macOS.dmg` is attached and its
  `pictura` is run with `--headless --self-test`
- **THEN** it exits 0, prints `SUMMARY passed=<n> failed=0`, and its `otool -L`
  output contains no path into the runner's Qt installation

### Requirement: The AppImage self-test is isolated from the build host

The AppImage workflow SHALL run the self-test inside a clean container with only
the AppImage filesystem mounted and `APPIMAGE_EXTRACT_AND_RUN=1`, so a missing
bundled library cannot be masked by the build runner's Qt.

#### Scenario: The AppImage runs without the host Qt [rb_appimage_isolated]

- **WHEN** the built AppImage is executed in `ubuntu:24.04`
- **THEN** it exits 0 and prints `SUMMARY passed=<n> failed=0` without
  `LD_LIBRARY_PATH` or `QT_PLUGIN_PATH` from the runner

### Requirement: Linux packages carry license and AppStream metadata

The `.deb` and `.rpm` SHALL install the project license and third-party license
text under `/usr/share/doc/kooka-pictura/`, and SHALL install a native AppStream
metainfo file for `pictura.desktop`. Both packages SHALL use the same version
string derived from the release tag without rewriting dashes.

#### Scenario: The packages install the license files [rb_pkg_license]

- **WHEN** either package is installed
- **THEN** `/usr/share/doc/kooka-pictura/copyright` and the third-party license
  text are present

#### Scenario: The packages install AppStream metadata [rb_pkg_appstream]

- **WHEN** either package is installed
- **THEN** `/usr/share/metainfo/pictura.metainfo.xml` is present and validates
  for the native `pictura.desktop` launchable

#### Scenario: Package versions agree [rb_pkg_version]

- **WHEN** the `.deb` and `.rpm` are built for tag `vX.Y.Z`
- **THEN** both report version `X.Y.Z` and the RPM does not rewrite the version

### Requirement: Flatpak metadata and vendored crates track the release

The Flatpak metainfo release version SHALL be updated by release-please. A cheap
check SHALL compare `Cargo.lock` with `flatpak/cargo-sources.json` and fail when
a locked crate or version is missing. Portal confinement SHALL be recorded as
unproven rather than implied.

#### Scenario: release-please updates the metainfo [rb_flatpak_metainfo]

- **WHEN** release-please bumps the version
- **THEN** the Flatpak metainfo `<release version>` is updated with the same
  version

#### Scenario: A stale crate list fails the lint [rb_flatpak_vendoring]

- **WHEN** `Cargo.lock` contains a crate version absent from
  `flatpak/cargo-sources.json`
- **THEN** the Flatpak lint job fails and names the missing crate

#### Scenario: Portal confinement is documented as partial [rb_flatpak_portal]

- **WHEN** the parity register and the Flatpak workflow are read
- **THEN** XC-7 is PARTIAL and both name the unproven portal path (`#647`)

### Requirement: Snap version and metadata are correct

The snap workflow SHALL assert that `v$(cat version.txt)` equals the requested
tag before uploading. `snap/snapcraft.yaml` SHALL declare the license, contact,
website, source-code, and issue-tracker metadata.

#### Scenario: A mismatched tag fails before upload [rb_snap_version]

- **WHEN** the snap workflow is dispatched for a tag whose version differs from
  `version.txt`
- **THEN** it fails before building or uploading the snap

#### Scenario: The snap declares its metadata [rb_snap_metadata]

- **WHEN** `snap/snapcraft.yaml` is inspected
- **THEN** it declares `license: GPL-3.0-or-later`, `contact`, `website`,
  `source-code`, and `issues`

### Requirement: Tag input and dry-run versions are consistent

Every packager SHALL accept a tag with or without a leading `v` for checkout and
upload. Pull-request dry runs SHALL use the same `0.0.0~pr<N>` version fallback,
and the snap SHALL use it when `KOOKA_VERSION` is set.

#### Scenario: A tag without the v prefix works [rb_tag_normalization]

- **WHEN** a packager is dispatched with `-f tag=X.Y.Z`
- **THEN** it checks out `vX.Y.Z` and uploads to the `vX.Y.Z` release

#### Scenario: Dry-run versions agree [rb_dry_run_version]

- **WHEN** the six packagers run for one pull request
- **THEN** each artifact name uses `0.0.0~pr<N>`

### Requirement: Documentation matches the shipped release pipeline

`DEVELOPING.md` SHALL list all six packagers and their artifacts, describe the
macOS 13 floor and the unsigned first-launch path, and describe the Flatpak
build's real network behavior. The parity register SHALL record the Flatpak
packaging as partial because portal confinement is unproven.

#### Scenario: The workflow list is complete [rb_docs_workflows]

- **WHEN** a maintainer reads "Cutting a release" in `DEVELOPING.md`
- **THEN** it names `appimage`, `linux-packages`, `snap`, `flatpak`, `windows`,
  and `macos` and their artifacts

#### Scenario: The unsigned macOS path is documented [rb_docs_macos]

- **WHEN** a maintainer reads the macOS section
- **THEN** it states the macOS 13 floor, that the DMG is unsigned, and that the
  user must allow it in System Settings → Privacy & Security → Open Anyway

#### Scenario: Flatpak network behavior is accurate [rb_docs_flatpak]

- **WHEN** a maintainer reads the Flatpak section
- **THEN** it no longer claims the build performs no network I/O and instead
  names the vendored sources and the manual regeneration step
