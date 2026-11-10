# Tasks

## 1. Spec and test plan

- [x] 1.1 Write the `ui/settings-backup` ADDED delta
- [x] 1.2 Pair it with `test-plan.md`, one row per scenario
- [x] 1.3 Add the named Qt Test cases before the implementation they cover

## 2. Backup module

- [x] 2.1 Add `settings_backup.{h,cpp}` with the format constants, store paths, collector, and writer
- [x] 2.2 Add the reader, format/version validation, and sanitizer
- [x] 2.3 Add staging, the pre-import backup, and `applyPendingSettingsImport`
- [x] 2.4 Register the module in `cmake/sources/shell.cmake`

## 3. UI and startup

- [x] 3.1 Add the General-pane Export/Import buttons and request signals
- [x] 3.2 Add the frame export/import handlers, file choosers, and restart prompt
- [x] 3.3 Apply the pending import in `main.cpp` before the frame is constructed

## 4. Tests

- [x] 4.1 Add `tst_settings_backup` covering every scenario and register it
- [x] 4.2 Add the Preferences-button signal case

## 5. Docs and bookkeeping

- [x] 5.1 Tick `ROADMAP.md` and update `docs/dev/STATE.md`
- [x] 5.2 Run `openspec validate --all --strict` and `python3 scripts/check-test-plan.py`
- [x] 5.3 Build once and run the affected Qt test
- [x] 5.4 Run `bash scripts/verify-fast.sh` (the only failure is the pre-existing ImageMagick `ramp_Divide` oracle)
- [x] 5.5 Archive the change with `openspec archive settings-json-backup-restore`
