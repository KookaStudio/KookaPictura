# Tasks

## 1. Foundations and refactors

- [x] 1.1 Split the picker math into `color_picker_math.{h,cpp}`; the dialog stays at 1041 lines (under the 1200 cap) and `tst_color_picker` passes.
- [x] 1.2 Add `crates/pictura-app/src/cxxqt_object/color_management.rs` with its own bridge for picker/profile free functions; include the generated header where needed; verify the app builds and no existing bridge file grows past its cap.
- [x] 1.3 Add `profile_warning_dialog.{h,cpp}` and `save_options_dialog.{h,cpp}` and their `CMakeLists.txt` entries; verify the app builds and the empty dialogs are constructible in a Qt Test.
- [x] 1.4 Add `pictura_color::ConversionOptions` (intent, BPC, dither) and a deterministic 8-bit ordered-dither helper; add `pictura_color::convert_options_*` unit tests for intent/BPC/dither and verify `cargo test -p pictura-color`.
- [x] 1.5 Add profile-pair RGB↔CMYK conversion tests around the existing `coated_cmyk` profile; verify `pictura_color::cmyk_round_trip_*` passes and records the lcms2 tolerance.
- [x] 1.6 Add `OpenOptions` and `read_psd_with_options` as a thin wrapper over the existing read path; verify the PSD tests still pass before behaviour changes.
- [x] 1.7 Add the tests named by `test-plan.md` that do not exist yet, before their implementation tasks, so each later task can turn its row green.

## 2. CMYK profile-aware PSD read and write (#809)

- [x] 2.1 Add a codec helper that extracts a CMYK profile from resource `1039` and falls back to the working profile; verify `pictura_codec::cmyk_profile_source_*` covers embedded, working, and absent.
- [x] 2.2 Decode a CMYK composite and every layer through the embedded/working profile in `normalize`; add `pictura_codec::cmyk_profile_read_*` and the profile-bearing `cmyk_profile.psd` fixture/lcms2 oracle; verify `cargo test -p pictura-codec`.
- [x] 2.3 Apply the RGB and CMYK incoming policies through `read_psd_with_options`; add `pictura_codec::cmyk_policy_*` for Preserve/Convert/Off and update the existing RGB policy tests.
- [x] 2.4 Re-encode an edited CMYK composite and layer through the document profile on write; add `pictura_codec::cmyk_profile_write_*` and keep the profile-free fallback test for untagged files.
- [x] 2.5 Attach the working CMYK profile to resource `1039` when `Image > Mode > CMYK` converts an RGB document; verify `pictura_render::mode_profile_*`.
- [x] 2.6 Add the profile-bearing fixture generator to `scripts/` (stable bytes) and update `color_mode_oracle.rs`; verify the oracle uses lcms2 for the profiled fixture and `psd-tools` only for the profile-free one.
- [x] 2.7 Update the `codec/psd-color-modes` golden expectations and the parity register row for #809; verify `cargo test -p pictura-codec` and `cargo test -p pictura-render`.

## 3. Color Settings and persisted conversion options (#569)

- [x] 3.1 Extend `SessionState` with the working CMYK choice, RGB/CMYK policies, intent, BPC, dither, mismatch/missing ask flags, and picker choice; add load/save normalization tests.
- [x] 3.2 Add the Rust view settings, setters/getters, and the global picker settings accessor; verify `cargo test -p pictura-app` and the bridge round-trip tests.
- [x] 3.3 Rebuild `ColorSettingsDialog` with the fixed sRGB RGB space, CMYK working-space combo, RGB/CMYK policy combos, ask toggles, conversion options, and the existing More Options text gamma; add `tst_color_settings` cases and verify `ctest -R '^tst_color_settings'`.
- [x] 3.4 Apply the stored settings to open, Convert to Profile, and the picker; verify `tst_color_settings::settingsReachOpen` and the existing `tst_text_gamma_fonts` after updating its expectations.
- [x] 3.5 Update the `color_settings_policy` self-test expectations (or retire the check if the Qt Test replaces it); verify `./build/pictura --headless --self-test` and the self-test budget gate.
- [x] 3.6 Add session round-trip cases for every new preference; verify `tst_color_settings::preferencesSurviveRestart`.

## 4. Convert to Profile options, flatten, and preview (#568)

- [x] 4.1 Add `convert_document_with(doc, dst, options, flatten)` and thread intent/BPC/dither through the 8-bit and native paths; add `pictura_codec::convert_options_*` and `pictura_codec::convert_native_options_*`.
- [x] 4.2 Rebuild `ProfileDialog` with the source-space readout, destination combo, conversion options, Flatten Image, and Preview; add `tst_profile_dialog` cases and verify `ctest -R '^tst_profile_dialog'`.
- [x] 4.3 Implement the Convert preview through the existing mode-preview mechanism; add `tst_profile_dialog::preview*` and verify cancel restores the pre-preview document.
- [x] 4.4 Keep one Convert to Profile history state for both plain and previewed commits; add `tst_profile_dialog::commitAddsOneState`.
- [x] 4.5 Update the `profile_assign_convert` self-test expectations and the existing bridge history tests; verify `cargo test -p pictura-app` and the self-test.
- [x] 4.6 Document the RGB-only Convert ceiling in the parity register; add the row update in the same commit.

