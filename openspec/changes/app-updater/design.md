# Design

## Context

The repository already has the pieces the updater needs, but none of them are
connected:

- `PICTURA_VERSION` is a compile definition from the CMake `project()` version
  (`CMakeLists.txt`), and release-please keeps it in sync with `version.txt`,
  `Cargo.toml`, and the Flatpak metainfo.
- `.github/workflows/release-please.yml` cuts a GitHub Release and calls the
  AppImage, Linux package, Snap, Flatpak, Windows, and macOS packaging
  workflows. Their asset names are stable:
  `KookaPictura-<version>-x86_64.AppImage`,
  `KookaPictura_<version>_amd64.deb`,
  `KookaPictura-<version>-1.x86_64.rpm`,
  `kooka-pictura_<version>_amd64.snap`,
  `KookaPictura-<version>-x86_64.flatpak`,
  `KookaPictura-<version>-windows-x64-setup.exe`,
  `KookaPictura-<version>-windows-x64-portable.zip`, and
  `KookaPictura-<version>-macOS.dmg`.
- Qt Network is already required by `find_package` and linked into
  `pictura_shell`; no new dependency is needed.
- `SessionState` already persists UI preferences in
  `$XDG_STATE_HOME/kooka-pictura/state.json`; the automatic-check flag belongs
  there.
- `ControlApprovalBar` is an existing non-modal, non-activating overlay
  pattern that the update banner can follow.
- `PicturaMainWindow::closeEvent` already has the exact unsaved-document loop
  the update flow must run before replacing the application.
- `agent_control.cpp` already detects Flatpak, Snap, and AppImage from the
  environment. The updater needs a fuller detector, including deb, rpm,
  Windows setup/portable, macOS, and development builds.

The existing XC-011 draft proposes a Rust `pictura_update` crate, but the
updater is shell/OS integration: network I/O, package-manager commands,
installer processes, and Qt widgets. The C++ shell owns those concerns today
(`system_info.cpp`, `agent_control.cpp`, `session.cpp`), so the implementation
lives there. The pure logic is split into small C++ units that Qt Test can
exercise without a network or a real package manager. XC-011 is updated to
record the actual mapping.

## Goals / Non-Goals

**Goals:**

- A launch check that is quick, non-blocking, and silent on failure.
- A banner that names the transition and asks before doing anything.
- A real update path for the formats the application owns: AppImage automatic,
  Windows setup assisted, macOS DMG assisted.
- A safe hint, never a self-update, for Flatpak, Snap, deb, and rpm.
- A single preference that turns automatic checks off, reachable from both the
  banner and Preferences.
- No network traffic in tests or headless runs.
- No new dependency.

**Non-Goals:**

- Beta/nightly channels, staged rollouts, or per-channel feeds.
- zsync/AppImageUpdate delta downloads. The first version downloads the whole
  asset; this is the `ponytail:` ceiling.
- Silent Windows installs or automatic macOS app replacement.
- Code-signing or notarization verification. The SHA-256 digest published by
  the GitHub Releases API is the integrity check.
- Self-update for Flatpak, Snap, deb, or rpm. The package manager owns those
  files.
- Telemetry or a unique installation identifier.

## Decisions

### Decision: GitHub Releases `releases/latest`, not a bespoke feed

`GET https://api.github.com/repos/KookaStudio/KookaPictura/releases/latest`
returns the newest non-prerelease, non-draft release, its tag, body, HTML URL,
and assets with `name`, `browser_download_url`, `size`, and `digest`. That is
exactly the data the banner, release notes, asset selection, and integrity check
need. The endpoint is public, needs no token, and matches the repository's
existing release automation.

Request headers:

```
Accept: application/vnd.github+json
X-GitHub-Api-Version: 2022-11-28
User-Agent: KookaPictura/<PICTURA_VERSION>
```

A 10-second transfer timeout, no retry loop, and a single automatic request per
launch. A 403/429 (rate limit) or a network error is silent for an automatic
check and becomes a one-line error for a manual check.

Alternatives considered:

- **A project-hosted JSON manifest**: another artifact to publish and keep in
  sync; the GitHub release already carries the version, notes, assets, and
  digests.
- **Parsing the `releases/latest` redirect**: avoids the API rate limit but
  still needs the API for assets and digests, and a redirect chain is more
  fragile.
- **An update-info file in the AppImage**: AppImage-specific and does not cover
  Windows or macOS.

### Decision: C++ shell units, not a new Rust crate

The split is:

