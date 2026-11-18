# Tasks

## 1. Tests first

- [x] 1.1 Write the `pictura-script` batch engine tests in `crates/pictura-script/src/batch/` (`naming_*`, `folder_source_*`, `validate_*`, `error_log_line_names_the_input`) per `test-plan.md`
- [x] 1.2 Add `atn_round_trips_an_open_step` and `atn_round_trips_save_as_options` to `crates/pictura-script/tests/atn.rs`
- [x] 1.3 Add the suppressed-playback slots to `tst_actions_panel` (`tst_actions_panel_cases.cpp`); the Open-step slots need the real engine and coordinator and live in `tst_batch_run`
- [x] 1.4 Add the Save As options slots to `tst_save_format_options.cpp`
- [x] 1.5 Create `tst_batch_dialog.cpp` and `tst_batch_run.cpp` and register both in `crates/pictura-app/cpp/tests/CMakeLists.txt`

## 2. Batch engine (`pictura-script::batch`)

- [x] 2.1 `naming.rs`: `NamingField`, `FileNaming::render`, `has_unique_field`, compatibility sanitizing (D1)
- [x] 2.2 `source.rs`: `resolve_folder` with ordering, hidden-entry skip, link-cycle guard
- [x] 2.3 `options.rs`: `BatchOptions` serde model, `validate`, `action_has_save_as`/`action_has_open` through Play steps, `override_save_path`, `error_log_line`

## 3. Action engine changes

- [x] 3.1 Add `file.open` (`Opn `, `path`→`null`) to `VERIFIED`; classify it recordable in the catalogue coverage
- [x] 3.2 Record Open steps from successful interactive opens; add the replay executor (D5)
- [x] 3.3 Record `options` in Save As steps and replay a step's own options without touching remembered options (D6)
- [x] 3.4 `ActionsPanel`: `playbackFinished` signal, `playActionForBatch` with `BatchPlayback` (suppressed dialogs, Open override, Save As redirect) (D4)

## 4. App

- [x] 4.1 cxx-qt bridge `cxxqt_object/batch.rs` (D2)
- [x] 4.2 `FileNamingWidget`, `BatchOptionsWidget`, `BatchDialog` (enablement, example, validation, Open warning, persistence in `Session`)
- [x] 4.3 `BatchRunner` state machine, destinations (D7), error policy and log, progress dialog with Cancel, end-of-run messages (D3)
- [x] 4.4 Flip `file.automate.batch` to implemented, regenerate `command_ids.h` (`scripts/gen-command-ids.py`), wire the handler and enabled provider
- [x] 4.5 Register new `.cpp` files in `cmake/sources/*.cmake`

## 5. Verify and document

- [x] 5.1 Single build, then `scripts/verify-full.sh`; `python3 scripts/check-test-plan.py` passes
- [x] 5.2 `docs:` commit (`TASK-ALLOWS-DOCS`): parity register `AUTO-3`, `ROADMAP.md` line for #618, `docs/dev/STATE.md`
- [x] 5.3 `openspec archive file-automate-batch`
