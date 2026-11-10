# Test Plan — settings-json-backup-restore

One row per scenario in the change's spec delta. Rows name a test this change
creates unless marked `(existing)`.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `sb_prefs_buttons` | Open Preferences > General | Both **Export Settings…** and **Import Settings…** are present | `tst_settings_backup::preferencesOffersBothButtons` |
| `sb_export_cancel` | Open Export Settings… and cancel the chooser | No file is written and every store is unchanged | manual: the chooser is a native modal dialog |
| `sb_import_cancel` | Open Import Settings… and cancel the chooser | No pending file is staged | manual: the chooser is a native modal dialog |
| `sb_export_sections` | Write a session, workspace, adjustment preset, `.atn` set, Actions palette, and PDF preset; export | The file has `session`, `workspaces.index`, `workspaces.files`, and all four `presets` categories | `tst_settings_backup::exportCarriesEveryStore` |
| `sb_export_unknown_keys` | Put an unknown key in `state.json`; export | The exported `session` object contains the key unchanged | `tst_settings_backup::unknownSessionKeysSurvive` |
| `sb_export_excludes_caches` | Put `gpu-backend.json`, `gpu-init.marker`, and `recovery/` in the state dir; export | None appears in the file | `tst_settings_backup::exportExcludesCaches` |
| `sb_import_rejects_malformed` | Import a non-JSON file and a JSON file with the wrong `format` | Both are refused with an error and nothing is staged | `tst_settings_backup::malformedFileIsRefused` |
| `sb_import_rejects_newer` | Import a file with `formatVersion` 2 | It is refused and nothing is staged | `tst_settings_backup::newerFormatIsRefused` |
| `sb_import_rejects_unsafe_names` | Import a workspace or preset entry named `../evil` or with a separator | The file is refused and nothing outside the store dir is written | `tst_settings_backup::unsafeNamesAreRefused` |
| `sb_import_backs_up` | Stage a valid import | A timestamped `pictura-*.json` backup of the current settings exists | `tst_settings_backup::importBacksUpCurrentSettings` |
| `sb_import_sanitizes` | Import a session with a missing recent path and an off-screen geometry | The pending session drops both | `tst_settings_backup::importSanitizesMachineSpecificValues` |
| `sb_import_stages` | Import a valid file | A pending file exists, `state.json` is unchanged, and a restart is requested | `tst_settings_backup::validImportIsStaged`, `tst_settings_backup::frameStagesImportAndRequestsRestart` |
| `sb_apply_all` | Stage a full import and apply it | Session, workspaces, and presets are written; the pending file is removed | `tst_settings_backup::pendingImportAppliesEverySection` |
| `sb_apply_absent` | Stage a session-only import and apply it | Workspace and preset stores keep their current contents | `tst_settings_backup::absentSectionsAreLeftAlone` |
| `sb_apply_corrupt` | Write an invalid pending file and apply it | It is renamed to `.corrupt-<epoch>` and the stores are unchanged | `tst_settings_backup::corruptPendingIsQuarantined` |
| `sb_round_trip` | Export, change every store, import and apply | The stores equal their exported contents | `tst_settings_backup::roundTripRestoresEveryStore` |
| `sb_restart_quit` | Choose **Quit Now** after staging | The application quits, the pending file remains, and the next launch applies it | `tst_settings_backup::frameStagesImportAndRequestsRestart`; manual: the actual quit |
| `sb_restart_later` | Choose **Later** after staging | The window stays open with its current settings and the pending file remains | manual: the modal choice keeps the live window open |
