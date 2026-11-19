# Spec Delta

## MODIFIED Requirements

### Requirement: The matching release asset is selected

The application SHALL select the release asset that matches the detected
method and the current CPU architecture. When no matching asset exists, it MUST
report that no downloadable asset is available rather than choosing an
unrelated file.

#### Scenario: An AppImage asset is selected [au_select_appimage]

- **WHEN** the method is AppImage on x86_64 and the release contains
  `KookaPictura-0.4.0-x86_64.AppImage`
- **THEN** the selector returns that asset

#### Scenario: An aarch64 AppImage asset is selected [au_select_appimage_aarch64]

- **WHEN** the method is AppImage on aarch64 (reported as `arm64` or `aarch64`)
  and the release contains both `KookaPictura-0.4.0-x86_64.AppImage` and
  `KookaPictura-0.4.0-aarch64.AppImage`
- **THEN** the selector returns the aarch64 asset, never the x86_64 one

#### Scenario: arm64 deb and rpm assets are selected [au_select_deb_rpm_arm64]

- **WHEN** the method is deb or rpm on aarch64 (reported as `arm64` or
  `aarch64`) and the release contains both architectures of each package
- **THEN** the selector returns `KookaPictura_0.4.0_arm64.deb` for deb and
  `KookaPictura-0.4.0-1.aarch64.rpm` for rpm, never an x86_64 package

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

## ADDED Requirements

### Requirement: The deb and rpm update command names the host's package

When a deb or rpm build cannot install an update itself, the command it shows
SHALL name the package file for the version and the current CPU architecture.
That name SHALL be the same one the asset selector matches. When the
architecture has no package, the application MUST show no command and SHALL
offer the release page instead.

#### Scenario: An x86_64 package command is shown [au_command_deb_rpm]

- **WHEN** the method is deb or rpm on x86_64 and the release is `v0.4.0`
- **THEN** the command is `sudo apt install ./KookaPictura_0.4.0_amd64.deb` or
  `sudo dnf install ./KookaPictura-0.4.0-1.x86_64.rpm`

#### Scenario: An arm64 package command is shown [au_command_deb_rpm_arm64]

- **WHEN** the method is deb or rpm on aarch64 (reported as `arm64` or
  `aarch64`) and the release is `0.4.0`
- **THEN** the command is `sudo apt install ./KookaPictura_0.4.0_arm64.deb` or
  `sudo dnf install ./KookaPictura-0.4.0-1.aarch64.rpm`

#### Scenario: An architecture without a package shows no command [au_command_missing_arch]

- **WHEN** the method is deb or rpm on an architecture with no package, such
  as `riscv64`
- **THEN** the command is empty
