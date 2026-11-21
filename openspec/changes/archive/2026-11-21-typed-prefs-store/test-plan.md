# Test Plan

New test names are scheduled in tasks.md. The physical startup chord and the installed sandbox check are reviewer steps.

## ui/preference-storage

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `prefs_paths` | the application launches with empty, distinct config and state roots | it creates a versioned TOML preferences file with defaults in the config root, and any saved window and recent-file state is JSON in the state root | `tst_preferences_store::freshProfileUsesSeparateRoots` |
| `prefs_layers` | a system file specifies two known preferences and the user file overrides one | the override wins, the other system value applies, and missing fields inherit compiled defaults | `pictura-app::preference_layers_follow_precedence` |
| `prefs_round_trip` | the user changes live and restart-class preferences, then quits and launches again | the changes return with their documented apply timing and the session JSON contains no migrated preference keys | `tst_preferences_store::preferencesReachConsumersAfterRelaunch` |
| `prefs_locale` | the user enters a decimal value in a decimal-comma locale and relaunches | the control restores the value and the stored TOML uses the standard decimal representation | `tst_preferences_store::localeDoesNotChangeStoredNumbers` |
| `prefs_migrate` | an existing state file contains global preferences, shortcut overrides, a dock layout, recent files, and an unrecognized key | recognized preferences are restored from TOML on the next launch and the state and unrecognized key survive in JSON | `tst_preferences_store::mixedLegacyProfileSplits` |
| `prefs_migrate_retry` | writing TOML fails, startup is interrupted after commit, or the legacy session lock is busy while a preference edit is attempted | the source survives and retry merges the edit with all legacy values before cleanup | `tst_preferences_store::interruptedMigrationRetries`, `tst_preferences_store::busyLegacyMigrationDefersPatch`, `pictura-app::migration_guard_checks_under_preference_lock` |
| `prefs_deleted` | the user deletes the TOML file after completing migration, with a last-good backup and stale legacy keys still present | the launched application uses defaults and retains saved session state | `tst_preferences_store::deletedPreferencesStayReset` |
| `prefs_merge` | two instances edit different fields in a profile with unfamiliar values at the root and inside a known table | both changes and unrecognized values survive in valid TOML | `pictura-app::preference_edits_merge_without_losing_unknown_keys` |
| `prefs_clean` | the app opens and closes a supported user store without a preference change | neither the user file nor the backup is rewritten | `pictura-app::unchanged_preferences_do_not_rewrite_files` |
| `prefs_unwritable` | the preferences directory cannot be written or commit fails during save, including a read-only filesystem | the app keeps the effective preference in memory, warns, and leaves the previous valid file intact | `pictura-app::failed_preference_write_keeps_last_complete_file`, `pictura-app::readonly_presence_errors_allow_shared_open` |
| `prefs_future` | a preferences file declares a schema newer than the application understands | the application warns, loads readable known values, and retains the source bytes across a normal launch and quit | `pictura-app::future_preferences_remain_read_only` |
| `prefs_quarantine` | `prefs.toml` is malformed, with either a valid backup or no valid backup | the original bytes are available in quarantine, the app uses the backup or defaults respectively, and one startup notice reports the recovery | `tst_preferences_store::corruptPreferencesRecoverWithOneNotice` |
| `prefs_quarantine_failure` | the user store is malformed and quarantine cannot be written | startup uses the available fallback in memory, warns, and leaves the original bytes intact | `pictura-app::blocked_quarantine_preserves_original` |
| `prefs_recovery_agent_closed` | Enable then disable agent access, stage a v1 or v2 import with enabled grants, corrupt the current file with an enabled backup, recover, retry the import, reset and relaunch | Ordinary preferences recover and import; agent control and grants remain disabled across retries, reset and relaunch until an explicit new grant | `tst_preferences_store::corruptPreferencesCannotRestoreAgentGrants` |
| `prefs_reset_chord` | the user holds `Ctrl+Alt+Shift` while launching on a desktop that reports held modifiers before the main window loads | the reset confirmation appears before stored configuration takes effect | manual: startup modifier state needs a real desktop session and cannot be observed by the offscreen runner |
| `prefs_reset_cancel` | the user starts with `--reset-preferences` and cancels | the app loads their existing preferences and workspace normally | `tst_preferences_reset::startupFlagCancelKeepsProfile` |
| `prefs_reset_confirm` | the user confirms reset with customized settings, a legacy session, a last-good backup, and a pending import | preferences, color settings, shortcuts, and workspace arrangements return to defaults across launches, the reset data is recoverable, and the recent files, window geometry, presets, and recovery snapshots survive | `tst_preferences_reset::startupFlagConfirmRestoresFactoryProfile` |
| `prefs_reset_refused` | the user requests reset while a second instance holds either profile root, or a write fails during reset | the application warns and the prior profile remains available | `tst_preferences_reset::resetRefusesBusyProfileOrWriteFailure` |
| `prefs_reset_interrupted` | a reset is interrupted during registration, publication, or after rollback before marker retirement, including a launch sharing one root | recovery preserves unrelated newly created files and completes rollback before applying preferences and workspace layout | `tst_preferences_reset::interruptedResetRecoversBeforeLoad`, `tst_preferences_reset::incompleteRegistrationPreservesOverlappingProfile`, `tst_preferences_reset::completedRollbackDoesNotReplay` |
| `prefs_reset_parallel_recovery` | Start two processes after partial reset publication, one sharing only one root | Both wait for exclusive recovery of both roots and neither loads a partial profile | `tst_preferences_reset::concurrentInterruptedResetRecoverySerializes` |
| `prefs_isolation` | self-test, a Qt Test suite, or an app launch with `--state-home` runs with a real config home already set | it reads and writes only its isolated user profile and leaves the real preferences file unchanged | `tst_preferences_store::isolatedLaunchesProtectRealConfig` |

