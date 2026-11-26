# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `rs_balance` | Render a mid-grey pixel with red shadows, blue highlights, Balance +100, then Balance -100. | The positive balance is blue-dominant; the negative is red-dominant. | `pictura-adjust::split_toning_balance_biases_the_highlights` |
| `rs_luminance` | Render dark and bright grey pixels with opposite shadow/highlight tints and compare luminance. | Both luminances are within rounding of the source. | `pictura-adjust::split_toning_preserves_luminance` |
| `rs_saturation_ramp` | Build the Split Toning panel, set Highlights Hue 167 and Shadows Hue 291, and read the slider ramps. | The Saturation ramps end in those hue colours; the Hue ramps have 13 stops. | `tst_acr_dialog::splitToningSaturationRampsFollowHue` |

## Verification record

- CMake build: passed.
- `cargo nextest run -p pictura-adjust split_toning`: 3/3 passed.
- `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_(acr_dialog|acr_settings|raw_open|pictura_raw|theme_apply)$'`: 5/5 passed.
- `cargo clippy -p pictura-adjust -p pictura_app --all-targets -- -D warnings`: clean.
- `cargo fmt --all --check`: clean.
- `bash scripts/check-file-size.sh`: OK (5 files over budget, all allowlisted).
- `bash scripts/guard.sh`: OK.
- `openspec validate --all --strict`: 532 passed, 0 failed.
- `cargo nextest run --workspace --no-fail-fast`: 6894/6895 passed; the one failure is the pre-existing environment-dependent `pictura-render::oracle imagemagick_fixtures_reproduce` (`ramp_Divide.rgba`), which no file in this change touches.
