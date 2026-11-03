# interop/app-updater Specification

## Purpose
Defines how Kooka Pictura discovers a newer release, decides whether the running
build can update itself, verifies the download, protects unsaved work, and
installs or points at the correct update path for each packaging format.

## Requirements

### Requirement: The application checks for updates once per launch

When automatic checks are enabled, the application SHALL request the newest
stable release from the repository's GitHub Releases `releases/latest`
endpoint once per launch. The request MUST be asynchronous and MUST NOT delay
the main window from appearing. Automatic checks SHALL run for packaged builds;
a development build MUST NOT auto-check but SHALL allow a manual check. The
application MUST skip the request in headless, self-test, theme-dump,
MCP-bridge, GPU-probe, and test runs.

#### Scenario: A normal launch checks once [au_launch_check]

- **WHEN** the application starts interactively with automatic checks enabled
- **THEN** it sends exactly one `GET` request to the release endpoint and the
  window is usable while the request is in flight

#### Scenario: Automatic checks are disabled [au_launch_disabled]

- **WHEN** the application starts with automatic checks disabled
- **THEN** it sends no release request at launch

#### Scenario: A headless or test run starts [au_headless_skips]

- **WHEN** the application runs with `--headless`, `--self-test`,
  `--dump-theme`, `--mcp`, `--gpu-probe`, or with update checks disabled by
  the test harness
- **THEN** it sends no release request

#### Scenario: A development build starts [au_dev_no_auto_check]

- **WHEN** the detected package method is development and the person has not
  asked for a check
- **THEN** no automatic request is sent, while
  `Help > Check for Updates…` still performs one

### Requirement: Release versions are compared with SemVer ordering

The application SHALL parse the release tag as a SemVer version, accepting an
optional leading `v`, and MUST compare it to the running `PICTURA_VERSION`
using SemVer precedence. Build metadata MUST be ignored. A prerelease MUST sort
before its corresponding release. The application MUST offer an update only
when the release version is strictly newer than the running version.

#### Scenario: A newer release is available [au_newer_release]

- **WHEN** the running version is `0.3.0` and the release tag is `v0.4.0`
- **THEN** the application reports an available update

#### Scenario: The same version is published [au_same_version]

- **WHEN** the running version is `0.3.0` and the release tag is `v0.3.0`
- **THEN** the application reports no update

#### Scenario: The release is older [au_older_release]

- **WHEN** the running version is `0.3.0` and the release tag is `v0.2.9`
- **THEN** the application reports no update

#### Scenario: A prerelease is compared with its release [au_prerelease_order]

- **WHEN** the running version is `0.4.0-beta.1` and the release tag is
  `v0.4.0`
- **THEN** the application reports an update, and the reverse comparison
  reports no update

### Requirement: The application detects how it was installed

The application SHALL classify the running build as one of AppImage, Flatpak,
Snap, deb, rpm, Windows setup, Windows portable, macOS app, or development.
Detection MUST use the sandbox environment markers, the executable location,
the Windows install marker, the macOS bundle location, and the distribution
package database in that order. The detected method MUST be available to the
banner, Preferences, About, and System Info.

#### Scenario: A sandbox or AppImage marker is present [au_detect_env]

- **WHEN** `FLATPAK_ID`, `SNAP`, or `APPIMAGE` is set
- **THEN** the detector returns Flatpak, Snap, or AppImage respectively

#### Scenario: A Linux package install is detected [au_detect_path]

- **WHEN** the executable is under `/opt/kooka-pictura` and the deb or rpm
  package database contains `kooka-pictura`
- **THEN** the detector returns deb or rpm

#### Scenario: A Windows install is detected [au_detect_windows]

- **WHEN** the platform is Windows and the `HKLM\Software\KookaPictura`
  `InstallDir` marker contains the executable
- **THEN** the detector returns Windows setup; without the marker it returns
  Windows portable

#### Scenario: An unpackaged build is detected [au_detect_development]

- **WHEN** no sandbox marker, package database entry, Windows marker, or macOS
  bundle applies
- **THEN** the detector returns development

### Requirement: The matching release asset is selected

The application SHALL select the release asset that matches the detected
method and the current CPU architecture. When no matching asset exists, it MUST
report that no downloadable asset is available rather than choosing an
unrelated file.

#### Scenario: An AppImage asset is selected [au_select_appimage]

- **WHEN** the method is AppImage on x86_64 and the release contains
  `KookaPictura-0.4.0-x86_64.AppImage`
- **THEN** the selector returns that asset

#### Scenario: A Windows setup asset is selected [au_select_windows]

- **WHEN** the method is Windows setup on x64 and the release contains
  `KookaPictura-0.4.0-windows-x64-setup.exe`
- **THEN** the selector returns that asset

#### Scenario: A macOS asset is selected [au_select_macos]

- **WHEN** the method is macOS app and the release contains
  `KookaPictura-0.4.0-macOS.dmg`
- **THEN** the selector returns that asset

#### Scenario: No matching asset exists [au_select_missing]

- **WHEN** the method or architecture has no matching release asset
- **THEN** the selector reports no asset and the UI offers the release page

### Requirement: Downloads are verified before installation

The installer SHALL download only over HTTPS, except a loopback endpoint used
by the test harness, MUST refuse a download larger than 512 MiB, and MUST
verify the downloaded bytes against the SHA-256 digest published for the release
asset. It MUST write through a temporary file in the
destination directory and MUST NOT install when the digest is missing or does
not match. A failed or cancelled download MUST leave the running application
usable and MUST NOT leave a partial file in the install location.

