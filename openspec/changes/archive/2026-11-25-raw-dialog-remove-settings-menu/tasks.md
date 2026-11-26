# Tasks

## 1. Tests first

- [x] 1.1 Update `tst_acr_dialog::panelContentIsInset` to require a left margin of at least twelve pixels
- [x] 1.2 Update `tst_acr_dialog::effectsTabUsesFxIcon` to require eight tabs, the Effects tab at index 6 with the fx icon, and no `acrSettingsButton`
- [x] 1.3 Remove the settings-action tests: image settings, defaults, previous conversion, clear imported, XMP load/save, and the settings list

## 2. Remove the settings menu

- [x] 2.1 Remove `AcrPresetsPanel`, its source file, and the CMake source entry
- [x] 2.2 Remove the settings button, popup, session conversion, actions, file load/save, and `initialSettings_` from `PicturaRawDialog`
- [x] 2.3 Remove `pictura_raw_settings_from_xmp` and `pictura_raw_settings_to_xmp` from the `filter_tools` bridge and regenerate the verb manifest

## 3. Keep the Effects tab and inset

- [x] 3.1 Keep the eight-tab strip with the Effects tab on the fx icon and remove the settings button row
- [x] 3.2 Move the panel content inset to twelve pixels on the left and eight on the right

## 4. Docs and verification

- [x] 4.1 Update `public_docs/pictura-raw.md`, `ROADMAP.md`, and `PARITY.md` to drop the settings menu
- [x] 4.2 Run the focused Qt suites, fmt, file-size, guard, test-plan, and OpenSpec validation
- [x] 4.3 Archive the change with `openspec archive raw-dialog-remove-settings-menu`
