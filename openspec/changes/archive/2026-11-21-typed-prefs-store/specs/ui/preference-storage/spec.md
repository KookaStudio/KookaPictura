# Spec Delta

## Purpose

Define durable application preferences, upgrades from legacy profiles, and recovery and reset behavior across launches.

## ADDED Requirements

### Requirement: Application preferences have a typed configuration store

The application SHALL store its existing global preferences in a typed TOML
document with a `schema_version` beginning at 1. On Linux the user file SHALL be
`$XDG_CONFIG_HOME/kooka-pictura/prefs.toml`, with the XDG home fallback for unset,
empty, or relative paths. On other supported platforms the application SHALL use
the per-user configuration location. Per-user keys SHALL override optional
read-only system defaults, which SHALL override compiled defaults. The app SHALL
create a valid user file on the first writable launch and retain existing
defaults, field validation, and apply timing. Window geometry, opaque dock
layout, named workspaces, recent files, and remembered workflow state SHALL use
the state stores. New Unix configuration directories and files SHALL be private
to the user.

#### Scenario: A fresh profile uses independent roots [prefs_paths]

- **WHEN** the application launches with empty, distinct config and state roots
- **THEN** it creates a versioned TOML preferences file with defaults in the config root, and any saved window and recent-file state is JSON in the state root

#### Scenario: User preferences override system defaults [prefs_layers]

- **WHEN** a system file specifies two known preferences and the user file overrides one
- **THEN** the override wins, the other system value applies, and missing fields inherit compiled defaults

#### Scenario: Changed preferences survive relaunch [prefs_round_trip]

- **WHEN** the user changes live and restart-class preferences, then quits and launches again
- **THEN** the changes return with their documented apply timing and the session JSON contains no migrated preference keys

#### Scenario: Numeric preferences round-trip across locales [prefs_locale]

- **WHEN** the user enters a decimal value in a decimal-comma locale and relaunches
- **THEN** the control restores the value and the stored TOML uses the standard decimal representation

### Requirement: Legacy preference migration is retryable and runs once

On upgrade the application SHALL import known preference keys from legacy
`state.json` into the new store using the existing legacy defaults and validation.
It SHALL commit TOML before removing these keys and recording completion in
session state. Session data and unrecognized JSON keys SHALL survive. A valid
user TOML file SHALL take precedence over legacy preferences. On a failure before
commit the application SHALL retain the source for a later retry and use its
readable preferences in memory if there is no valid TOML file. An interrupted
cleanup SHALL be retryable without overwriting committed preferences. After
completion, deleting `prefs.toml` SHALL seed defaults even when an older build
has written preference keys back into session state.

#### Scenario: A mixed legacy profile is split [prefs_migrate]

- **WHEN** an existing state file contains global preferences, shortcut overrides, a dock layout, recent files, and an unrecognized key
- **THEN** recognized preferences are restored from TOML on the next launch and the state and unrecognized key survive in JSON

#### Scenario: A failed or interrupted upgrade can be retried [prefs_migrate_retry]

- **WHEN** writing TOML fails, or startup is interrupted after TOML commits and before JSON cleanup
- **THEN** the first failure leaves the source available, and retry after either failure yields the split while respecting committed TOML values

#### Scenario: Deletion after migration restores defaults [prefs_deleted]

- **WHEN** the user deletes the TOML file after completing migration, with a last-good backup and stale legacy keys still present
- **THEN** the launched application uses defaults and retains saved session state

### Requirement: Preference writes preserve valid data

The store SHALL atomically replace the user document and retain one previous
validated copy. A failed write SHALL keep the prior complete document and warn
without preventing in-memory use. Unchanged stores SHALL avoid rewriting.
Rewriting a supported schema SHALL preserve unrecognized TOML values, including
within known tables. The store SHALL serialize competing writes and merge
explicit edits into the current validated file so unrelated changes from another
instance survive. The later committed edit SHALL win for the same field.
A supported older schema SHALL run ordered migrations. A newer schema SHALL
warn, load compatible values read-only, and preserve the original bytes.

#### Scenario: A rewrite retains unknown values and unrelated concurrent changes [prefs_merge]

- **WHEN** two instances edit different fields in a profile with unfamiliar values at the root and inside a known table
- **THEN** both changes and unrecognized values survive in valid TOML

#### Scenario: An unchanged launch keeps the same file [prefs_clean]

- **WHEN** the app opens and closes a supported user store without a preference change
- **THEN** neither the user file nor the backup is rewritten

#### Scenario: An unwritable store is non-fatal [prefs_unwritable]

- **WHEN** the preferences directory cannot be written or commit fails during save
- **THEN** the app keeps the effective preference in memory, warns, and leaves the previous valid file intact

#### Scenario: A newer schema is read-only [prefs_future]

- **WHEN** a preferences file declares a schema newer than the application understands
- **THEN** the application warns, loads readable known values, and retains the source bytes across a normal launch and quit

### Requirement: Damaged preferences are quarantined and recovered

