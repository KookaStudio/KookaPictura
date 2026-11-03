# Tasks: app-updater

## 1. Test plan and test scaffolding

- [x] 1.1 Add `test-plan.md` to this change with one row per spec scenario.
- [x] 1.2 Register `tst_app_updater` and `tst_update_preferences` in
      `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- [x] 1.3 Add the planned `tst_app_updater` cases (initially failing or
      skipped) for version ordering, package detection, asset selection,
      release parsing, integrity, install, and the no-network paths.
- [x] 1.4 Add the planned `tst_update_preferences` cases for the update dialog,
      Preferences page, opt-out, progress/error states, menu entry, About, and
      System Info.
- [x] 1.5 Add `tst_system_info::packageMethodIsReported`.

## 2. Pure update logic

- [x] 2.1 Add `update_version.{h,cpp}`: SemVer parse (optional leading `v`,
      prerelease, build metadata), comparison, and `isNewer`.
- [x] 2.2 Add `update_package.{h,cpp}`: `PackageMethod` enum, injectable
      detection inputs, asset suffix, self-update capability, and the
      package-manager hint text.
- [x] 2.3 Add `update_release.{h,cpp}`: parse the GitHub release JSON, validate
      `tag_name`, `html_url`, `body`, and `assets[]`, and select an asset by
      method and architecture.
- [x] 2.4 Cover 2.1–2.3 with the planned `tst_app_updater` cases and a release
      JSON fixture.

## 3. Session and policy

- [x] 3.1 Add `checkForUpdatesOnLaunch = true` to `SessionState`; bump
      `schemaVersion` to 12 in `session.h` and
      `PicturaMainWindow::saveSession()`; load with a default of `true` when
      the key is absent; save it.
- [x] 3.2 Add a `PicturaMainWindow` constructor parameter
      `bool enableUpdateCheck = true`; pass `false` from
      `pictura::test::makeMainWindow()`.
- [x] 3.3 Set `PICTURA_DISABLE_UPDATE_CHECK=1` in `main.cpp` for
      `--headless`, `--self-test`, and `--dump-theme` before any window is
      constructed (the `--mcp` and `--gpu-probe` paths return before a window
      exists), and make the settings helper refuse the network when the process
      arguments name those modes. Add `PICTURA_UPDATE_URL` handling for local
      tests.
- [ ] 3.4 Cover the policy with `disabledCheckMakesNoRequest`,
      `headlessSkipsCheck`, `developmentBuildDoesNotAutoCheck`, and
      `preferencesDefaultOn`.

## 4. Network checker

- [x] 4.1 Add `update_checker.{h,cpp}`: `QNetworkAccessManager` request with
      the GitHub headers, 10-second timeout, one automatic request per launch,
      manual check, and result signals (`updateAvailable`, `upToDate`,
      `failed`, `progress`).
- [x] 4.2 Parse the reply with `QJsonDocument` and `update_release`; treat
      non-2xx, timeout, and malformed JSON as failures.
- [x] 4.3 Cover the checker with a local `QTcpServer` fixture:
      `checkOnLaunchSendsOneRequest`, `newerReleaseNotifies`,
      `sameVersionDoesNotNotify`, `olderReleaseDoesNotNotify`,
      `prereleaseOrdering`, `requestCarriesNoIdentifier`,
      `rateLimitIsSilent`, `offlineIsSilent`, `manualCheckWorksWhenAutoOff`,
      `manualCheckUpToDate`, `manualCheckError`.

## 5. Download and install

- [x] 5.1 Add `update_installer.{h,cpp}`: HTTPS-only check, 512 MiB cap,
      temporary file in the destination directory, SHA-256 verification,
      cancellation, and atomic replace.
- [x] 5.2 Implement the AppImage path: chmod, `.old` backup, atomic rename,
      restore on failure, and a restart callback.
- [x] 5.3 Implement the Windows setup path: launch the verified installer and
      quit.
- [x] 5.4 Implement the macOS path: open the verified DMG and offer to quit.
- [x] 5.5 Implement the Flatpak, Snap, deb, rpm, Windows portable, and
      development paths: no download, show the command or release page.
- [x] 5.6 Cover with `acceptsMatchingDigest`, `refusesMissingDigest`,
      `refusesMismatchedDigest`, `refusesOversizeDownload`,
      `appImageReplacesAndRelaunches`, `installFailureKeepsAppRunning`,
      `flatpakShowsUpdateCommand`, `snapShowsUpdateCommand`, and
      `debRpmShowsPackageManagerCommand`.

## 6. Update dialog and Preferences UI

- [x] 6.1 Add `update_banner.{h,cpp}`: non-modal popup dialog with the version
      transition, **Update now**, **Release Notes**, **Later**, the
      **Check for updates automatically** checkbox, progress, cancel, retry, and
      release-page states.
- [x] 6.2 Add `update_preferences_page.{h,cpp}`: automatic-check checkbox,
      running version, package method, **Check Now**, and last result.
- [x] 6.3 Add `PreferencesDialog::kUpdates` through `addPage`, wire it from
      `frame_preferences.cpp`, and persist the checkbox.
- [x] 6.4 Keep the dialog non-activating and defer it while a modal dialog is
      active; it sizes and centres itself.
- [x] 6.5 Add the `help.checkForUpdates` command to
      `crates/pictura-script/src/command_catalogue/entries.rs` and
      `const_names.rs`; regenerate `command_ids.h` with
      `scripts/gen-command-ids.py`; add the handler in `frame_menus.cpp`.
- [x] 6.6 Cover with `bannerShowsVersionTransition`, `bannerDoesNotStealFocus`,
      `laterHidesBanner`, `togglePersists`, `checkNowChecks`,
      `dontCheckDisablesAutoChecks`, `reenableFromPreferences`,
      `progressIsShown`, `cancelStopsDownload`, `failureOffersRetry`, and
      `menuEntryChecks`.

## 7. Wiring and save gate

- [x] 7.1 Add `frame_update.cpp`: schedule the launch check from `showEvent`
      after the window is visible, create the dialog and Preferences page,
      connect the menu action, and own the checker/installer lifetimes.
- [x] 7.2 Extract the unsaved-document loop from `closeEvent` into
      `bool saveAllDocumentsForRestart()` and call it from both `closeEvent`
      and the update flow; no behavior change to close.
- [x] 7.3 Refuse **Update Now** while a modal dialog is active, with a short
      message.
- [ ] 7.4 Cover with `savePromptRunsBeforeInstall`, `cancelSaveAbortsInstall`,
      and `saveFailureAbortsInstall`.

## 8. Support surface

- [x] 8.1 Add the running version and package method to the `Help > About`
      handler in `frame_menus.cpp`, factoring the message into a helper the
      test can assert without opening a modal.
- [x] 8.2 Add `packageMethod` to `SystemInfo`, print `Package:` in
      `formatSystemInfo`, and cover it with
      `tst_system_info::packageMethodIsReported`.
- [x] 8.3 Cover About with `aboutShowsVersionAndMethod`.

## 9. Packaging

- [ ] 9.1 Add `--share=network` to
      `flatpak/io.github.KookaStudio.KookaPictura.yml` and verify the manifest
      still lints.
- [x] 9.2 Confirm the Snap build already has network access; no manifest
      change unless the strict confinement needs one.
- [x] 9.3 Confirm the Windows NSIS installer's
      `HKLM\Software\KookaPictura\InstallDir` marker is the detection input;
      no installer change is expected.
- [x] 9.4 Add a release-asset-name check to the updater tests so a packaging
      rename fails a test.

## 10. Documentation and roadmap

- [ ] 10.1 Update `docs/11-cross-cutting/update-and-versioning.md` to record
      the owner decisions: on-by-default with opt-out, the C++ shell mapping,
      the per-format install matrix, the digest gate, and the stable-only
      channel. Commit with `TASK-ALLOWS-DOCS`.
- [ ] 10.2 Update `ROADMAP.md`: tick the in-app update checker/updater line
      (#1146) and note that #649 still covers `--version` and the remaining
      About details.

## 11. Verification and archive

- [ ] 11.1 `openspec validate --all --strict`.
- [ ] 11.2 `cargo fmt --all` and
      `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] 11.3 `cargo nextest run --workspace`.
- [ ] 11.4 Build once, then run
      `ctest --test-dir build -R '^(tst_app_updater|tst_update_preferences|tst_system_info)$' --output-on-failure`.
- [ ] 11.5 Run `bash scripts/verify-fast.sh`.
- [ ] 11.6 Manual: real AppImage replacement and relaunch.
- [ ] 11.7 Manual: Windows setup/UAC and macOS DMG drag.
- [ ] 11.8 Manual: Flatpak network permission, Snap refresh, deb/rpm command,
      and a live GitHub API check.
- [ ] 11.9 Archive the change with `openspec archive app-updater` in the same
      pull request that ships it.
