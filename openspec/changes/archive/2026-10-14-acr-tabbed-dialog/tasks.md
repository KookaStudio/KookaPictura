# Tasks

## 1. Settings serialization

- [x] 1.1 Add `pictura_codec::pictura_raw_to_text()` / `pictura_raw_from_text()` in `crates/pictura-codec/src/pictura_raw/` (free functions — `PicturaRawSettings` is in `pictura-core`), using the `Fltr` 4-byte keys; verify a round-trip unit test over a settings value that sets every modelled tab (curves, HSL arrays, enums, text) reproduces it exactly.
- [x] 1.2 Implement the escape rules (in keys and values: `\`→`\\`, `;`→`\;`, `=`→`\=`, bytes `<0x20`/`0x7f`→`\xNN`; a key's trailing space is preserved, never trimmed); verify a test round-trips a value with a trailing-space key (`Crv `) and a text value carrying a trailing NUL (`CamP`).
- [x] 1.3 Verify the text form and the `Fltr` codec agree: a test asserts the key set of `pictura_raw_to_text()` matches the keys the codec writes for the same settings, so the two cannot drift.

## 2. Bridge contract

- [x] 2.1 Change `layer_pictura_raw_settings` to return the full serialized text and `apply_pictura_raw_filter(path, settings_text)` to take it in `impl_pictura_raw.rs` and `cxxqt_object.rs` (the `#[qinvokable]` signatures); update the callers in `frame_menus.cpp` and `tst_layers_smart_filters.cpp`; verify a Rust bridge test applies multi-tab settings and reads the same values back.
- [x] 2.2 Add `pictura_raw_preview(path, settings_text, max_dim)` returning a downscaled image: for a raster layer render the layer's pixels directly, for an embedded smart object the payload, without converting or mutating; verify a bridge test returns a non-null image for a raster and a smart-object layer and a null/empty result for an ineligible target.
- [x] 2.3 Migrate the bridge checks in `crates/pictura-app/cpp/selftest_pictura_raw.cpp` (exit 528) to a Qt Test suite under `crates/pictura-app/cpp/tests/`, delete `selftest_pictura_raw.{h,cpp}`, remove them from `CMakeLists.txt`, unwire `runPicturaRawChecks` from `selftest_layers_controls.cpp`, and lower `scripts/selftest-budget.txt` by one; verify `scripts/check-selftest-budget.sh` passes.

## 3. Dialog chrome

- [x] 3.1 Add `crates/pictura-app/cpp/acr/acr_settings.{h,cpp}` — the `AcrSettings` value map with `encode()/decode()` mirroring the codec grammar; verify a Qt Test parses a committed fixture string the Rust round-trip test also uses.
- [x] 3.2 Rebuild `pictura_raw_dialog.{h,cpp}` as the ACR chrome — icon tab strip (Basic first), panel host, OK/Cancel action bar, over the shared value map — and register the new files in `CMakeLists.txt`; verify a Qt Test opens the dialog and finds every tab selectable with Basic selected.
- [x] 3.3 Add the preview pane (renders via `pictura_raw_preview`, transparency checkerboard, zoom readout, shadow/highlight clipping toggles); verify a Qt Test that changing a value updates the pane image and that a clipping toggle tints out-of-range pixels.
- [x] 3.4 Add the histogram (reusing `HistogramView`) and RGB readout computed from the preview image; verify a Qt Test that the histogram series change when a control changes and that the readout reports per-channel values.

## 4. Tab panels

Each panel exposes the controls the reference screenshots list for its tab.

- [x] 4.1 Basic panel (WB dropdown, temp/tint, exposure…saturation, Auto/Default links); verify a Qt Test that each control writes its key into the value map.
- [x] 4.2 Tone Curve panel (Parametric/Point sub-tabs, `Curve`/`Channel` menus, `Input:`/`Output:`); verify region sliders and point-curve edits both reach the value map.
- [x] 4.3 Detail panel (Sharpening; Noise Reduction) with all modelled controls; verify each writes its key.
- [x] 4.4 HSL / Grayscale panel (Convert to Grayscale, Hue/Saturation/Luminance, Default link, 8 ranges); verify the range sliders write keys and the grayscale toggle reveals the mix.
- [x] 4.5 Split Toning panel (Highlights; Balance; Shadows); verify the modelled controls write keys.
- [x] 4.6 Lens Corrections panel (info banner, Distortion, Defringe, Vignetting); verify its controls write keys.
- [x] 4.7 Effects panel (Dehaze; Grain; Post Crop Vignetting, with the Style select enabled only once Amount leaves 0); verify the enable rule and that controls write keys.
- [x] 4.8 Camera Calibration panel (Process = `mode.process_version`, Shadows = `shadow_tint`, Red/Green/Blue primary hue + saturation); verify each writes its key and reads back.
- [x] 4.9 Presets panel (stub menu items); verify the menu exposes the listed items and selecting one does not corrupt the value map.
- [x] 4.10 Register every new panel `.cpp`/`.h` in `CMakeLists.txt` and its Qt Test in `crates/pictura-app/cpp/tests/CMakeLists.txt`; verify a clean CMake configure and `ctest -R '^tst_acr'` build and pass.

## 5. Integration

- [x] 5.1 Verify the `Filter > Pictura Raw…` command end to end in a Qt Test: editing controls across tabs and accepting applies the full settings in exactly one `"Pictura Raw"` history state with the settings readable back.
- [x] 5.2 Verify Cancel applies nothing and records no history state.
- [x] 5.3 Run `openspec validate --all --strict` and `bash scripts/verify-fast.sh` and confirm both pass.
