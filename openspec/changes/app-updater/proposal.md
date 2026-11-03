# Proposal

## Why

Kooka Pictura publishes eight release artifacts per tag (AppImage, deb, rpm,
snap, flatpak, Windows setup, Windows portable, macOS DMG), but the application
has no way to notice that a newer release exists. A person must watch the
repository, know which artifact matches their install, and repeat the whole
install by hand. The application also cannot say how it was installed, which
makes support and bug reports harder.

Issue #1146 asks for a launch check, an upgrade prompt that names the version
transition, a "don't show this" opt-out, and a flow that saves work, downloads
the matching package, and restarts. The existing draft contract,
`docs/11-cross-cutting/update-and-versioning.md` (XC-011), specifies update
delivery but no code exists; the missing `--version`, About and update-checker
surface is tracked by #649.

## What Changes

- Add a startup update check against the repository's GitHub Releases
  `releases/latest` endpoint. The check is asynchronous and never blocks the
  window from appearing.
- Compare the running `PICTURA_VERSION` with the latest release tag using
  SemVer ordering, and show the upgrade path (`0.3.0 → 0.4.0`).
- Add a non-modal update banner with **Update Now**, **Release Notes**, **Later**,
  and a **Don't check for updates automatically** checkbox.
- Add `Help > Check for Updates…` for an explicit check even when automatic
  checks are off.
- Add `Edit > Preferences > Updates` with the automatic-check toggle, the
  running version, the detected package method, **Check Now**, and the last
  check result.
- Detect how the running build was installed: AppImage, Flatpak, Snap, deb,
  rpm, Windows setup, Windows portable, macOS app, or a development build.
- Download the matching release asset over HTTPS, verify its SHA-256 digest and
  size, write it atomically, and install it where the platform allows:
  - AppImage: replace the running image atomically, keep a `.old` backup, and
    relaunch.
  - Windows setup: launch the downloaded installer and quit so its finish page
    can relaunch the application.
  - macOS DMG: open the verified DMG and offer to quit so the person can drag
    the app to Applications.
  - Flatpak, Snap, deb, and rpm: never self-update; show the package-manager
    command and the release page instead.
- Before downloading, run the same unsaved-document prompt as window close and
  abort the update if the person cancels or a save fails.
- Skip all network activity in headless, self-test, theme-dump, MCP-bridge, and
  GPU-probe runs, and in tests that construct the window with checks disabled.
- Show the version and package method in `Help > About` and in the System Info
  report, so a bug report says how the build was installed.
- Add the Flatpak `--share=network` permission, because the check runs inside
  the sandbox; Snap already has network access.
- Update XC-011 and `ROADMAP.md` to record the owner decisions and the shipped
  surface.

## Capabilities

### New Capabilities

- `interop/app-updater`: release discovery, SemVer comparison, package-method
  detection, asset selection, integrity verification, save-before-update,
  per-format installation/restart, and the no-self-update rule for
  package-manager-owned builds.
- `ui/update-preferences`: the startup banner, the Preferences > Updates pane,
  the automatic-check opt-out, progress and error states, the manual-check menu
  entry, and version/package visibility in About and System Info.

### Modified Capabilities

(none)

## Impact

- **New shell files:** `update_version.{h,cpp}`, `update_package.{h,cpp}`,
  `update_release.{h,cpp}`, `update_checker.{h,cpp}`,
  `update_installer.{h,cpp}`, `update_banner.{h,cpp}`,
  `update_preferences_page.{h,cpp}`, and `frame_update.cpp` under
  `crates/pictura-app/cpp/`.
- **Edited shell files:** `frame.h`, `frame.cpp`, `frame_columns.cpp`,
  `frame_menus.cpp`, `frame_preferences.cpp`, `preferences_dialog.{h,cpp}`,
  `session.{h,cpp}`, `system_info.{h,cpp}`, `gpu_probe.{h,cpp}`, `main.cpp`,
  and `CMakeLists.txt`.
- **Command catalogue:** a new `help.checkForUpdates` entry in
  `crates/pictura-script/src/command_catalogue/entries.rs` and
  `const_names.rs`, with `crates/pictura-app/cpp/command_ids.h` regenerated.
- **Packaging:** `flatpak/io.github.KookaStudio.KookaPictura.yml` gains
  `--share=network`. No installer or release-asset naming changes are required;
  the updater selects the existing names.
- **Tests:** a new `tst_app_updater` suite, a new `tst_update_preferences`
  suite, a new `tst_system_info::packageMethodIsReported` case, and the
  `tests/CMakeLists.txt` registrations.
- **Dependencies:** none. Qt Network is already a `find_package` component and
  is already linked into `pictura_shell`; the updater uses it directly.
- **Privacy:** automatic checks are on by default by owner decision for #1146.
  The request sends no identifier beyond the standard `User-Agent:
  KookaPictura/<version>`; GitHub sees the client IP. The banner checkbox and
  Preferences turn automatic checks off, and manual checks still work.
- **Security:** downloads are HTTPS-only, capped in size, verified against the
  release asset's SHA-256 digest, and written through a temporary file in the
  destination directory. A missing or mismatched digest refuses auto-install
  and offers the release page instead.
- **Out of scope:** beta/nightly channels, delta/zsync downloads, rollback
  beyond the AppImage `.old` backup, silent Windows installs, code-signing or
  notarization verification, and self-update for Flatpak/Snap/deb/rpm.
