## ADDED Requirements

### Requirement: The unsigned snap's install command reaches users

While the snap is published only as a release asset, `README.md`,
`public_docs/installing.md`, the site's Download panel, and each GitHub Release's
notes SHALL give the install command with the `--dangerous` flag. The Download
panel SHALL show the command only when the latest release has a snap asset, and
SHALL use that asset's filename. The snap workflow SHALL add the command to the
release notes without duplicating it when re-run for the same tag.

#### Scenario: A user reads the install documentation

- **WHEN** a user reads `README.md` or `public_docs/installing.md`
- **THEN** they find `sudo snap install --dangerous` with the snap's filename
  pattern

#### Scenario: The Download panel lists a snap

- **WHEN** the latest release has a `.snap` asset and a visitor opens the Linux
  download choices
- **THEN** the snap entry shows the install command with the asset's filename,
  and no other format shows a command

#### Scenario: The workflow runs for a tag

- **WHEN** the snap workflow uploads the `.snap` to a release
- **THEN** that release's notes contain the install command

#### Scenario: The workflow is re-run for the same tag

- **WHEN** the snap workflow runs for a tag whose notes already contain the
  install command
- **THEN** the notes are left unchanged

#### Scenario: The documented command installs the release snap

- **WHEN** a user runs the documented command on a release `.snap`
- **THEN** snapd installs it and the app's headless self-test reports `failed=0`
