# Tasks

## 1. Shared channel-row table

- [x] 1.1 Extract the Channels-panel `channelsForMode` name/shortcut table into a header-only `channels_rows.h` and have `panels/channels_panel.cpp` include it; verify `cmake --build build --target pictura_shell tst_channels_panel` and `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_channels_panel$' --output-on-failure` still pass

## 2. Load Selection colour rows

- [x] 2.1 Write the Qt cases the test plan names (`loadDialogListsColourRows`, `loadDialogWithNoSavedChannels`, `loadDialogGrayscaleCompositeRow`, `loadDialogDefaultsToExtraChannel`, `loadDialogModeRows`, `loadDialogColourRowTarget`, `loadColourRowCoverage`, `loadColourRowCombines`, `loadColourRowCrossDocument`, `loadColourRowThroughFrame`, `selectLoadEnabledWithoutChannels`, `saveDialogExcludesColourRows`) and update `loadDialogDefaults`, replace `loadDialogDisabledWithoutSources` with `loadDialogWithNoSavedChannels`, and update `loadDialogCrossDocument`; verify `cmake --build build --target tst_selection_save_load` and that the new cases fail before the next task. For `loadDialogModeRows`, cover RGB, Grayscale, CMYK, Lab via `convert_image_mode`, and Indexed via `convert_to_indexed`; record in the case that Bitmap, Duotone, and Multichannel have no fixture (the mode bridge and the codec do not build them)
- [x] 2.2 Build the Load Channel list from the shared table — composite row, colour plates, layer masks, extra channels — with `color:<row>` targets and the first-extra-channel default (the first listed colour row when the mode has no composite); verify the listing, default, single-row-mode, no-saved-channels, and mode-rows cases pass
- [x] 2.3 Route `color:` targets in `frame_selection_dialogs.cpp` through `color_row_coverage` and `load_selection_coverage` for the same-document and cross-document paths, and reduce the `SelectLoad` provider in `menu_select.cpp` to `selectHasDoc`, refreshing the registry; drive the real command/frame path in `loadColourRowThroughFrame` (registry dispatch plus a QTimer that fills and accepts the dialog, or a minimal frame seam) and verify the plate, composite, combine, cross-document, frame-routing, and `selectLoadEnabledWithoutChannels` cases pass
- [x] 2.4 Leave Save's Channel list unchanged and keep the exclusion covered; verify `saveDialogExcludesColourRows` passes
- [x] 2.5 Run `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_selection_save_load$|^tst_channels_panel$' --output-on-failure`; the full-suite regression run is group 4

## 3. Docs and parity

- [x] 3.1 Correct the Save `Channel` wording, the no-alpha edge case, the Load `Channel` line, and the open question in `docs/08-selection/save-and-load-selections.md` (Load lists colour rows; Save does not; the first-extra-channel default is inferred) — update line 89 to record the inferred default and line 198 to close or qualify the open question — in a docs commit carrying `TASK-ALLOWS-DOCS`; verify `bash scripts/guard.sh` passes
- [x] 3.2 Update the SEL-7 ceiling in `docs/dev/doc-code-parity-gaps.md` to record the Load-side colour rows and the deliberate Save exclusion; verify the same guard run passes

## 4. Integration

- [x] 4.1 Run `openspec validate selection-colour-row-targets --strict` and confirm the change validates
- [x] 4.2 Run `python3 scripts/check-test-plan.py` and the targeted ctest suites before archiving; the plan check must run now because `scripts/check-test-plan.py` skips archived changes (`scripts/check-test-plan.py:126-130`)
- [x] 4.3 Archive the change with `openspec archive selection-colour-row-targets`
- [x] 4.4 After archiving, run `bash scripts/verify-fast.sh` and `openspec validate --all --strict`; `verify-fast.sh` runs `scripts/check-openspec-archived.sh`, which fails while the change is unarchived (`scripts/check-openspec-archived.sh:29-32`)
