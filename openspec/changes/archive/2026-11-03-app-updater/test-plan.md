# Test Plan — app-updater

One row per scenario in `specs/interop/app-updater/spec.md` and
`specs/ui/update-preferences/spec.md`. Coverage names the automated test that
fails if the behaviour regresses, or `manual: <reason>` when the check needs a
real package format or host.

The `tst_app_updater` and `tst_update_preferences` suites are created by the
implementation tasks; until then, `scripts/check-test-plan.py` reports the
missing test references for this in-progress change.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `au_launch_check` | Start an interactive window with checks on and a local release server | Exactly one request is sent; the window is usable while it is in flight | `tst_app_updater::checkOnLaunchSendsOneRequest` |
| `au_launch_disabled` | Start with `checkForUpdatesOnLaunch = false` | The local server sees no connection | `tst_app_updater::disabledCheckMakesNoRequest` |
| `au_headless_skips` | Construct the window with `enableUpdateCheck = false` for each headless entry point | No request is sent | `tst_app_updater::headlessSkipsCheck` |
| `au_dev_no_auto_check` | Detect a development build and start the window without a manual check | No request is sent; a manual check still runs | `tst_app_updater::developmentBuildDoesNotAutoCheck` |
| `au_newer_release` | Compare `0.3.0` with `v0.4.0` | The result is an available update | `tst_app_updater::newerReleaseNotifies` |
| `au_same_version` | Compare `0.3.0` with `v0.3.0` | The result is up to date | `tst_app_updater::sameVersionDoesNotNotify` |
| `au_older_release` | Compare `0.3.0` with `v0.2.9` | The result is up to date | `tst_app_updater::olderReleaseDoesNotNotify` |
| `au_prerelease_order` | Compare `0.4.0-beta.1` with `0.4.0` and the reverse | The release is newer; the prerelease is not | `tst_app_updater::prereleaseOrdering` |
| `au_detect_env` | Detect with each of `FLATPAK_ID`, `SNAP`, and `APPIMAGE` set | Flatpak, Snap, and AppImage are returned | `tst_app_updater::detectsEnvironmentMarkers` |
| `au_detect_path` | Detect a `/opt/kooka-pictura` executable with the deb and rpm databases present | deb and rpm are returned | `tst_app_updater::detectsLinuxPackages` |
| `au_detect_windows` | Detect with the Windows OS and the HKLM install marker present, then absent | Windows setup, then Windows portable | `tst_app_updater::detectsWindowsInstallKinds` |
| `au_detect_development` | Detect with no marker, package database, or bundle | Development is returned | `tst_app_updater::detectsDevelopmentBuild` |
| `au_select_appimage` | Select from a release containing the AppImage asset on x86_64 | The AppImage asset is returned | `tst_app_updater::selectsAppImageAsset` |
| `au_select_windows` | Select from a release containing the Windows setup asset on x64 | The setup asset is returned | `tst_app_updater::selectsWindowsSetupAsset` |
| `au_select_macos` | Select from a release containing the macOS DMG | The DMG is returned | `tst_app_updater::selectsMacDmgAsset` |
| `au_select_missing` | Select for an architecture with no matching asset | No asset is returned and the release page is offered | `tst_app_updater::missingAssetFallsBackToReleasePage` |
| `au_digest_ok` | Download a file whose SHA-256 matches the asset digest | The installer proceeds to install | `tst_app_updater::acceptsMatchingDigest` |
| `au_digest_missing` | Download an asset with no digest | Auto-install is refused and the release page is offered | `tst_app_updater::refusesMissingDigest` |
| `au_digest_mismatch` | Download a file whose SHA-256 differs from the digest | The temporary file is deleted and an error is reported | `tst_app_updater::refusesMismatchedDigest` |
| `au_oversize` | Serve a response above the 512 MiB cap | The download aborts with the size error | `tst_app_updater::refusesOversizeDownload` |
| `au_network_failure` | Make the local server return HTTP 500 | An automatic check is silent and a manual check reports the failure | `tst_app_updater::networkFailureIsSilent` |
| `au_save_dirty` | Mark a document dirty, choose Save, then start the update | The document is saved before the update proceeds | `tst_app_updater::savePromptRunsBeforeInstall` |
| `au_save_cancel` | Mark a document dirty, choose Cancel in the prompt | The update is aborted and the document stays open | `tst_app_updater::cancelSaveAbortsInstall` |
| `au_save_fail` | Make the save fail after choosing Save | The update is aborted and the document stays open | `tst_app_updater::saveFailureAbortsInstall` |
| `au_install_appimage` | Install a verified AppImage into a temp directory with a restart callback | The old file is `.old`, the new file is in place, and the callback receives it | `tst_app_updater::appImageReplacesAndRelaunches` |
| `au_install_windows` | Run the Windows setup path on a Windows host | The installer opens and the app quits | manual: needs a Windows host with UAC and the NSIS installer |
| `au_install_macos` | Run the macOS path on a macOS host | The DMG opens and the quit offer appears | manual: needs a macOS host and a DMG |
| `au_install_failure` | Make the AppImage rename or restart callback fail | The old image remains runnable and the error is shown | `tst_app_updater::installFailureKeepsAppRunning` |
| `au_flatpak_hint` | Check with the Flatpak method and an available update | The flatpak update command is shown and nothing is downloaded | `tst_update_preferences::flatpakShowsUpdateCommand` |
| `au_snap_hint` | Check with the Snap method and an available update | The snap refresh command is shown and nothing is downloaded | `tst_update_preferences::snapShowsUpdateCommand` |
| `au_deb_rpm_hint` | Check with the deb and rpm methods | The apt and dnf commands are shown and nothing is downloaded | `tst_update_preferences::debRpmShowsPackageManagerCommand` |
| `au_no_identifier` | Capture the request headers at the local server | The User-Agent and GitHub API headers are present and no identifier header is | `tst_app_updater::requestCarriesNoIdentifier` |
| `au_rate_limited` | Answer 403 and 429 | The automatic check ends silently with no retry | `tst_app_updater::rateLimitIsSilent` |
| `au_offline` | Refuse the connection | The automatic check ends silently and the app stays usable | `tst_app_updater::offlineIsSilent` |
| `au_manual_when_off` | Disable automatic checks, then invoke the manual check | The request is sent and the result is reported | `tst_app_updater::manualCheckWorksWhenAutoOff` |
| `au_manual_up_to_date` | Manual check against the running version | The up-to-date result is reported | `tst_app_updater::manualCheckUpToDate` |
| `au_manual_error` | Manual check against a refused connection | The failure is reported without blocking the document | `tst_app_updater::manualCheckError` |
| `up_banner_version_path` | Show the update dialog for `0.3.0` with `0.4.0` available | The text contains `0.3.0 → 0.4.0`, **Update now**, **Release Notes**, and **Later** | `tst_update_preferences::bannerShowsVersionTransition` |
| `up_banner_auto_on` | Show the update dialog | The **Check for updates automatically** box is checked | `tst_update_preferences::bannerShowsVersionTransition` |
| `up_banner_no_focus` | Focus a document field, then show the dialog | The field keeps focus and the dialog is non-activating | `tst_update_preferences::bannerDoesNotStealFocus` |
| `up_banner_later` | Choose Later | The dialog hides for the launch | `tst_update_preferences::laterHidesBanner` |
| `up_prefs_default_on` | Open Preferences > Updates in a fresh session | The automatic-check checkbox is on | `tst_update_preferences::preferencesDefaultOn` |
| `up_prefs_toggle_persists` | Turn the checkbox off, save the session, reload it | The checkbox stays off and the launch check is skipped | `tst_update_preferences::togglePersists` |
| `up_prefs_check_now` | Press Check Now | A check runs and the result text updates | `tst_update_preferences::checkNowChecks` |
| `up_dont_check_disables` | Uncheck **Check for updates automatically** | The session stores automatic checks off and Preferences follows | `tst_update_preferences::dontCheckDisablesAutoChecks` |
| `up_reenable` | Turn automatic checks on in Preferences, then restart | The next launch checks | `tst_update_preferences::reenableFromPreferences` |
| `up_progress` | Start a download and report progress | The banner shows a progress indicator and sizes | `tst_update_preferences::progressIsShown` |
| `up_cancel` | Choose Cancel during a download | The transfer stops, the temp file is removed, and the banner resets | `tst_update_preferences::cancelStopsDownload` |
| `up_retry` | Fail a download | The error, Retry, and Release Page are shown | `tst_update_preferences::failureOffersRetry` |
| `up_menu_entry` | Choose `Help > Check for Updates…` | A manual check runs and reports its result | `tst_update_preferences::menuEntryChecks` |
| `up_about_version` | Open `Help > About` | The version and package method are shown | `tst_update_preferences::aboutShowsVersionAndMethod` |
| `up_system_info_package` | Copy the System Info report | It contains a `Package:` line with the method | `tst_system_info::packageMethodIsReported` |

## Manual rows

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Real AppImage replacement | Build an AppImage, serve a newer verified AppImage, choose Update Now | The new image replaces the old one, `.old` is kept, and the new image launches | manual: needs a built AppImage and a real relaunch |
| Live GitHub check | Run a release build with checks on against the real endpoint | The banner names the latest release and its notes | manual: needs the live GitHub API and a released version |
| Flatpak network permission | Build the Flatpak with `--share=network`, launch it, and check | The check reaches GitHub and the banner appears | manual: needs a Flatpak build and sandbox |
| Snap refresh path | Install the snap, check, and run the shown command | `snap refresh kooka-pictura` updates the snap | manual: needs a Snap install |
| deb/rpm package-manager path | Install the deb and rpm, check, and run the shown command | The package manager updates the package | manual: needs the real packages and package managers |