## 5. Profile warnings (#568/#569)

- [x] 5.1 Add a PSD profile probe (header + image resources only) with hostile-input tests; verify `pictura_codec::probe_profile_*`.
- [x] 5.2 Add JPEG `APP2`, TIFF `34675`, and PNG `iCCP` profile probes with malformed/truncated tests; verify `pictura_codec::raster_profile_*`.
- [x] 5.3 Implement the mismatch and missing-profile dialogs with the CS6 choices, default off, and test drivers; add `tst_profile_warnings` cases.
- [x] 5.4 Wire the probe into the PSD open pipeline before commit and pass the per-open policy override; add `tst_profile_warnings::psd*` and verify no history is added.
- [x] 5.5 Wire the same flow into raster open and Place; add `tst_profile_warnings::raster*` and `tst_image_import` cases.
- [x] 5.6 Verify cancelling the warning aborts the open; add `tst_profile_warnings::cancelAborts`.
- [x] 5.7 Update the parity register and ROADMAP for the warning surface; verify `check-test-plan.py` sees every scenario row.

## 6. Embed Color Profile and raster ICC export

- [x] 6.1 Add the Save As options dialog and its per-document last choice; add `tst_save_options` cases for PSD/PSB/JPEG/TIFF and the PNG/WebP/BMP absence.
- [x] 6.2 Strip or keep resource `1039` in the PSD/PSB save clone; add `pictura_codec::psd_embed_toggle_*` and verify a save/reopen.
- [x] 6.3 Add JPEG APP2 and TIFF tag 34675 profile injection and independent parsing; add `pictura_codec::raster_icc_embed_*` and an ImageMagick/exiftool oracle where available.
- [x] 6.4 Add the Save As JPEG/TIFF path that preserves the working RGB samples; add `tst_save_options::workingProfilePixels` and verify Export As still normalises to sRGB.
- [x] 6.5 Keep PNG/WebP/BMP untagged and verify no toggle is offered; add `tst_save_options::unsupportedFormats`.
- [x] 6.6 Update `interop/icc-embedding` and `interop/raster-export` parity rows; verify `openspec validate --all --strict`.

## 7. Raster ICC import

- [x] 7.1 Apply the incoming policy in the raster decode edge for JPEG/TIFF/PNG; add `pictura_app::raster_import_policy_*` and verify Preserve/Convert/Off.
- [x] 7.2 Set `document_icc`/`source_icc` and the status notice for a profiled raster; add `tst_image_import::profileNotice`.
- [x] 7.3 Convert a placed non-sRGB raster to the host sRGB working space; add `tst_image_import::placeConvertsProfile`.
- [x] 7.4 Verify untagged, sRGB, and malformed profiles import cleanly; add `tst_image_import::malformedProfileIgnored`.
- [x] 7.5 Add the embedded-profile fixtures and their generation script; verify the lcms2 oracle.

## 8. Color picker alerts, CMYK, and preference (#600/#604)

- [x] 8.1 Add the Rust picker service (`picker_cmyk`, `picker_rgb`, `picker_gamut`) over the working CMYK profile; add `pictura_app::picker_service_*`.
- [x] 8.2 Add the web-safe alert cube; add `tst_color_picker::webAlert*` and verify clicking snaps.
- [x] 8.3 Add the out-of-gamut triangle and closest-CMYK swatch; add `tst_color_picker::gamutAlert*` and verify the sample matches the service.
- [x] 8.4 Make the CMYK fields editable through the working profile; add `tst_color_picker::cmykFields*` and verify the readouts update.
- [x] 8.5 Add the Preferences > General picker choice and the `getColor` branch; add `tst_color_picker::systemPickerPreference` and verify the default and restart persistence.
- [x] 8.6 Keep Color Libraries disabled and 16/32-bit fields out; add the `ponytail:` notes and the parity-register ceiling rows.

## 9. Integration and release checks

- [x] 9.1 Update `ROADMAP.md` for #568/#569/#600/#604/#809; verify the ticked/links rules in `scripts/check-roadmap.py` (or the equivalent guard).
- [x] 9.2 Update `docs/dev/doc-code-parity-gaps.md` rows IMG-16, IMG-17, CLR-3, CLR-7 and add the #809/raster ICC rows; verify the guard.
- [x] 9.3 Update `docs/dev/STATE.md` with the bundle summary and ceilings.
- [x] 9.4 Run the full gate: `openspec validate --all --strict`, `bash scripts/verify-fast.sh`, and `bash scripts/verify-full.sh`; verify the test report has no missing suites.
- [x] 9.5 Archive `color-management-bundle` with `openspec archive` and verify `scripts/check-openspec-archived.sh` passes.
- [x] 9.6 Write the PR body with the manual test-plan rows and the exact issue links; verify the PR template checks.
