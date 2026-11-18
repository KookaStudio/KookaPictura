# Test plan — file-automate-batch

Every scenario is automated. Engine rules (naming, sources, validation) are
std tests in `pictura-script`; the per-input pipeline runs real files through
the app offscreen in `tst_batch_run`, with temporary folders and small 8x4 PNG
fixtures written by the test; the test action rotates them 90°, so a processed
file is 4x8.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `bfn_tokens` | Render `document name` + `_v` + 3-digit serial + `EXTENSION` for `Beach.PSD`, serial 7, JPEG | `beach_v007.JPG` | `pictura-script::naming_tokens_evaluate_in_order` |
| `bfn_serial_letter` | Render an uppercase serial letter at positions 0, 25, 26 | `A`, `Z`, `AA` | `pictura-script::naming_serial_letters_continue_past_z` |
| `bfn_dates` | Render each date token for 2026-03-04 | `030426`, `0304`, `20260304`, `260304`, `260403`, `040326`, `0403` | `pictura-script::naming_date_tokens_use_the_start_date` |
| `bfn_default_extension` | Render `Document Name` alone for `a.tif` as TIFF | `a.tif` | `pictura-script::naming_appends_a_missing_extension` |
| `bfn_serial_start` | Render three inputs with a 2-digit serial from 9 | `09`, `10`, `11` | `pictura-script::naming_serials_count_from_the_start_value` |
| `bfn_compat_windows` | Sanitize `a:b?c. ` with Windows on | `a_b_c` | `pictura-script::naming_windows_compatibility_sanitizes` |
| `bfn_compat_unix` | Render custom text `x/y` with optional flags off | `x_y` | `pictura-script::naming_unix_compatibility_always_applies` |
| `bfn_unique` | Validate a Folder destination with `yyyymmdd` + `extension` | Unique-field diagnostic | `pictura-script::validate_rejects_a_scheme_without_a_unique_field` |
| `bp_validate_action` | Validate options naming an absent action | Diagnostic names the action | `pictura-script::validate_rejects_a_missing_action` |
| `bp_validate_save_step` | Validate Override Save As for an action with no Save As | Save As diagnostic | `pictura-script::validate_override_save_needs_a_save_as_step` |
| `bp_validate_nested_save` | Validate Override Save As for an action that plays one with Save As | Passes | `pictura-script::validate_accepts_a_nested_save_as_step` |
| `bp_validate_folders` | Validate a missing source folder, then a missing destination folder | Each diagnostic names its folder | `pictura-script::validate_rejects_missing_folders` |
| `bp_source_folder` | Resolve a fixture tree without subfolders | `A.png`, `b.jpg` | `pictura-script::folder_source_lists_top_level_images_in_order` |
| `bp_source_recurse` | Resolve the same tree with subfolders | `A.png`, `b.jpg`, `sub/c.jpg` | `pictura-script::folder_source_includes_subfolders` |
| `bp_source_cycle` | Resolve a tree with a symlink back to the root | Terminates, each file once | `pictura-script::folder_source_survives_a_link_cycle` |
| `bp_source_opened` | Open two 8x4 documents, batch a Rotate 90° action, source Opened Files | Both rotated to 4x8 | `tst_batch_run::openedFilesSourceProcessesTabs` |
| `bp_dest_none` | Batch two files, no-save action, destination None | Both open and modified; files unchanged | `tst_batch_run::destinationNoneLeavesDocumentsOpen` |
| `bp_dest_save_close` | Batch two 8x4 PNGs with Rotate 90°, Save And Close | PNGs are 4x8 on disk, none open | `tst_batch_run::saveAndCloseOverwritesInPlace` |
| `bp_dest_folder` | Batch two files, Folder destination, name + 2-digit serial | `<n1>01.<ext>`, `<n2>02.<ext>`, none open | `tst_batch_run::folderDestinationWritesNamedFiles` |
| `bp_override_open` | Action Open `x.png` + Rotate over two inputs, override on, Save And Close | Inputs rotated on disk, `x.png` unchanged, none open | `tst_batch_run::overrideOpenProcessesInputs` |
| `bp_no_override_open` | Same action, one input, override off, destination None | `x.png` rotated; input open, not rotated | `tst_batch_run::withoutOverrideOpenTheRecordedFileIsOpened` |
| `bp_override_save_folder` | JPEG q3 Save As step, two inputs, Folder, override on | `<n>.jpg` at q3 in destination; recorded path unwritten | `tst_batch_run::overrideSaveAsWritesToTheDestination` |
| `bp_override_save_close` | Same action, Save And Close, override on | JPEG beside the input; document closed | `tst_batch_run::overrideSaveAsWithSaveAndCloseKeepsTheInputFolder` |
| `bp_no_override_save` | Same action, two inputs, None, override off | Recorded path holds the second result | `tst_batch_run::withoutOverrideSaveAsTheRecordedPathIsReused` |
| `bp_error_stop` | Three inputs, second unreadable, Stop For Errors | Stops after second; third never opened | `tst_batch_run::stopForErrorsHaltsOnTheFirstFailure` |
| `bp_error_log` | Same inputs, Log Errors To File | First and third processed; one log line; count 1 | `tst_batch_run::logErrorsContinuesAndRecords`, `pictura-script::error_log_line_names_the_input` |
| `bp_error_log_unwritable` | Log path in a missing directory | Path reported; stops at first error | `tst_batch_run::unwritableLogFallsBackToStopping` |
| `bp_cancel` | Cancel during the second of three inputs, Folder destination | Only the first output; no batch document open | `tst_batch_run::cancelStopsAfterTheCurrentStep` |
| `ae_open_record` | Record, then open `a.png` via File > Open | One `file.open` step with the absolute path | `tst_batch_run::openWhileRecordingAppendsAnOpenStep` |
| `ae_open_replay` | Play an action with an Open step | `a.png` active before the next step | `tst_batch_run::playingAnOpenStepOpensTheFile` |
| `ae_open_missing` | Play an Open step whose file is gone | Playback stops; step reported failed | `tst_batch_run::openStepForAMissingFileFails` |
| `ae_open_atn` | Write a set with an Open step to `.atn` and read it | `file.open` with the same path | `pictura-script::atn_round_trips_an_open_step` |
| `ae_saveas_record` | Record, Save As JPEG quality 4 | Step `options` holds quality 4 | `tst_save_format_options::saveAsStepRecordsTheConfirmedOptions` |
| `ae_saveas_replay` | Remembered quality 12; play a step carrying quality 4 | JPEG at quality 4; remembered stays 12 | `tst_save_format_options::saveAsStepPlaysWithItsOwnOptions` |
| `ae_saveas_legacy` | Play a Save As step without `options` | Written with remembered options | `tst_save_format_options::saveAsStepWithoutOptionsUsesRemembered` |
| `ae_saveas_atn` | Write a set with a Save As step with `options` and read it | Equal `options` | `pictura-script::atn_round_trips_save_as_options` |
| `ae_play_suppressed` | Play a dialog-on step in suppressed mode | Executes with recorded values; no dialog | `tst_actions_panel::suppressedPlaybackUsesRecordedValues` |
| `ae_play_suppressed_stop` | Suppressed playback over a continue Stop then a plain Stop | First continues silently; second fails with its message | `tst_actions_panel::suppressedPlaybackStopSteps` |
| `bd_menu` | Dispatch `file.automate.batch` with an action loaded | The Batch dialog opens | `tst_batch_dialog::commandOpensTheDialog` |
| `bd_menu_disabled` | Empty the library | `file.automate.batch` disabled | `tst_batch_dialog::commandDisabledWithoutActions` |
| `bd_controls` | Open the dialog and inspect its groups | All groups and choices present | `tst_batch_dialog::dialogShowsEveryGroup` |
| `bd_enablement` | Choose Opened Files and None | Source/destination/naming controls disabled | `tst_batch_dialog::controlsFollowSourceAndDestination` |
| `bd_naming_example` | `DOCUMENT NAME` + 1-digit serial + `extension`, start 3 | `MYFILE3.gif` | `tst_batch_dialog::exampleFollowsTheScheme` |
| `bd_invalid` | OK with Folder destination and no folder | Diagnostic; dialog stays open; no batch | `tst_batch_dialog::invalidOptionsKeepTheDialogOpen` |
| `bd_open_warning` | OK for an action with an Open step, override off; decline | Warning shown; no batch | `tst_batch_dialog::bakedInOpenWarnsBeforeRunning` |
| `bd_persist` | Accept with folder, subfolders, serial 5; reopen | Same values shown | `tst_batch_dialog::optionsPersistBetweenRuns` |
| `bd_progress` | Three-input batch, Cancel during the second | Progress showed `1 of 3`, `2 of 3`; third skipped | `tst_batch_run::progressAdvancesAndCancelStops` |
| `bd_log_message` | Log-errors batch with one error | End message states one error and the path | `tst_batch_run::loggedErrorsAreReportedAtTheEnd` |
| Dialog layout reads like CS6 | Open the Batch dialog in a real session and compare the grouping with the CS6 dialog | Groups, labels, and order match | manual: visual layout parity has no oracle offscreen |