## document/workspace-persistence

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Session state is restored on launch | the application starts and a valid session state exists | the saved layout and panel-column state are applied, alongside the brightness from the preferences store | `tst_session_store::sessionAndPreferencesRestoreIndependently` |
| Missing session state | no session state file exists | the application starts with the default layout and the brightness from the preferences store | `tst_session_store::missingOrCorruptSessionKeepsPreferences` |
| Corrupt session state | the session state file cannot be parsed | the application starts with default session state, retains valid preferences, and does not crash | `tst_session_store::missingOrCorruptSessionKeepsPreferences` |
| Atomic write | the session state is saved | the store is written to a temporary file and renamed over the previous store | `tst_session_store::failedSessionWriteKeepsPreviousFile` |
| `m41_session` | a schema-4 store with none of the version-5 fields is loaded | the panel-column mode is `normal`, auto-collapse and auto-show are off, and the existing keys are preserved | `tst_floating_workspaces::schema4StoreKeepsDefaultGroups` |
| `m43_session` | a multi-column layout with floating groups and columns is saved and the store is reloaded | each column's side, order, width, rail mode, floating state, overlay geometry, and group state are restored | `tst_floating_workspaces::restartRestoresFloats` |
| `m43_session` | a schema-5 store with a top-level per-group state and no per-column layout is loaded | the workspace shows one right-hand column containing those groups and the existing keys are preserved | `tst_session_store::legacyPanelLayoutDefaultsSurviveSplit` |
| `lpr_v7` | a schema-6 store whose per-column entries carry no width is loaded | each column loads the default width and the legacy top-level `railWidth` seeds the primary column's width | `tst_session_store::legacyPanelLayoutDefaultsSurviveSplit` |
| `lpr_v8_railmode` | a schema-7 store whose per-column entries carry no `railMode` is loaded | each column loads the legacy top-level `panelRailMode` as its mode | `tst_session_store::legacyPanelLayoutDefaultsSurviveSplit` |
| `lpr_column_mode` | a non-primary column's `normal`/`iconic` mode is changed, the application is quit, and it is relaunched | each column's mode after launch equals its stored mode, measured after the first layout | `tst_session_store::columnRailModePersistsIndependently` |
| `spr_stale_refused` | a second launch writes the store after a first launch started, and the first launch then saves | the first launch's save is refused and the second launch's store is byte-identical afterwards | `tst_session_store::anotherInstancesNewerWriteIsRefused` |
| `spr_stale_adopted` | the store's writer token is another launch's but its write time predates this launch's start | the save succeeds and the store carries this launch's token | `tst_session_store::aStoreFromBeforeTheLaunchIsAdopted` |
| `spr_legacy_adopted` | the store carries no writer token | the save succeeds and stamps the store with this launch's token | `tst_session_store::ownWritesRoundTrip` |
| `spr_floats_group` | a panel group is floating, the application is quit, and it is relaunched | the group is floating again at its stored position and size | `tst_floating_workspaces::restartRestoresFloats` |
| `spr_floats_column` | a whole widget column is floating, the application is quit, and it is relaunched | the column is floating again at its stored position and size | `tst_floating_workspaces::restartRestoresFloats` |
| `spr_floats_tools` | the Tools column is floating, the application is quit, and it is relaunched | the Tools column is floating again with its content-driven size at its stored position | `tst_floating_workspaces::toolsFloatRestores` |
| `spr_floats_legacy` | a session store written before the floating layout fields is loaded | every panel and column opens docked and no overlay is created | `tst_floating_workspaces::legacyStoreDocksEverything` |
| `spr_floats_clamped` | a stored overlay rectangle is larger than the available screen or frame, or smaller than the overlay minimum | the restored overlay is clamped to the available bounds and minimum before it is shown | `tst_floating_workspaces::restoredFloatIsClamped` |
| `spr_floats_startup_position` | a floating group or column is stored on the right of a frame whose window geometry is restored on launch, including a maximized frame | after relaunch the overlay is placed against the restored frame geometry and keeps its stored position instead of being clamped against the pre-restore window size | `tst_floating_workspaces_restart::restartRestoresFloatAtRightOfRestoredFrame` |

