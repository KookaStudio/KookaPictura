# Tasks

## 1. Shared parameter type and f32 operators

- [x] 1.1 Move `HdrToningParams` and its validation into `pictura-core` and re-export it from `pictura-filters`; verify `cargo nextest run -p pictura-core -p pictura-filters` and that the 8-bit HDR Toning output is unchanged.
- [x] 1.2 Add failing `pictura-adjust` unit tests for `each_method_changes_samples`, `equalize_histogram_keeps_uniform_input_finite`, `highlight_compression_keeps_midtones_and_maps_white`, and `bad_local_adaptation_parameters_are_refused`.
- [x] 1.3 Split `crates/pictura-adjust/src/hdr_toning.rs` into `hdr_toning/{mod,methods,local,tests}.rs` as a pure move; keep the existing Exposure & Gamma tests passing.
- [x] 1.4 Add `HdrToneMap`, `LocalAdaptationParams`, the four-method dispatch, Equalize Histogram, Highlight Compression, and the f32 Local Adaptation kernel; make the new unit tests pass.
- [x] 1.5 Re-export the new types from `pictura-adjust`/`pictura-render`; verify `cargo nextest run -p pictura-adjust -p pictura-render`.

## 2. Document conversion and source freshness

- [x] 2.1 Add failing `pictura-render` tests for `hdr_conversion_methods_convert`, `local_adaptation_leaves_alpha_untouched`, `hdr_conversion_copies_extra_planes`, `edited_document_converts_current_composite`, and `hdr_conversion_refuses_bad_parameters`.
- [x] 2.2 Add `convert_depth(doc, out, method)` plus a retained-store freshness check (compare the store's 8-bit narrowing with the composite; keep the file's merged composite on open so a clean 32-bit document is not misread as edited); flatten a converted layered document to one Background from the tone-mapped composite. Keep `convert_depth_exposure_gamma` as the Exposure & Gamma wrapper.
- [x] 2.3 Verify `cargo nextest run -p pictura-render` and `cargo clippy -p pictura-render --all-targets -- -D warnings`.

## 3. Dialog and free-function bridge

- [x] 3.1 Add the `tst_hdr_conversion` suite with `non32BitConvertsWithoutDialog`, `methodComboGatesFields`, `defaultsToLocalAdaptation`, `acceptConvertsWithSelectedMethod` (asserting method-specific pixels, not just depth), and `failedConversionRecordsNoHistory`; register the suite in `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- [x] 3.2 Extract only the nine numeric slider/spin rows from `hdr_toning_dialog.cpp` into `HdrToningControls` behind an editing signal; keep the preset menu and preview in the HDR Toning dialog; add the new files to `CMakeLists.txt`; verify the existing `tst_image_adjustments` HDR Toning cases.
- [x] 3.3 Extend `HdrConversionSpec` and `HdrConversionDialog` with the Method combo, per-method pages, and the `Default` preset values; make `tst_hdr_conversion` pass.
- [x] 3.4 Add a free-function conversion bridge carrying bits, method, and parameters in a new module (do not grow `cxxqt_object.rs`, which is at the 1200-line cap); keep `convert_depth(bits, exposure_ev, gamma)` as the wrapper. Update `frame_menus_image_mode.cpp`, `tst_image_mode.cpp:214`, and `selftest_layers_adjustments.cpp` only where the wrapper is not enough.
- [x] 3.5 Add the new source/header files to `CMakeLists.txt`; verify `ctest --test-dir build -R '^tst_(hdr_conversion|image_mode|image_adjustments)$' --output-on-failure`.

## 4. Integration and records

- [x] 4.1 Update `docs/dev/doc-code-parity-gaps.md` (IMG-5), `ROADMAP.md` (tick only the #557 half of the #557/#558 line), and `docs/dev/STATE.md`; record the Toning Curve follow-up in the register/roadmap. Carry `TASK-ALLOWS-DOCS`.
- [x] 4.2 Run `openspec validate --all --strict`, `python3 scripts/check-test-plan.py`, and `bash scripts/verify-full.sh`; fix every failure.
- [x] 4.3 Archive the change with `openspec archive hdr-conversion-methods`, update the main `color/hdr-conversion` Purpose to cover the four methods, and verify `scripts/check-openspec-archived.sh` passes.
