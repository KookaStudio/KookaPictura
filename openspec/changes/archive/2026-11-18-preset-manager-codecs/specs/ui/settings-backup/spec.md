# Settings Backup

## MODIFIED Requirements

### Requirement: Export writes one complete settings file

Export SHALL write a single pretty-printed JSON object with `format` equal to
`pictura-settings`, an integer `formatVersion`, the running `appVersion`, an
ISO-8601 `exportedAt`, and the sections `session`, `workspaces`, and
`presets`. The `session` section MUST carry the stored session object
verbatim, including keys this build does not understand. The `workspaces`
section MUST carry the workspace index and every workspace file. The `presets`
section MUST carry the user adjustment preset files, the installed `.atn`
action sets, the Actions palette, and the PDF presets. A `currentPresetLists` section MUST carry the effective current list for all
eight types, including defaults resolved from native or legacy data without
mutating live settings and explicitly empty native lists. Export and the pre-import backup MUST report
unreadable current-list files or a backup exceeding the supported import size,
and MUST NOT commit an incomplete backup. Machine caches
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

#### Scenario: Incomplete current lists cannot be exported [sb_current_lists_export_failure]

- **WHEN** a current-list file is unreadable or the complete backup exceeds the supported import size
- **THEN** export reports failure and leaves any existing destination intact

### Requirement: Import validates and stages instead of mutating live stores

Import SHALL read and validate the file before touching any store. It MUST
refuse malformed JSON, a missing or unknown `format`, a `formatVersion` newer
than this build, an unsafe file name, and invalid base64. On success it MUST
write a timestamped backup of the current settings first, then sanitize and
write a pending import file. It MUST NOT change the live session, workspace,
preset, or current-list stores while the application is running. The current-list
section MUST validate each present native set and enforce bounded sizes before
staging. A failed validation or staging MUST leave the previous pending import
and live stores unchanged. Sanitization MUST drop
recent-file entries that do not exist on this machine and window geometry that
does not intersect an available screen. Absent sections MUST leave the
corresponding store untouched. An absent `currentPresetLists` section, including
in older backups, MUST leave all current lists untouched; absent types within a
present section MUST also remain untouched, while an explicitly empty set MUST
clear its type when applied.

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

#### Scenario: Older backups leave current lists untouched [sb_current_lists_absent]

- **WHEN** a backup without the current-list section or a section missing one type is imported and applied
- **THEN** the corresponding current lists remain unchanged

#### Scenario: Invalid current lists do not disturb staged or live settings [sb_current_lists_invalid]

- **WHEN** a current-list section is malformed, oversized, or cannot be staged
- **THEN** import reports failure and preserves both the live stores and any previous pending import

### Requirement: A staged import applies before the next session load

On the next launch, before the window loads the session, the application SHALL
apply the pending import: it MUST write each present section to its store,
remove the pending file, and leave absent sections untouched. A corrupt pending
file MUST be quarantined and the launch MUST continue with the existing
settings. Present current lists MUST be validated before mutation and applied before the
current lists are loaded. All changed native per-type files MUST be staged in
a candidate generation, with unchanged types carried forward, then published
together by atomically switching a small active manifest. A failed write or
manifest switch MUST keep the previous generation active and the pending
import available. Startup MUST report the failure and continue loading the
previous complete generation; a retry before loading lists on a later startup
MUST be idempotent and publish the complete candidate.
Applying a file exported from the same build MUST restore every store's contents.

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

#### Scenario: Failed current-list apply retries before use [sb_current_lists_retry]

- **WHEN** a staged current-list replacement fails during startup
- **THEN** the pending import remains, startup reports the error and loads the previous complete generation, and a later successful retry publishes all present lists together

#### Scenario: Current lists survive portable backup and restore [sb_current_lists_round_trip]

- **WHEN** populated, default and explicitly empty current lists are exported, then the destination edits each type, and the backup is imported and applied after restart
- **THEN** all eight lists have their exported contents, including the formerly default list and explicit emptiness, and the pre-import backup contains the prior lists