| Unit | Responsibility | Test surface |
|---|---|---|
| `update_version.{h,cpp}` | Parse `vX.Y.Z[-pre][+build]`, SemVer ordering, `isNewer`. | Pure Qt Test cases |
| `update_package.{h,cpp}` | `PackageMethod` enum, detection from explicit inputs (env map, OS, executable path, install marker), asset suffix and package-manager hint. | Pure Qt Test cases with injected inputs |
| `update_release.{h,cpp}` | Parse the GitHub release JSON, validate fields, select an asset by method and architecture. | Qt Test with a release JSON fixture |
| `update_checker.{h,cpp}` | `QNetworkAccessManager` request, timeout, result signals, one-per-launch guard, manual check. | Qt Test against a local `QTcpServer` |
| `update_installer.{h,cpp}` | Download to a temp file, size cap, SHA-256, atomic install, restart callback. | Qt Test with a local server and a temp directory |
| `update_banner.{h,cpp}` | Non-modal overlay: version path, buttons, opt-out checkbox, progress, error. | Qt Test widget assertions |
| `update_preferences_page.{h,cpp}` | Preferences > Updates page. | Qt Test widget assertions |
| `frame_update.cpp` | Wiring: schedule the check, show the banner, save-work gate, menu handler, Preferences page. | Qt Test through `PicturaMainWindow` |

Keeping this in the shell matches the existing ownership boundary and avoids a
cross-language bridge for a handful of string and enum operations. The units
still expose pure functions, so the test plan is not weaker than a Rust crate's.

### Decision: package method detection is explicit and injectable

Detection order and evidence:

| Method | Evidence |
|---|---|
| Flatpak | `FLATPAK_ID` is set and non-empty |
| Snap | `SNAP` is set and non-empty |
| AppImage | `APPIMAGE` is set and non-empty |
| Windows setup | `_WIN32`, and `HKLM\Software\KookaPictura\InstallDir` exists and the executable is under it |
| Windows portable | `_WIN32`, no install marker |
| macOS app | `Q_OS_MACOS`, and the executable is inside a `.app` bundle |
| deb | Linux, not a sandbox or AppImage, executable under `/opt/kooka-pictura`, and `/var/lib/dpkg/info/kooka-pictura.list` exists |
| rpm | Linux, not a sandbox or AppImage, executable under `/opt/kooka-pictura`, and `rpm -q kooka-pictura` succeeds |
| Development | everything else |

The detector takes its inputs as parameters (`env`, `os`, `executablePath`,
`installMarker`, `packageDatabase`) rather than reading the process directly,
so every row is testable on Linux. The production entry point fills the inputs
from `QProcessEnvironment`, `QSysInfo`, `QCoreApplication::applicationFilePath`,
the registry, and the package database. Detection runs once at startup and is
cached; it never runs on a paint or a panel refresh.

`/opt/kooka-pictura` is the prefix used by `packaging/build-packages.sh` for
both deb and rpm. The package database check distinguishes them; if neither
database is present, the method is `Development` and the UI does not offer a
self-update.

### Decision: one asset selector, with an explicit no-asset result

`update_release` maps the method and `QSysInfo::currentCpuArchitecture()` to a
suffix:

| Method | Architecture | Asset suffix |
|---|---|---|
| AppImage | x86_64 | `-x86_64.AppImage` |
| deb | amd64 | `_amd64.deb` |
| rpm | x86_64 | `-1.x86_64.rpm` |
| Snap | amd64 | `_amd64.snap` |
| Flatpak | x86_64 | `-x86_64.flatpak` |
| Windows setup | x64 | `-windows-x64-setup.exe` |
| Windows portable | x64 | `-windows-x64-portable.zip` |
| macOS | any | `-macOS.dmg` |

The selector prefers an asset whose name contains the release version, then
falls back to the suffix alone. An unknown architecture or a missing asset
returns `std::nullopt`; the UI then offers the release page rather than a broken
download.

### Decision: integrity is a hard gate for auto-install

The GitHub asset `digest` field is `sha256:<hex>` when present. The installer:

1. Refuses a non-HTTPS URL, except a loopback `http://127.0.0.1` URL used by
   tests.
2. Follows GitHub's asset redirects with
   `QNetworkRequest::NoLessSafeRedirectPolicy`; the test harness disables the
   system proxy for loopback requests.
3. Refuses a `Content-Length` or streamed size above 512 MiB.
4. Downloads to a temporary file in the destination directory, so the final
   rename is atomic on the same filesystem. The SHA-256 is updated as chunks
   arrive rather than over the whole file on the GUI thread.
5. Compares the incremental SHA-256 to the asset digest.
6. Refuses auto-install when the digest is missing. The banner then offers
   **Release Page** instead of **Update Now**.

This is deliberately stricter than "download and run". A release asset without
a digest is still available through the release page, but the application does
not execute it.

