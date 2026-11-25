## MODIFIED Requirements

### Requirement: The DMG packages the bundle

The macOS workflow SHALL pack the deployed bundle into
`KookaPictura-<version>-macOS.dmg` with `create-dmg`, including an Applications
shortcut beside the app for drag-to-install. The workflow SHALL smoke-check
the deployed bundle before packing it and verify the shortcut on the mounted
DMG before retaining or publishing the artifact.

#### Scenario: The DMG contains the application

- **WHEN** the DMG is opened in Finder
- **THEN** `Kooka Pictura.app` and a shortcut to `/Applications` are available
  in the same window, and the bundle declares `Kooka Pictura` as its display name

#### Scenario: The smoke check runs before packing

- **WHEN** the workflow reaches the DMG step
- **THEN** the deployed bundle has already exited 0 from
  `--headless --self-test`

### Requirement: Signing and notarization are deferred

The macOS build MUST NOT require an Apple Developer certificate or notarization
credentials. It SHALL add all bundle resources, including the package-method
marker, before applying the app's ad-hoc signature. The workflow SHALL verify
the app with `codesign --verify --deep --strict` from the mounted DMG before
retaining the Release PR artifact or publishing a release asset. The DMG SHALL
remain unsigned, and `DEVELOPING.md` SHALL document the signing status and
first-launch Gatekeeper behavior.

#### Scenario: The build runs without signing secrets

- **WHEN** the macOS workflow runs without Apple signing secrets
- **THEN** the app in the mounted DMG contains the package marker and passes
  strict verification of its ad-hoc signature

#### Scenario: Packaging alters the signed bundle

- **WHEN** a resource is added to the signed app before it is packed into the DMG
- **THEN** the mounted-DMG signature check fails before the artifact can be
  retained or published