## ui/settings-backup

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `sb_export_sections` | the person exports after saving preferences and a session, creating a workspace, saving an adjustment preset, installing an `.atn` set, and saving a PDF preset | the file contains the preferences TOML, session object, workspace index and files, and every preset file under `presets` | `tst_settings_backup::exportCarriesEveryStore` |
| `sb_export_unknown_keys` | `state.json` contains a key this build does not know | the exported `session` object contains that key unchanged | `tst_settings_backup::unknownSessionKeysSurvive` |
| `sb_export_excludes_caches` | the state directory contains `gpu-backend.json`, `gpu-init.marker`, and a `recovery/` directory | the exported file contains none of them | `tst_settings_backup::exportExcludesCaches` |
| `sb_current_lists_export_failure` | a current-list file is unreadable or the complete backup exceeds the supported import size | export reports failure and leaves any existing destination intact | `tst_settings_backup::currentPresetListsExportFailure` |
| `sb_import_rejects_malformed` | the person chooses a file that is not a JSON object with the `pictura-settings` format | the import reports an error, stages nothing, and leaves every store unchanged | `tst_settings_backup::malformedFileIsRefused` |
| `sb_import_rejects_newer` | the file's `formatVersion` is greater than this build's version | the import reports that the file was written by a newer version and stages nothing | `tst_settings_backup::newerFormatIsRefused` |
| `sb_import_rejects_unsafe_names` | a workspace or preset entry names a path with a separator or `..` | the import refuses the file and writes nothing outside the store directories | `tst_settings_backup::unsafeNamesAreRefused` |
| `sb_import_backs_up` | a valid import is staged | a timestamped `pictura-*.json` backup of the current settings exists in the state directory's backup folder | `tst_settings_backup::importBacksUpCurrentSettings` |
| `sb_import_sanitizes` | the imported session lists a recent file that is missing and a window geometry that is off every screen | the pending session drops the missing recent entry and the geometry | `tst_settings_backup::importSanitizesMachineSpecificValues` |
| `sb_import_stages` | a valid file is imported | a pending import file exists, the live preferences and session stores are unchanged, and the application offers a restart | `tst_settings_backup::validImportIsStaged` |
| `sb_current_lists_absent` | a backup without the current-list section or a section missing one type is imported and applied | the corresponding current lists remain unchanged | `tst_settings_backup::currentPresetListsAbsent` |
| `sb_current_lists_invalid` | a current-list section is malformed, oversized, or cannot be staged | import reports failure and preserves both the live stores and any previous pending import | `tst_settings_backup::currentPresetListsInvalid` |
| `sb_prefs_invalid` | a selected backup contains malformed TOML or an unsupported preference schema | the import reports an error and leaves live settings and any earlier pending import intact | `tst_settings_backup::invalidPreferencesLeavePendingImportIntact` |
| `sb_apply_all` | a pending import contains preferences, session state, workspaces, and presets | the next launch writes all of them and removes the pending file | `tst_settings_backup::pendingImportAppliesEverySection` |
| `sb_apply_absent` | a pending import contains only the session section | the preferences, workspace, and preset stores keep their current contents | `tst_settings_backup::absentSectionsAreLeftAlone` |
| `sb_apply_corrupt` | the pending file is unreadable or invalid | it is renamed to a `.corrupt-<epoch>` file and the launch continues with the existing settings | `tst_settings_backup::corruptPendingIsQuarantined` |
| `sb_round_trip` | Export an unguarded profile with an unknown TOML date value, change the stores, and apply the backup to an unguarded profile without sanitization changes | Portable user TOML values including the date, session state, workspaces, and presets match the export | `tst_settings_backup::roundTripRestoresEveryStore` |
| `sb_current_lists_retry` | a staged current-list replacement fails during startup | the pending import remains, startup reports the error and loads the previous complete generation, and a later successful retry publishes all present lists together | `tst_settings_backup::currentPresetListsRetry` |
| `sb_current_lists_round_trip` | populated, default and explicitly empty current lists are exported, then the destination edits each type, and the backup is imported and applied after restart | all eight lists have their exported contents, including the formerly default list and explicit emptiness, and the pre-import backup contains the prior lists | `tst_settings_backup::currentPresetListsRoundTrip` |
| `sb_prefs_legacy` | a partial version 1 backup carries preferences without a session schema version, or is applied while legacy migration is blocked | supplied preferences are restored without losing existing legacy values; a blocked migration leaves the import pending | `tst_settings_backup::legacyBackupAppliesPresentPreferencesOnly`, `tst_settings_backup::legacyBackupWaitsForMigration` |
| `sb_prefs_retry_edits` | For v1 and v2, interrupt after preferences commit or fail another section or pending removal, edit preferences, then retry | Receipt prevents replay and the later edits survive while remaining work completes | `tst_settings_backup::preferenceImportReceiptPreservesLaterEdits` |
| `sb_prefs_receipts_shared_config` | Apply distinct pending imports from two state roots sharing config, including an old v1 pending file without an ID; retry the first after the second and later edits | Each receipt persists until its own pending file is durably retired; retry does not replay the first preferences | `tst_settings_backup::preferenceImportReceiptsSurviveSharedConfig` |
| `sb_prefs_retry` | startup cannot commit the preferences from a pending import | the prior complete preferences file is retained and a later writable startup retries the pending import | `tst_settings_backup::failedPreferencesImportRetries` |

