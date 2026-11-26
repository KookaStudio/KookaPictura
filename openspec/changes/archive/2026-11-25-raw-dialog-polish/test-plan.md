# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `rp_panel_inset` | Build a control panel and read its layout's left and right content margins. | Both are at least eight pixels. | `tst_acr_dialog::panelContentIsInset` |
| `rp_hsl_ramps` | Read the middle ramp stop of each Hue, Saturation, and Luminance range slider; read a Grayscale Mix slider's ramp. | The middle stops are red, orange, yellow, green, cyan, blue, violet, and magenta; the grayscale ramp is grey. | `tst_acr_dialog::hslPanelColorsMatchRanges` |
| `rp_effects_icon` | Show the dialog and read the tab strip and the settings button. | Eight tabs, the Effects tab is index 6, and `acrSettingsButton` exists. | `tst_acr_dialog::effectsTabUsesFxIcon` |
| `rp_settings_button` | Click the fx button, find the settings list, and choose Camera Raw Defaults. | The list is visible and the dialog's settings become empty. | `tst_acr_dialog::settingsButtonOpensList` |

## Verification record

- CMake configure + build: passed.
- `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_(acr_dialog|acr_settings|raw_open|pictura_raw|theme_apply)$'`: 5/5 passed.
- `tst_acr_dialog panelContentIsInset hslPanelColorsMatchRanges effectsTabUsesFxIcon settingsButtonOpensList`: all passed.
- `cargo fmt --all --check`: clean.
- `bash scripts/check-file-size.sh`: OK (5 files over budget, all allowlisted).
- `bash scripts/guard.sh`: OK.
- `openspec validate --all --strict`: 532 passed, 0 failed.