#### Scenario: A matching digest is accepted [au_digest_ok]

- **WHEN** the downloaded file's SHA-256 equals the asset digest
- **THEN** the installer proceeds to the install step

#### Scenario: The digest is missing [au_digest_missing]

- **WHEN** the release asset has no digest
- **THEN** the installer refuses auto-install and the UI offers the release
  page

#### Scenario: The digest does not match [au_digest_mismatch]

- **WHEN** the downloaded file's SHA-256 differs from the asset digest
- **THEN** the installer deletes the temporary file and reports an error

#### Scenario: The download is too large [au_oversize]

- **WHEN** the response declares or streams more than 512 MiB
- **THEN** the installer aborts the download and reports the size limit

#### Scenario: The network fails [au_network_failure]

- **WHEN** the request fails, times out, or is rate-limited
- **THEN** an automatic check is silent, a manual check reports the failure,
  and the application remains usable

### Requirement: Unsaved work is handled before an update

Before downloading or installing an update, the application SHALL run the same
unsaved-document prompt as closing the window. It MUST abort the update when the
person cancels or when a save fails, and MUST NOT discard or close a document
without the person's choice.

#### Scenario: A dirty document is saved [au_save_dirty]

- **WHEN** a document has unsaved changes and the person chooses Save
- **THEN** the document is saved before the update proceeds

#### Scenario: The person cancels the save prompt [au_save_cancel]

- **WHEN** a dirty document is open and the person cancels the prompt
- **THEN** the update is aborted and the document remains open

#### Scenario: Saving fails [au_save_fail]

- **WHEN** the person chooses Save and the save operation fails
- **THEN** the update is aborted and the document remains open

### Requirement: The update is installed according to the package method

The application SHALL install only through the path the detected method
supports. AppImage MUST replace the running image atomically, keep a `.old`
backup, and relaunch the new image. Windows setup MUST launch the downloaded
installer and quit so the installer can relaunch the application. macOS MUST
open the verified DMG and offer to quit. Flatpak, Snap, deb, and rpm MUST NOT
self-update. An install failure MUST leave the old application runnable.

#### Scenario: An AppImage is replaced and relaunched [au_install_appimage]

- **WHEN** a verified AppImage update is installed in a writable directory
- **THEN** the old image is kept as `.old`, the new image replaces it, and the
  new image is launched

#### Scenario: A Windows setup is launched [au_install_windows]

- **WHEN** a verified Windows setup update is installed
- **THEN** the installer is launched and the application quits

#### Scenario: A macOS DMG is opened [au_install_macos]

- **WHEN** a verified macOS DMG update is installed
- **THEN** the DMG is opened and the application offers to quit

#### Scenario: Installation fails [au_install_failure]

- **WHEN** the replacement or relaunch fails
- **THEN** the old application remains runnable and the person sees the error

### Requirement: Package-manager-owned builds never self-update

For Flatpak, Snap, deb, and rpm builds, the application MUST NOT download or
replace the installed files. It SHALL show the package-manager command that
updates the installation and SHALL offer the release page.

#### Scenario: A Flatpak build checks [au_flatpak_hint]

- **WHEN** the detected method is Flatpak and an update is available
- **THEN** the UI shows `flatpak update io.github.KookaStudio.KookaPictura` and
  does not download the flatpak bundle

#### Scenario: A Snap build checks [au_snap_hint]

- **WHEN** the detected method is Snap and an update is available
- **THEN** the UI shows `snap refresh kooka-pictura` and does not download the
  snap

#### Scenario: A deb or rpm build checks [au_deb_rpm_hint]

- **WHEN** the detected method is deb or rpm and an update is available
- **THEN** the UI shows the matching install command and the release page

### Requirement: Automatic checks are private and bounded

The automatic request MUST send no unique installation identifier. It SHALL
send only the standard `User-Agent: KookaPictura/<version>` and the GitHub API
headers. The application MUST make at most one automatic request per launch,
MUST NOT retry automatically, and MUST remain usable while offline.

#### Scenario: The request carries no identifier [au_no_identifier]

- **WHEN** an automatic check is sent
- **THEN** the request headers contain no machine, user, or installation
  identifier

#### Scenario: GitHub rate-limits the request [au_rate_limited]

- **WHEN** the endpoint returns 403 or 429
- **THEN** the automatic check ends silently and no retry is scheduled

#### Scenario: The machine is offline [au_offline]

- **WHEN** the request cannot connect
- **THEN** the automatic check ends silently and the application remains usable

### Requirement: A manual check is always available

`Help > Check for Updates…` SHALL perform an explicit check even when automatic
checks are disabled. A manual check MUST report an available update, an
up-to-date result, or a failure.

#### Scenario: A manual check runs with automatic checks off [au_manual_when_off]

- **WHEN** automatic checks are disabled and the person chooses
  `Help > Check for Updates…`
- **THEN** the application performs the request and reports the result

#### Scenario: A manual check finds no update [au_manual_up_to_date]

- **WHEN** the manual check finds the running version is current
- **THEN** the application reports that the version is up to date

#### Scenario: A manual check fails [au_manual_error]

- **WHEN** the manual check cannot reach the endpoint
- **THEN** the application reports the failure without closing or blocking the
  document