On a syntax or type validation failure in the user file, the application SHALL
preserve the failing bytes as `prefs.toml.corrupt-<epoch>` or a unique variant
and use the last validated `prefs.toml.bak` if available. It SHALL otherwise
load defaults. A writable launch SHALL recreate a valid user file. The app SHALL
log a warning and show one notice per recovery incident on interactive startup.
Recovery SHALL NOT restore agent control opt-in or grants from the backup or system defaults: it SHALL disable agent control and reset its access level, approval mode, and allowed folders to restrictive defaults until the user explicitly grants access again. The recovered document SHALL persist that safe state and a local recovery guard. While guarded, staged preference imports and retries SHALL apply ordinary preferences without enabling agent control or restoring grants, and SHALL preserve the guard across both version 1 patches and version 2 replacements. Portable exports SHALL omit the guard. Only a fresh explicit user grant SHALL clear it, atomically with that grant under the preference write lock. If quarantine or safe-state persistence is blocked, the application SHALL preserve the original file, use the safe state and guard in memory on every recovery attempt, and SHALL NOT apply an import that could persist agent access.

#### Scenario: A damaged store is preserved and the app starts [prefs_quarantine]

- **WHEN** `prefs.toml` is malformed, with either a valid backup or no valid backup
- **THEN** the original bytes are available in quarantine, the app uses the backup or defaults respectively, and one startup notice reports the recovery

#### Scenario: A failed quarantine preserves the source [prefs_quarantine_failure]

- **WHEN** the user store is malformed and quarantine cannot be written
- **THEN** startup uses the available fallback in memory, warns, and leaves the original bytes intact

#### Scenario: Recovery cannot revive revoked agent access [prefs_recovery_agent_closed]

- **WHEN** agent access was enabled and then disabled, and the current preferences file becomes corrupt while the backup and a previously staged version 1 or version 2 import contain enabled settings, including when import application later retries and the profile is reset
- **THEN** recovery and import restore ordinary preferences but disable agent access and clear grants, and retries, reset and later normal launches remain disabled until a fresh explicit user grant

### Requirement: Startup reset offers a combined preferences and workspace reset

On an interactive launch before preference and workspace values are applied,
holding `Ctrl+Alt+Shift` SHALL offer a reset where the window system exposes
the held modifiers on startup. `--reset-preferences` SHALL offer the same
confirmation and SHALL work on Wayland. Cancelling SHALL leave files untouched
by the reset. Confirming SHALL back up the affected data and clear user
preferences, color settings, custom shortcut overrides, named workspaces, and
the active dock arrangement so the factory workspace can load. Reset SHALL
preserve recent files, window geometry, presets, and recovery snapshots. A
confirmed reset SHALL prevent a staged import or last-good backup from
automatically restoring reset values. Reset SHALL preserve any active local agent recovery guard and its restrictive agent overrides until a fresh explicit user grant. The app SHALL refuse with a warning if
another instance uses either affected store or the reset cannot be safely
completed, and a failed reset SHALL leave the prior profile available. Before publishing reset changes the application SHALL durably register the transaction and both affected roots at each root. Any launch sharing either root SHALL discover pending recovery, exclusively lock both recorded roots, and complete recovery before loading affected settings or taking normal shared presence locks. A launch SHALL recheck for recovery records after acquiring shared locks and release them to recover if needed. Under both exclusive locks, an incomplete registration proven to precede publication SHALL be discarded. If a root is unavailable or transaction state is ambiguous, startup SHALL refuse to load the affected profile until recovery can safely complete.

#### Scenario: The held modifiers open the confirmation [prefs_reset_chord]

- **WHEN** the user holds `Ctrl+Alt+Shift` while launching on a desktop that reports held modifiers before the main window loads
- **THEN** the reset confirmation appears before stored configuration takes effect

#### Scenario: The flag uses the same confirmation and Cancel keeps settings [prefs_reset_cancel]

- **WHEN** the user starts with `--reset-preferences` and cancels
- **THEN** the app loads their existing preferences and workspace normally

#### Scenario: Confirming reset restores the factory profile [prefs_reset_confirm]

- **WHEN** the user confirms reset with customized settings, a legacy session, a last-good backup, and a pending import
- **THEN** preferences, color settings, shortcuts, and workspace arrangements return to defaults across launches, the reset data is recoverable, and the recent files, window geometry, presets, and recovery snapshots survive

#### Scenario: A running instance or failed reset keeps the profile [prefs_reset_refused]

- **WHEN** the user requests reset while a second instance holds either profile root, or a write fails during reset
- **THEN** the application warns and the prior profile remains available

#### Scenario: Launch resumes after an interrupted reset [prefs_reset_interrupted]

- **WHEN** a reset is interrupted after publishing only part of the changes
- **THEN** the next launch recovers a complete profile before applying preferences and workspace layout

#### Scenario: Concurrent launches serialize interrupted reset recovery [prefs_reset_parallel_recovery]

- **WHEN** two processes start concurrently after a reset was interrupted partway through publication, including a launch sharing only one affected root
- **THEN** they serialize recovery under exclusive locks on both recorded roots and neither loads a partial profile

### Requirement: Isolated launches isolate preferences as well as session state

The self-test and each Qt Test suite SHALL use temporary user preferences and
session roots, and SHALL restore pre-existing overrides for scoped fixtures.
An explicit `--state-home DIR` SHALL also isolate user preferences under `DIR`
so existing automation continues to launch independently of the user's profile.

#### Scenario: Isolated launches never use the real preferences file [prefs_isolation]

- **WHEN** self-test, a Qt Test suite, or an app launch with `--state-home` runs with a real config home already set
- **THEN** it reads and writes only its isolated user profile and leaves the real preferences file unchanged
