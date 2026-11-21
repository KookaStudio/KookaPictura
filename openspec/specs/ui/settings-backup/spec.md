# ui/settings-backup Specification

## Purpose
Lets a person export every persisted Kooka Pictura setting into one portable
`pictura.json` file and restore it safely on the same or another machine.

## Requirements

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
`pictura-settings`, `formatVersion` equal to 2, the running `appVersion`, an
ISO-8601 `exportedAt`, and the sections `preferences`, `session`, `workspaces`,
and `presets`. The `preferences` section MUST carry the validated user TOML
document as a string, including unrecognized TOML values. With no user file it
MUST carry a valid document representing the absence of user overrides. Export
MUST report an error when it cannot include a present user file. The `session` section MUST carry the stored session object
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

- **WHEN** the person exports after saving preferences and a session, creating a workspace,
  saving an adjustment preset, installing an `.atn` set, and saving a PDF
  preset
- **THEN** the file contains the preferences TOML, session object, workspace index and files,
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
than this build, an unsafe file name, invalid base64, and an invalid or
unsupported `preferences` document. It SHALL accept version 1 backups.
Imported TOML SHALL be validated with the live store's schema and size limits. On success it MUST
write a timestamped backup of the current settings first, then sanitize and
write a pending import file. It MUST NOT change the live preferences, session, workspace,
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
- **THEN** a pending import file exists, the live preferences and session stores are
  unchanged, and the application offers a restart

#### Scenario: Older backups leave current lists untouched [sb_current_lists_absent]

- **WHEN** a backup without the current-list section or a section missing one type is imported and applied
- **THEN** the corresponding current lists remain unchanged

#### Scenario: Invalid current lists do not disturb staged or live settings [sb_current_lists_invalid]

- **WHEN** a current-list section is malformed, oversized, or cannot be staged
- **THEN** import reports failure and preserves both the live stores and any previous pending import

#### Scenario: Invalid preference data is refused before staging [sb_prefs_invalid]

- **WHEN** a selected backup contains malformed TOML or an unsupported preference schema
- **THEN** the import reports an error and leaves live settings and any earlier pending import intact

### Requirement: A staged import applies before the next session load

On the next launch, before the window loads the session, the application SHALL
apply the pending import: it MUST write each present section to its store,
remove the pending file, and leave absent sections untouched. An explicit
`preferences` section SHALL replace the user preferences document. For version 1
backups, known preference keys in the legacy `session` section SHALL be translated
and applied as preference edits over the current user document. Legacy keys
SHALL be removed from the imported session JSON. Each staged import SHALL have a durable unique identity. Before applying a pre-existing pending version 1 file without an identity, the application SHALL atomically assign and sync one to that pending file; failure SHALL leave it unapplied for retry. Its preference application, whether a version 1 patch or version 2 replacement, SHALL atomically record a local receipt for that identity in the preferences document under the preference write lock. Ordinary edits and other imports, including version 2 replacements, SHALL retain all outstanding receipts, even when separate state roots share the same config root. Portable exports SHALL omit receipts and the local agent recovery guard. A recovery guard SHALL survive either import form and all retries; while it is active imports SHALL restore ordinary preferences without restoring agent opt-in or grants until the user explicitly grants access again. A retry with a matching receipt SHALL skip preference application while retrying the remaining sections and pending-file removal, so later preference edits survive. A different import identity SHALL apply normally. Identity-specific receipt cleanup SHALL use the preference write lock and occur only after that identity’s pending import is durably retired. A failed preference commit
SHALL leave the pending import available for retry. A corrupt pending
file MUST be quarantined and the launch MUST continue with the existing
settings. Present current lists MUST be validated before mutation and applied before the
current lists are loaded. All changed native per-type files MUST be staged in
a candidate generation, with unchanged types carried forward, then published
together by atomically switching a small active manifest. A failed write or
manifest switch MUST keep the previous generation active and the pending
import available. Startup MUST report the failure and continue loading the
previous complete generation; a retry before loading lists on a later startup
MUST be idempotent and publish the complete candidate.
Applying a file exported from the same build MUST restore portable store contents, subject to machine-specific sanitization and any active local agent recovery guard. Local receipts and recovery guards are not portable contents and SHALL remain local.

#### Scenario: Every section is applied [sb_apply_all]

- **WHEN** a pending import contains preferences, session state, workspaces, and presets
- **THEN** the next launch writes all of them and removes the pending file

#### Scenario: Absent sections are left alone [sb_apply_absent]

- **WHEN** a pending import contains only the session section
- **THEN** the preferences, workspace, and preset stores keep their current contents

#### Scenario: A corrupt pending file is quarantined [sb_apply_corrupt]

- **WHEN** the pending file is unreadable or invalid
- **THEN** it is renamed to a `.corrupt-<epoch>` file and the launch continues
  with the existing settings

#### Scenario: Export then import restores the stores [sb_round_trip]

- **WHEN** a backup is exported from an unguarded profile, the stores are changed, and the backup is imported and applied to an unguarded profile without machine-specific sanitization changes
- **THEN** portable user TOML values, session state, workspaces, and presets equal their exported contents, including unknown TOML values

#### Scenario: Failed current-list apply retries before use [sb_current_lists_retry]

- **WHEN** a staged current-list replacement fails during startup
- **THEN** the pending import remains, startup reports the error and loads the previous complete generation, and a later successful retry publishes all present lists together

#### Scenario: Current lists survive portable backup and restore [sb_current_lists_round_trip]

- **WHEN** populated, default and explicitly empty current lists are exported, then the destination edits each type, and the backup is imported and applied after restart
- **THEN** all eight lists have their exported contents, including the formerly default list and explicit emptiness, and the pre-import backup contains the prior lists

#### Scenario: Version 1 preferences restore from the legacy section [sb_prefs_legacy]

- **WHEN** an older settings file carries only some legacy preferences within its session object
- **THEN** startup restores those values in TOML, preserves other preferences, and writes the supplied state keys to the session JSON

#### Scenario: Later edits survive a partially applied import [sb_prefs_retry_edits]

- **WHEN** a version 1 or version 2 import commits preferences but another section or pending-file removal fails, and the user edits preferences before a later retry
- **THEN** the retry finishes the import without reapplying its preferences and the later edit survives, including after interruption between preference commit and retry

#### Scenario: Concurrent pending imports retain independent receipts [sb_prefs_receipts_shared_config]

- **WHEN** two state roots share a config root and each applies a distinct pending import, including a pre-existing version 1 pending file without an identity, while the first import remains pending after preference commit
- **THEN** each identity remains recorded until its own pending file is durably retired, and retrying the first does not overwrite subsequent preference edits or the second import

#### Scenario: Failed preference commits can be retried [sb_prefs_retry]

- **WHEN** startup cannot commit the preferences from a pending import
- **THEN** the prior complete preferences file is retained and a later writable startup retries the pending import

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