### Decision: save-work gate reuses the close path

`PicturaMainWindow` gets a private `bool saveAllDocumentsForRestart()` that runs
the same loop as `closeEvent`: for each dirty document, ask Save / Don't Save /
Cancel, save when chosen, and return false on Cancel or a failed save. Both
`closeEvent` and the update flow call it. This keeps one definition of "is it
safe to replace the running application?" and avoids a second, subtly different
prompt.

If a modal dialog is active when the person clicks **Update Now**, the flow
refuses with a short message rather than stacking a second modal on top.

### Decision: per-format install behavior

| Method | Download | Install | Restart |
|---|---|---|---|
| AppImage | yes | chmod 0755, rename current to `<name>.old`, rename new into place; restore `.old` on failure | `QProcess::startDetached(newPath, {})` then quit |
| Windows setup | yes | launch the downloaded `setup.exe` | quit; the installer's finish page runs the app |
| macOS DMG | yes | open the verified DMG | offer **Quit Now**; the person drags the app to Applications |
| Flatpak | no | show `flatpak update io.github.KookaStudio.KookaPictura` | none |
| Snap | no | show `snap refresh kooka-pictura` | none |
| deb | no | show `sudo apt install ./KookaPictura-<version>_amd64.deb` and open the release page | none |
| rpm | no | show `sudo dnf install ./KookaPictura-<version>-1.x86_64.rpm` and open the release page | none |
| Windows portable | no | open the release page for the portable zip | none |
| Development | no | show the release page and the running version | none |

The AppImage install refuses a read-only directory and reports the path instead
of corrupting the running image. `$APPIMAGE` is resolved through
`QFileInfo::canonicalFilePath()` before choosing the sibling destination, so a
launcher symlink does not send the update to the wrong directory. The `.old`
file is kept as the rollback copy; the next successful update replaces it.

### Decision: the banner is an overlay, not a modal dialog

The banner follows `ControlApprovalBar`: a `QFrame` parented to the window,
positioned at the top centre, `Qt::WA_ShowWithoutActivating`, and no focus
stealing. It shows:

```
Kooka Pictura 0.4.0 is available. You have 0.3.0.
[Release Notes] [Later] [Update Now]   [ ] Don't check for updates automatically
```

The transition text is `current → available`, which is the upgrade path the
issue asks for. **Later** hides the banner for this launch. **Don't check for
updates automatically** writes `checkForUpdatesOnLaunch = false` to the session
and updates the Preferences checkbox if the dialog exists. **Release Notes**
opens a `QTextBrowser` with the release body as Markdown. For Flatpak, Snap,
deb, rpm, Windows portable, and development methods, **Update Now** is replaced
by **Show Update Command** (or **Release Page** where there is no command), so
the banner never promises an install it cannot perform.

If a modal dialog is active when the result arrives, the banner waits for the
modal to close rather than appearing behind it. The banner is repositioned in
`PicturaMainWindow::resizeEvent`.

### Decision: automatic checks are on by default, with an explicit opt-out

The owner decision for #1146 is on-by-default, overriding the XC-011 draft's
off/manual default. The preference is a single checkbox in a non-CS6
**Updates** page, added through `PreferencesDialog::addPage` like **Agent
Control**. The page also shows the running version, the detected package
method, **Check Now**, and the last result.

A development build (`PackageMethod::Development`) does not auto-check; it
would otherwise hit GitHub on every `./build/pictura` launch and could never
install the result. `Help > Check for Updates…` still works there and shows
the release page.

`Help > Check for Updates…` is always available and performs an explicit check
even when automatic checks are off. This is the consent boundary: the automatic
request is opt-out, the manual request is always explicit.

### Decision: no network in headless or tests

`PicturaMainWindow` gains a constructor parameter
`bool enableUpdateCheck = true`. The flag disables the automatic launch
request only; the checker object, the banner, the Preferences page, and the
manual menu action still exist. The test helper `makeMainWindow()` passes
`false`, so the 300+ existing Qt Test suites never touch the network.
Production `main.cpp` sets `PICTURA_DISABLE_UPDATE_CHECK=1` for `--headless`,
`--self-test`, and `--dump-theme` before any window is constructed, so
self-test windows created inside `runSelfTest()` are covered too; `--mcp` and
`--gpu-probe` return before a window exists. The settings helper also refuses
the network when the process arguments name one of those modes.
`PICTURA_DISABLE_UPDATE_CHECK` is an additional escape hatch for CI and
packaging smoke tests.

