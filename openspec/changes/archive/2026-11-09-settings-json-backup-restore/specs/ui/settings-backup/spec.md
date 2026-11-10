# Spec Delta

## Purpose

Lets a person export every persisted Kooka Pictura setting into one portable
`pictura.json` file and restore it safely on the same or another machine.

## ADDED Requirements

### Requirement: Preferences exposes export and import

`Edit > Preferences > General` SHALL show **Export Settings…** and **Import
Settings…** buttons. Each button MUST open a file chooser filtered to
`*.json`, with `pictura.json` as the suggested export name. Cancelling a
chooser MUST leave every store unchanged.

#### Scenario: The General pane offers both buttons [sb_prefs_buttons]

- **WHEN** the person opens `Edit > Preferences > General`
- **THEN** the pane shows **Export Settings…** and **Import Settings…**

#### Scenario: Export is cancelled [sb_export_cancel]

- **WHEN** the person opens **Export Settings…** and cancels the file chooser
- **THEN** no file is written and every store is unchanged

#### Scenario: Import is cancelled [sb_import_cancel]

- **WHEN** the person opens **Import Settings…** and cancels the file chooser
- **THEN** no pending import is staged and every store is unchanged

### Requirement: Export writes one complete settings file

Export SHALL write a single pretty-printed JSON object with `format` equal to
`pictura-settings`, an integer `formatVersion`, the running `appVersion`, an
ISO-8601 `exportedAt`, and the sections `session`, `workspaces`, and
`presets`. The `session` section MUST carry the stored session object
verbatim, including keys this build does not understand. The `workspaces`
section MUST carry the workspace index and every workspace file. The `presets`
section MUST carry the user adjustment preset files, the installed `.atn`
action sets, the Actions palette, and the PDF presets. Machine caches
(`gpu-backend.json`, `gpu-init.marker`) and crash-recovery snapshots MUST NOT
be included.

#### Scenario: Every user store is present [sb_export_sections]

- **WHEN** the person exports after saving a session, creating a workspace,
  saving an adjustment preset, installing an `.atn` set, and saving a PDF
  preset
- **THEN** the file contains the session object, the workspace index and files,
  and every preset file under `presets`

#### Scenario: Unknown session keys survive [sb_export_unknown_keys]

- **WHEN** `state.json` contains a key this build does not know
- **THEN** the exported `session` object contains that key unchanged

#### Scenario: Machine caches are excluded [sb_export_excludes_caches]

- **WHEN** the state directory contains `gpu-backend.json`,
  `gpu-init.marker`, and a `recovery/` directory
- **THEN** the exported file contains none of them

### Requirement: Import validates and stages instead of mutating live stores

Import SHALL read and validate the file before touching any store. It MUST
refuse malformed JSON, a missing or unknown `format`, a `formatVersion` newer
than this build, an unsafe file name, and invalid base64. On success it MUST
write a timestamped backup of the current settings first, then sanitize and
write a pending import file. It MUST NOT change the live session, workspace,
or preset stores while the application is running. Sanitization MUST drop
recent-file entries that do not exist on this machine and window geometry that
does not intersect an available screen. Absent sections MUST leave the
corresponding store untouched.

#### Scenario: A malformed file is refused [sb_import_rejects_malformed]

- **WHEN** the person chooses a file that is not a JSON object with the
  `pictura-settings` format
- **THEN** the import reports an error, stages nothing, and leaves every store
  unchanged

#### Scenario: A newer format version is refused [sb_import_rejects_newer]

- **WHEN** the file's `formatVersion` is greater than this build's version
- **THEN** the import reports that the file was written by a newer version and
  stages nothing

#### Scenario: Unsafe names are refused [sb_import_rejects_unsafe_names]

- **WHEN** a workspace or preset entry names a path with a separator or `..`
- **THEN** the import refuses the file and writes nothing outside the store
  directories

#### Scenario: The current settings are backed up [sb_import_backs_up]

- **WHEN** a valid import is staged
- **THEN** a timestamped `pictura-*.json` backup of the current settings exists
  in the state directory's backup folder

#### Scenario: Machine-specific values are sanitized [sb_import_sanitizes]

- **WHEN** the imported session lists a recent file that is missing and a
  window geometry that is off every screen
- **THEN** the pending session drops the missing recent entry and the geometry

#### Scenario: A valid import is staged [sb_import_stages]

- **WHEN** a valid file is imported
- **THEN** a pending import file exists, the live `state.json` is unchanged,
  and the application offers a restart

### Requirement: A staged import applies before the next session load

On the next launch, before the window loads the session, the application SHALL
apply the pending import: it MUST write each present section to its store,
remove the pending file, and leave absent sections untouched. A corrupt pending
file MUST be quarantined and the launch MUST continue with the existing
settings. Applying a file exported from the same build MUST restore every
store's contents.

#### Scenario: Every section is applied [sb_apply_all]

- **WHEN** a pending import contains the session, workspaces, and presets
- **THEN** the next launch writes all of them and removes the pending file

#### Scenario: Absent sections are left alone [sb_apply_absent]

- **WHEN** a pending import contains only the session section
- **THEN** the workspace and preset stores keep their current contents

#### Scenario: A corrupt pending file is quarantined [sb_apply_corrupt]

- **WHEN** the pending file is unreadable or invalid
- **THEN** it is renamed to a `.corrupt-<epoch>` file and the launch continues
  with the existing settings

#### Scenario: Export then import restores the stores [sb_round_trip]

- **WHEN** a backup is exported, the stores are changed, and the backup is
  imported and applied
- **THEN** the session, workspaces, and presets equal their exported contents

### Requirement: The restart prompt lets the person choose when to apply

After staging a valid import, the application SHALL offer **Quit Now** and
**Later**. **Quit Now** MUST quit without applying the import in the running
process, so the next launch applies it. **Later** MUST keep the application
open and MUST NOT apply the import in the running process; the next launch
applies it.

#### Scenario: The person quits now [sb_restart_quit]

- **WHEN** the person chooses **Quit Now**
- **THEN** the application quits, the pending file remains, and the next launch
  applies the import

#### Scenario: The person chooses Later [sb_restart_later]

- **WHEN** the person chooses **Later**
- **THEN** the application stays open with its current settings and the pending
  file remains for the next launch