## tools/tool-framework

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `m40_session` | the application preference is read with no saved value | `Use Shift Key For Tool Switch` is on | `tst_preferences_store::shiftSwitchDefaultsToOn` |
| `m40_session` | the preference is changed, the session is saved, and the store is reloaded | the changed value is restored alongside the other session fields | `tst_preferences_store::shiftSwitchPersistsInPrefs` |
| `m40_session` | a schema-3 store with neither new field is loaded | the column count is 1 and the preference is on | `tst_preferences_store::legacyShiftSwitchDefaultsToOn` |

## ui/keyboard-shortcuts-editor

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ks_ok_applies` | `File > New…` is given `Ctrl+Alt+N` and OK is pressed | the File > New action's shortcut is `Ctrl+Alt+N` | `tst_keyboard_shortcuts::okAppliesCancelDiscards` |
| `ks_cancel_discards` | a shortcut is changed and Cancel is pressed | every action keeps its previous shortcut | `tst_keyboard_shortcuts::okAppliesCancelDiscards` |
| `ks_persisted` | File > New is given a second binding with OK and the main window is recreated | the new window's action carries both bindings, and the preferences store holds only that override | `tst_keyboard_shortcuts::overridesPersistAcrossLaunch` |

## compositing/gpu-compute-backend

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `gb_no_gpu_unchecked` | a launch has no usable adapter and no safe-mode reset path, and the Performance preferences page is opened | Use GPU Compute is unchecked and disabled, and saving preferences records `gpuCompute` false | `tst_gpu_backend::unavailableGpuWritesOffToPreferences` |
| `gb_safe_mode_stored` | the candidate chain ended in safe mode and the stored `gpuCompute` is true | the toggle remains enabled and checked, and saving preferences keeps `gpuCompute` true | `tst_gpu_backend::safeModeKeepsStoredGpuPreference` |
| Round-trip | `gpuCompute` is set to false and the preferences store is loaded again | the loaded `gpuCompute` is false | `pictura-app::gpu_preference_round_trips_through_toml` |
| Missing field defaults true | a legacy schema-1 session without a `gpuCompute` field is migrated and no system setting overrides the default | `gpuCompute` is true | `tst_preferences_store::legacyGpuDefaultSurvivesMigration` |

## interop/agent-permissions

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ap_off_by_default` | a preferences store has no agent control keys | no endpoint exists | `tst_control_live::offByDefault` |
| `ap_live_start_stop` | agent control is enabled, then disabled | a client can connect after the first, and the endpoint and token are gone after the second | `tst_control_live::startsAndStopsAtRuntime` |
| `ap_live_settings_persist` | settings are changed and a new `AgentControl` loads the store; another instance revokes a grant before an unrelated edit | the settings persist and the unrelated edit does not restore the revoked grant | `tst_control_live::settingsPersist`, `tst_control_live::concurrentRevocationSurvivesUnrelatedEdit` |
| `ap_second_window` | a second `AgentControl` starts while one is serving | it reports the conflict and the first keeps serving | `tst_control_live::secondWindowDoesNotTakeOver` |
| `ap_dead_holder` | a lock file is left behind by a process that is gone | agent control starts | `tst_control_live::deadHolderDoesNotBlock` |
| `ap_sandbox` | the app runs under Flatpak or Snap and agent control is enabled | it serves | `tst_control_live::sandboxedBuildsStart` |
| `ap_sandbox_endpoint` | the app runs under Flatpak | the endpoint is under `$XDG_RUNTIME_DIR/app/$FLATPAK_ID` | `tst_control_live::flatpakInstancesShareTheEndpoint` |
| `ap_endpoint_outside_flatpak` | the app runs under Snap or outside a sandbox | the endpoint is under `$XDG_RUNTIME_DIR/pictura` | `tst_control_live::otherBuildsKeepTheirEndpoint` |
| `ap_sandbox_round_trip` | a window serves in an installed Flatpak or strictly confined Snap, and the advertised command runs outside the sandbox | it completes an MCP exchange with that window, with only protocol on stdout | manual: requires an installed Flatpak or strictly confined Snap, with `scripts/check-sandbox-mcp.sh` covering the packaged CI run |

