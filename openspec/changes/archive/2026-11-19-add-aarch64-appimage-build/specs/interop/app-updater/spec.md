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
