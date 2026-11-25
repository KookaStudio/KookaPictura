# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `rp_panel_inset` | Build the dialog and read the Basic panel's layout margins. | The left margin is at least twelve pixels and the right at least eight. | `tst_acr_dialog::panelContentIsInset` |
| `rp_effects_icon` | Show the dialog and read the tab strip and the right of the tab row. | Eight tabs, the Effects tab is index 6 with the fx icon, and no `acrSettingsButton` exists. | `tst_acr_dialog::effectsTabUsesFxIcon` |

## Verification record

- CMake configure + build: passed.
- `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_(acr_dialog|acr_settings|raw_open|pictura_raw|theme_apply)$'`: 5/5 passed.
- `cargo nextest run -p pictura_app build_config`: 20/20 passed.
- `cargo clippy -p pictura_app --all-targets -- -D warnings`: clean.
- `cargo fmt --all --check`: clean.
- `bash scripts/check-file-size.sh`: OK (5 files over budget, all allowlisted).
- `python3 scripts/check-cmake-source-fragments.py`: OK.
- `bash scripts/guard.sh`: OK.
- `openspec validate --all --strict`: 532 passed, 0 failed.
- `cargo nextest run --workspace --no-fail-fast`: 6892/6893 passed; the one failure is the pre-existing environment-dependent `pictura-render::oracle imagemagick_fixtures_reproduce` (`ramp_Divide.rgba`), which no file in this change touches.