| `ap_path_roots` | Open inside an allowed folder | The file opens without asking | `tst_control_approval::insideARootDoesNotAsk` |
| `ap_path_outside_headless` | Headless control opens outside allowed roots | The request is refused | `tst_control_policy::headlessRefusesOutsideItsRoots` |
| `ap_path_symlink_escape` | Open through a symlink escaping an allowed root | The path is judged outside | `tst_control_policy::symlinkDoesNotEscape` |
| `ap_path_sibling_prefix` | Judge a sibling path sharing a root prefix | The path is judged outside | `tst_control_policy::siblingPrefixIsNotInside` |
| `ap_hard_deny` | Target the state directory in Allow all mode | The request is refused | `tst_control_policy::protectedDirectoriesAreRefused` |
| `ap_approval_matrix` | Judge inside, outside, overwrite and known targets in each mode | Verdicts match the approval rules | `tst_control_policy::approvalModesDecide` |
| `ap_config_hard_deny` | Live and CLI control target a distinct config root directly and via symlink using each operation and approval mode | Every request is refused | `tst_control_live::separateConfigRootIsHardDenied`, `tst_control_policy::separateConfigRootIsHardDenied` |

## document/raw-settings-storage

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `raw_store_preference` | Sidecar ".XMP" Files is chosen in Preferences and the session is saved and loaded | the coordinator's store is Sidecar, and a fresh session's store is Camera Raw Database | `tst_raw_open::settingsStorePreferencePersists` |
| `raw_store_database_by_content` | settings with exposure 0.75 are saved to the database for `shot.dng`, and the same bytes are decoded from `moved.dng` in another folder | the record file is `<sha256>.xmp` in the database folder, no sidecar is written, the DNG's bytes are unchanged, and the seed of `moved.dng` has exposure 0.75 | `pictura-app::database_record_follows_content` |

## ui/type-preferences

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `type_pref_defaults` | the Type preferences page is opened without any prior change | Show Asian Text Options is off, Middle Eastern is off, and Enable Missing Glyph Protection is on | `tst_type_preferences::defaults` |
| A change applies to open documents | the Middle Eastern setting is turned on while a type layer is open | the Paragraph panel exposes its direction control without recording a history state | `tst_type_preferences::engineTogglesPanelVisibility` |
| The settings survive a restart | the three settings are changed and the session is saved and loaded again | each setting reads back its changed value and the renderer reports the saved glyph-protection value | `tst_type_preferences::persists` |

## ui/update-preferences

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `up_dont_check_disables` | the person unchecks **Check for updates automatically** | the preferences store records automatic checks as off and the Preferences checkbox follows | `tst_update_preferences::dontCheckDisablesAutoChecks` |
| `up_reenable` | the person turns automatic checks on in Preferences | the next launch checks for updates again | `tst_update_preferences::reenableFromPreferences` |

## Desktop verification

On 2026-11-21, the built app displayed the Reset Preferences confirmation with
Ctrl+Alt+Shift held at launch under X11 (Xvfb). The
`--reset-preferences` flag displayed the same confirmation under a virtual
KWin Wayland session. Each launch used an isolated profile.

The profile startup helper also loaded default preferences successfully with
`XDG_CONFIG_HOME` on the read-only `/sys` mount and a writable temporary state
root.
