# Tasks

## 1. Native render foundation (codec)

- [x] 1.1 Write the codec tests named in `test-plan.md` for native rendering (`native_rgb16_render_keeps_more_than_eight_bits`, `native_cmyk16_keeps_rendered_inks`, `native_cmyk16_is_not_the_gcr_conversion`, `native_gray16_keeps_more_than_eight_bits`, `native_render_keeps_unpainted_areas_transparent`, `native_antialias_toggles_edge_coverage`, `native_crop_matches_the_rgb8_import`, `native_image_render_keeps_more_than_eight_bits`, `native_eps_render_keeps_cmyk_inks`) plus the PSD-read spike `native_psd_device_output_reads`; verify each test is named exactly and covers its plan row before any implementation code is added
- [x] 1.2 Add the Ghostscript native PSD device arguments (`psdcmyk`, `psdcmyk16`, `psdrgb16`) and a bounded read of their output through `read_psd_with_options` with `Policy::Off`; verify `native_psd_device_output_reads` reports the expected planes and depth
- [x] 1.3 Add the coverage decode, un-composite-over-white, and native-plane crop helpers; the codec returns the cropped native planes, coverage, and warnings and does not assemble the document, which needs `pictura-render`; verify the native codec test set from 1.1 passes
- [x] 1.4 Route page, embedded-image, and EPS rendering to the native path only for CMYK or 16-bit targets, keeping RGB8/Grayscale8/Lab8 on the existing pngalpha path; verify the native tests and the existing `pdf::import` tests pass together

## 2. Images mode on encrypted PDFs

- [x] 2.1 Write `encrypted_images_open_with_the_password` and the Qt slot `encryptedImagesNeedThePassword`; verify the new test fails on the intended encrypted-bail assertion before the fix, and update the Rust fixtures that call `PdfImages::read`/`PdfImage.object` (`pdf/import/tests.rs:301,314,333,342`, `tests/hostile_inputs.rs:617`, including `encrypted_pdf_disables_images`) for the new password parameter and `Option<u32>` object
- [x] 2.2 Give `PdfImages::read` the password, decrypt through the existing `PdfObjects::decrypt`, keep the encrypted guard as a post-decrypt check, and update the `list_images`/`PdfImages::read` doc comments to the new contract; verify `encrypted_images_open_with_the_password` lists and renders the image with the right password and returns `None` with a wrong one
- [x] 2.3 Pass the password through `pdf_import_probe`, the image-thumbnail bridge, the image-open bridge, and their calls in `import_pdf_dialog.cpp` and `import_save_import.cpp`; make `PdfImage.object` an `Option<u32>` and update every remaining `PdfImages::read`/`PdfImage.object` call site (`pdf_export/import/imp.rs:75,124`, `open_jobs/imp.rs:515`); verify `tst_import_pdf::encryptedImagesNeedThePassword` shows Images enabled and opens the image

## 3. Inline images

- [x] 3.1 Write `inline_images_are_listed_after_xobjects`, `inline_image_renders_at_its_native_size`, `inline_indexed_image_renders`, `inline_images_skip_unsupported_content_filters`, the hostile test `inline_image_scan_is_bounded_and_never_panics`, and the Qt slots `inlineImagesAreListed` and `inlineImageOpens`; verify the tests are named exactly for their plan rows
- [x] 3.2 Decode page `/Contents` streams and scan `BI`/`ID`/`EI` with the abbreviated-key mapping (including `I`→Interpolate, `Mask`, and the `/CS` `I`→Indexed shorthand), filter expansion, and the `AHx`/`A85` terminator rules; verify the listing test reports XObjects then inline images in content order
- [x] 3.3 Synthesize an image stream for each inline image, extend `PdfImages` with the source list used by `render`, and enforce the `MAX_IMAGES` cap; verify the render, Qt, and hostile tests pass
- [x] 3.4 Keep unsupported content-stream filters and malformed `EI` scans from aborting XObject listing; verify the existing images tests still pass

## 4. App integration and Qt coverage

- [x] 4.1 Write the Qt slots `nativeRgb16ImportKeepsDepth`, `nativeCmyk16ImportKeepsTransparency`, and `epsNativeCmyk16`; verify they model the plan rows before wiring the native path
- [x] 4.2 Return `Imported { Rgba(Rendered), Native(NativeRender) }` from `finish_rendered`, `pdf_import_open_*`, and `eps_import_open`, and finalize the native result in `open_jobs` (document, `Layer 1`, alpha, source stores, resolution, name, history); refresh the stored composite with `DocumentComposite::new(doc).refresh(pictura_render::CompositeScope::Full, gpu)` (or clear `merged_composite_present`) before `store_opened_composite`; verify the three Qt slots and the existing `modeAndDepthApply` pass
- [x] 4.3 Verify RGB8 and Grayscale8 imports still take the single-pass path by running the existing `tst_import_pdf` suite and the `pdf::import` codec tests
- [x] 4.4 Add **Bit Depth** (8 bit, 16 bit; default 8 bit) to the Rasterize Generic EPS Format dialog with `RasterSizeFields(true)`, and update `tst_import_pdf::epsDialogDefaults` to read the default depth and `epsNativeCmyk16` to set 16 bit; verify both pass

## 5. Scope, roadmap, and parity

- [x] 5.1 Tick the `ROADMAP.md` #627 row (`ROADMAP.md:68-69`) to `[x]`, mark WF-3 `SHIPPED` (`docs/dev/doc-code-parity-gaps.md:259`), and describe only the shipped work plus this change (no 3D); run `python3 scripts/check-doc-claims.py`; commit the docs with `TASK-ALLOWS-DOCS`
- [x] 5.2 Update issue #627's title and body to remove the 3D item and comment that 3D stays covered by the ROADMAP not-planned entry; verify with `gh issue view 627`
- [x] 5.3 Run `openspec validate --all --strict`; verify the change validates and `python3 scripts/check-test-plan.py` maps every scenario

## 6. Final integration

- [x] 6.1 Configure and build the full tree (`cmake -S . -B build -G Ninja && cmake --build build --parallel`), then run `cargo test -p pictura-codec`, `cmake --build build --target tst_import_pdf`, and `ctest --test-dir build -R '^tst_import_pdf$' --output-on-failure`; verify every test passes or skips only because `gs` is absent
- [x] 6.2 Archive the change with `openspec archive pdf-eps-import-remainder`; verify it moved under `openspec/changes/archive/`
- [ ] 6.3 Run `bash scripts/verify-full.sh` and `bash scripts/check-openspec-archived.sh`; verify fmt, clippy, test-report, file-size, guard, and OpenSpec gates pass and no non-archived change remains