The checker also reads `PICTURA_UPDATE_URL` (default: the GitHub API endpoint)
so Qt Test can point it at a local `QTcpServer` serving a captured release
fixture. A loopback `http://127.0.0.1` URL is allowed only in this test path.

### Decision: one request per launch, no retry loop

An automatic check happens once per process. A manual check is allowed at any
time and is disabled while a check is in flight. There is no background timer,
no exponential backoff, and no repeated nagging. A failed automatic check is
silent; the next launch tries again. This keeps the GitHub unauthenticated rate
limit (60 requests/hour/IP) out of the critical path.

### Decision: version and package method are support data

The `Help > About` handler in `frame_menus.cpp` gains the running version and
package method. `SystemInfo` gains a `packageMethod` field and
`formatSystemInfo` prints `Package: <method>`. This is the "every build should
include what type of pkg it's installed from" part of #1146, and it makes a
submitted issue actionable.

## Risks / Trade-offs

- **GitHub API rate limits.** One request per launch is usually fine, but a
  shared NAT or a scripted restart loop can hit 60/hour. The check is silent on
  403/429 and retries on the next launch. A future cached ETag or a
  project-hosted manifest would remove the dependency.
- **Asset names are release-please/tag-derived.** A packaging change that
  renames an asset silently disables that format's update path. The selector
  tests use the exact current names and the release workflow is the single
  source of those names.
- **Windows needs UAC.** The NSIS installer requests administrator and writes
  `HKLM`. The updater launches it interactively rather than silently, so the
  person sees the UAC prompt. A cancelled installer leaves the old version
  installed; the application has already quit, so the person relaunches it.
- **macOS is assisted, not automatic.** Copying a signed `.app` over the
  running bundle is not safe from inside the app. The DMG is opened and the
  person drags it; the banner offers **Quit Now**.
- **AppImage in a read-only directory.** The sibling rename fails. The updater
  reports the path and opens the release page instead of writing elsewhere and
  leaving the person with an unlaunchable image.
- **No code-signing verification.** The SHA-256 digest from GitHub proves the
  bytes match the release asset, not the publisher. Signing/notarization is a
  separate packaging task.
- **Flatpak network permission.** `--share=network` widens the sandbox. The
  check is opt-out and sends no identifier, but this is a real permission
  change and must be reviewed with the Flatpak manifest.
- **Test flakiness.** Network tests use a local `QTcpServer`, not the internet,
  and every asynchronous assertion waits on a signal with a timeout. Platform
  install paths (Windows/macOS) are manual rows.
- **Version comparison is easy to get subtly wrong.** Prerelease ordering is
  covered by dedicated cases; build metadata is ignored.

## File plan

New:

- `crates/pictura-app/cpp/update_version.{h,cpp}`
- `crates/pictura-app/cpp/update_package.{h,cpp}`
- `crates/pictura-app/cpp/update_release.{h,cpp}`
- `crates/pictura-app/cpp/update_checker.{h,cpp}`
- `crates/pictura-app/cpp/update_installer.{h,cpp}`
- `crates/pictura-app/cpp/update_banner.{h,cpp}`
- `crates/pictura-app/cpp/update_preferences_page.{h,cpp}`
- `crates/pictura-app/cpp/frame_update.cpp`
- `crates/pictura-app/cpp/tests/tst_app_updater.cpp`
- `crates/pictura-app/cpp/tests/tst_update_preferences.cpp`

Edited:

- `crates/pictura-app/cpp/frame.h`, `frame.cpp`, `frame_columns.cpp`,
  `frame_menus.cpp`, `frame_preferences.cpp`
- `crates/pictura-app/cpp/preferences_dialog.{h,cpp}`
- `crates/pictura-app/cpp/session.{h,cpp}`
- `crates/pictura-app/cpp/system_info.{h,cpp}`
- `crates/pictura-app/cpp/main.cpp`
- `crates/pictura-app/cpp/command_ids.h` (regenerated)
- `crates/pictura-script/src/command_catalogue/entries.rs`,
  `const_names.rs`
- `CMakeLists.txt`, `crates/pictura-app/cpp/tests/CMakeLists.txt`
- `flatpak/io.github.KookaStudio.KookaPictura.yml`
- `docs/11-cross-cutting/update-and-versioning.md`, `ROADMAP.md`

## Verification

- `openspec validate --all --strict`
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo nextest run --workspace`
- `cmake --build build --parallel` (after code is complete)
- `ctest --test-dir build -R '^(tst_app_updater|tst_update_preferences|tst_system_info)$' --output-on-failure`
- `bash scripts/verify-fast.sh`
- Manual: real AppImage replacement, Windows installer/UAC, macOS DMG drag,
  Flatpak network permission, Snap refresh, and a live GitHub API check.
