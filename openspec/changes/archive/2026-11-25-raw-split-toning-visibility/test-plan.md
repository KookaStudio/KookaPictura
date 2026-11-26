# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `rs_balance` | Render a mid-grey pixel with red shadows, blue highlights, Balance +100, then Balance -100. | The positive balance is blue-dominant; the negative is red-dominant. | `pictura-adjust::split_toning_balance_biases_the_highlights` |
| `rs_luminance` | Render dark and bright grey pixels with opposite shadow/highlight tints and compare luminance. | Both luminances are within rounding of the source, including a tint that would clip. | `pictura-adjust::split_toning_preserves_luminance` |
| `rs_saturation_ramp` | Build the Split Toning panel, set Highlights Hue 167 and Shadows Hue 291, and read the slider ramps. | The Saturation ramps end in those hue colours; the Hue ramps have 13 stops. | `tst_acr_dialog::splitToningSaturationRampsFollowHue` |
| `rs_highlights_only` | Render a dark and a bright grey pixel with Highlights Hue 167, Highlights Saturation 85, Shadows Saturation 0; then move only the Highlight sliders in the dialog. | The bright pixel turns green-cyan, the dark pixel is unchanged, and the dialog preview changes. | `pictura-adjust::split_toning_highlights_only_tints_the_bright_end`, `tst_acr_dialog::splitToningHighlightsChangeThePreview` |
| `rs_midtone_visibility` | Render a 60% grey with Highlights Hue 230 / Saturation 52 and a 35% grey with Shadows Hue 30 / Saturation 52. | The highlight pixel's blue channel is >40 above red; the shadow pixel's red channel is >30 above blue. | `pictura-adjust::split_toning_highlights_tint_the_upper_midtones`, `pictura-adjust::split_toning_shadows_tint_the_lower_midtones` |

## Verification record

- `cargo test -p pictura-adjust split_toning`: 6/6 passed.
- `cargo test -p pictura-adjust`: 113 passed.
- CMake build: passed.
- `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_(acr_dialog|acr_settings|raw_open|pictura_raw)$'`: 4/4 passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo fmt --all --check`: clean.
- `python3 scripts/check-test-plan.py`: 1 change checked.
- `bash scripts/check-file-size.sh`: OK (5 files over budget, all allowlisted).
- `bash scripts/guard.sh`: OK.
- `bash scripts/check-openspec-archived.sh`: OK.
- `openspec validate --all --strict`: 531 passed, 0 failed.
- The reported case was also checked against `RAW_NIKON_D7100.NEF` at Highlights Hue 230 / Saturation 52: an upper-midtone sample's blue channel rose from 34,799 to 60,996 (16-bit), visibly tinting the bell.
