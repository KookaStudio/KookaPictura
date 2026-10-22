# Tasks

## 1. Preview display and current source

- [ ] 1.1 Add failing tests for the shared display transform (`exposure_gamma_preview_is_neutral_at_zero`, `highlight_compression_preview_rolls_off_highlights`) and for the level-0 preview (`hdr_preview_level0_changes_only_the_frame`, `hdr_preview_uses_edited_composite`).
- [ ] 1.2 Add the display transform to `pictura-adjust`, the per-view `HdrPreviewOptions` state, the retained-store staleness signal, and the 32-bit `level0` builder with `buffer_to_srgb`; make the tests pass.
- [ ] 1.3 Add `tst_hdr_preview` with `exposureChangesDisplayOnly`, `highlightChangesDisplayOnly`, `resetRestoresStoredFrame`, `non32BitCommandDisabled`, `methodGatesNumericFields`, `acceptAndCancel`, `editedDocumentUsesCurrentSource`, and `statusBarExposureIsPerView`; register the suite in `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- [ ] 1.4 Add `HdrPreviewOptionsDialog` (Method/Exposure/Gamma/Reset), the `View > 32-Bit Preview Options…` command id, and the per-view `32-Bit Exposure` status-bar slider; add the new files to `CMakeLists.txt`; make the Qt suite pass.
- [ ] 1.5 Verify `cargo nextest run -p pictura-adjust -p pictura-app` and `ctest --test-dir build -R '^tst_hdr_preview$' --output-on-failure`.

## 2. 32-bit Info readout

- [ ] 2.1 Add failing `tst_info_panel` cases `hdrReadoutShowsUnclampedSamples`, `hdrSamplerUsesFloatSource`, `hdrReadoutFallsBackWithoutFloatStore`, and `cmykDepthIndependent`.
- [ ] 2.2 Add a free-function unclamped float sampling bridge (not a new `cxxqt_object.rs` invokable) and the Info panel 32-bit path, including sampler rows and sample-size averaging; make the cases pass while keeping the existing 8/16-bit cases unchanged.
- [ ] 2.3 Verify `ctest --test-dir build -R '^tst_info_panel$' --output-on-failure`.

## 3. HDR color picker and swatches

- [ ] 3.1 Add failing `tst_color_picker` cases `hdrModeOpensFor32Bit`, `intensityScalesLinearColor`, `floatFieldsAcceptAboveWhite`, `nonFiniteInputIsClamped`, `previewSwatchesFollowStopSize`, `relativeToDocumentFoldsPreview`, `addToSwatchesKeepsFloat`, `eightBitKeepsOrdinaryPicker`, `toolboxSwatchAdoptsColour`, and `colorPanelFieldSetsSwatch`; add `tst_swatches_panel::hdrSwatchRoundTrips`, `tst_swatches_panel::hdrSwatchClickRestoresFloat`, `tst_swatches_panel::ordinarySwatchUnchanged`, and `tst_session_store::hdrSwatchPersists`.
- [ ] 3.2 Add `HdrColorPickerDialog` in new files and the `ColorState` float triple; route the foreground/background and active Color-panel swatch entry points through an active-document-aware opener; add the files to `CMakeLists.txt`; make the picker cases pass.
- [ ] 3.3 Extend `Swatch` with the optional float triple, the grid's HDR selection signal, Add to Swatches, and session serialization/restore; make the swatch and session cases pass while keeping the existing swatch cases unchanged.
- [ ] 3.4 Verify `ctest --test-dir build -R '^tst_(color_picker|swatches_panel|session_store)$' --output-on-failure`.

## 4. Integration and records

- [ ] 4.1 Update `docs/dev/doc-code-parity-gaps.md` (IMG-18, CLR-5, UI-7), `ROADMAP.md` (the #570/#602 line and the colour-picker line), and `docs/dev/STATE.md`; record the file-persisted preview, 32-bit tool colour, and non-routed picker call-site ceilings with follow-up issues. Carry `TASK-ALLOWS-DOCS`.
- [ ] 4.2 Run `openspec validate --all --strict`, `python3 scripts/check-test-plan.py`, and `bash scripts/verify-full.sh`; fix every failure.
- [ ] 4.3 Archive the change with `openspec archive hdr-32-bit-preview` and verify `scripts/check-openspec-archived.sh` passes.
