# Tasks

## 1. Tests first and shared foundations

Authoring only: the single build and all verification run in section 12; do not
run build, list, or test commands while authoring.

- [x] 1.1 Add the codec test functions named in `test-plan.md` (`save_is_deterministic`, `save_atomic_failure`, `save_atomic_set`, `save_atomic_set_rename`, `bmp_depth_headers`, `bmp_rle_rules`, `bmp_flip_rows`, `bmp_os2_header`, `bmp_os2_limits`, `bmp_advanced_16_layouts`, `bmp_32_alpha`, `png_alpha_round_trip`, `png_adam7_round_trip`, `png_compression_levels`, `tiff_compression_tags`, `tiff_jpeg_limits`, `tiff_planar_strips`, `tiff_byte_order_honors_option_for_flat_files`, `layered_tiff_is_big_endian`, `tiff_predictor_tag`, `tiff_alpha_flatten`, `tiff_transparency_off_composites_cmyk_white`, `tiff_transparency_off_composites_lab_white`, `tiff_metadata_tags`, `tiff_mode_photometric`, `tiff_pyramid_subifds`, `tiff_layer_section_round_trip`, `tiff_layer_compression`, `tiff_discard_layers`, `gif89a_structure`, `gif_indexed_plane`, `gif_interlace_flag`, `gif_transparent_index`, `tga_depth_decode`, `tga_rle_decode`, `tga_origin`, `raw_header_bytes`, `raw_interleave_planes`, `raw_mode_channels`, `raw_refuses_multichannel_without_planes`, `raw_eight_bit_size`, `cineon_header_magic`, `cineon_10bit_packing`, `cineon_deterministic_mapping`, `cineon_no_alpha`, `eps_dsc_header`, `eps_preview_options`, `eps_encodings`, `eps_clipping_path`, `eps_vector_data`, `eps_color_spaces`, `dcs_1_plate_files`, `dcs_2_single_file`, `dcs_process_only`, `dcs_preview`, `save_refuses_invalid`, `save_refuses_mode`) as `#[test]` stubs under `crates/pictura-codec/src/save/tests/`.
- [x] 1.2 Add the planned Qt cases in `tst_save_format_options` (`filterHintChoosesDcs`, `filterHintChoosesEps`, `unknownSuffixRefused`, `missingSuffixRefused`, `saveStartRefusesNewFormats`, `featureWarningNewFormats`, `pngInterlaceWrite`, `bmpOptionsWrite`, `tiffPixelByteOrderWrite`, `tiffPyramidWrite`, `tiffLayerCompressionWrite`, `epsOptionsWrite`, `rawOptionsWrite`, `tgaOptionsWrite`, `cineonOptionsWrite`, `gifSaveConverts`, `gifCancelLeavesMode`, `gifRowOrderWrite`, `dcsFiles`, `dcsSpotCeiling`, `plainSaveKind`), the new `tst_save_format_dialogs` cases (`gifControls`, `epsControls`, `dcsControls`, `rawControls`, `tgaControls`, `cineonControls`, `bmpAdvancedControls`, `rawMacDisabled`; the six format-controls cases also assert the documented first-use defaults and remembered settings), and `tst_save_options::exportEdgeScaleAlpha`; register the new suite in `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- [x] 1.3 Extend `SaveFormatKind`, `SaveFormatOptions`, `saveFormatKindForSuffix`, `saveFormatKey`, the JSON round-trip and clamping in `save_format_options.{h,cpp}` for GIF, EPS, DCS, Raw, TGA (including `.vda`, `.icb`, `.vst`) and Cineon and the new BMP/TIFF fields; extend `tst_save_format_options::dialogDefaults`/`rememberedSettings` (or the `tst_save_format_dialogs::*Controls` cases) to the six new dialogs.
- [x] 1.4 Add the typed Rust `SaveOptions`/`SaveKind` and the JSON `set_save_options` bridge in `crates/pictura-app/src/cxxqt_object/save_format/`, including the `SaveRequest` that carries the document/mode planes plus the retained palette and index plane, and replace the flat raster option setter.
- [x] 1.5 Add the engine `save/` dispatch and atomic-write helper in `crates/pictura-codec/src/save/`, wire `PictureView::save` to it, and share one format set across `PictureView::save`, `saveRunsOnWorker`, `save_start_rust` and `document_api::saveStart` so unknown or missing suffixes and the new kinds are refused instead of reaching the PSD worker; verify via `pictura-codec::save_is_deterministic`, `save_atomic_failure`, `tst_save_format_options::unknownSuffixRefused`, `missingSuffixRefused` and `saveStartRefusesNewFormats` in section 12.
- [x] 1.6 Register every new app source and header in its owning fragment: dialogs and option parsing in `cmake/sources/documents.cmake`, save-flow sources beside `import_save_save.cpp` in `cmake/sources/frame.cmake`; run `python3 scripts/check-cmake-source-fragments.py` in section 12, and keep every new file under the size caps (`scripts/file-size-allowlist.txt` only shrinks).
- [x] 1.7 Migrate the existing `tst_save_format_options` cases from `pictura::set_save_raster_options` to the typed setter (or keep a compatibility wrapper), and remove the stale Qt-TIFF-availability skip from the Save As TIFF cases in `tst_save_format_options` and `tst_save_options`.

## 2. BMP

- [x] 2.1 Implement the BMP writer with Windows/OS/2 headers, depths 1/4/8/16/24/32, RLE4/RLE8, Flip Row Order and the three Advanced Modes 16-bit layouts; limit the OS/2 form to 1/4/8/24 bits and refuse OS/2 with Flip, 16-bit Advanced Modes or 32-bit; verify `pictura-codec::bmp_depth_headers`, `bmp_rle_rules`, `bmp_flip_rows`, `bmp_os2_header`, `bmp_os2_limits`, `bmp_advanced_16_layouts` and `bmp_32_alpha`.
- [x] 2.2 Enable the BMP Options controls, add the Advanced Modes sub-dialog, route the accepted values through the options bridge, and rewrite `tst_save_format_options::unsupportedControlsDisabled` so it asserts only the still-disabled controls; verify `tst_save_format_options::bmpOptionsWrite` and `tst_save_format_dialogs::bmpAdvancedControls`.
- [x] 2.3 Add the BMP filter, suffix mapping and remembered settings path; verify `tst_save_format_options::formatKindForSuffix`, `dialogDefaults` and `rememberedSettings`.

## 3. PNG

- [x] 3.1 Implement the PNG writer with 8-bit gray/gray-alpha/RGB/RGBA, adaptive filters, compression level and Adam7 interlace; verify `pictura-codec::png_alpha_round_trip`, `png_adam7_round_trip` and `png_compression_levels`.
- [x] 3.2 Enable PNG Interlace in `png_options_dialog`, route it to the writer, and drop the PNG interlace assertions that `unsupportedControlsDisabled` still expects; verify `tst_save_format_options::pngInterlaceWrite` and `pngCompressionChangesSize`.

## 4. TIFF flat options

- [x] 4.1 Implement the TIFF writer core: byte order for flat output (layered output is always big-endian because the PSD layer payload is big-endian), strips, photometric interpretation per mode, bits per sample, chunky/planar layout, ColorMap tag 320 for Indexed and the metadata tags; verify `pictura-codec::tiff_planar_strips`, `tiff_byte_order_honors_option_for_flat_files`, `layered_tiff_is_big_endian`, `tiff_metadata_tags` and `tiff_mode_photometric`.
- [x] 4.2 Add None/PackBits/LZW/Deflate/JPEG compression, the app-supplied JPEG callback, optional internal prediction for LZW/Deflate, and the documented JPEG gate (opaque 8-bpc RGB/gray, at most 30,000 px); verify `pictura-codec::tiff_compression_tags`, `tiff_predictor_tag` and `tiff_jpeg_limits`.
- [x] 4.3 Implement alpha as an extra sample and Save Transparency flattening over each mode's white (RGB/gray, CMYK, Lab); verify `pictura-codec::tiff_alpha_flatten`, `tiff_transparency_off_composites_cmyk_white` and `tiff_transparency_off_composites_lab_white`.
- [x] 4.4 Enable the TIFF compression, pixel order, byte order and transparency controls, route them through the options bridge, and update `unsupportedControlsDisabled` for the controls that are now honored; verify `tst_save_format_options::tiffCompressionTag`, `tiffPixelByteOrderWrite` and `tiffTransparencyOffFlattens`.

## 5. TIFF pyramid and layered TIFF

- [x] 5.1 Extract the PSD layer-and-mask section builder from `write.rs`/`write_layers.rs` into a reusable engine function without changing PSD output; verify the existing PSD round-trip tests still pass.
- [x] 5.2 Implement Save Image Pyramid with reduced-resolution SubIFDs, keeping IFD0 as the full-size image; verify `pictura-codec::tiff_pyramid_subifds` and `tst_save_format_options::tiffPyramidWrite`.
- [x] 5.3 Embed the layer-and-mask section in tag 37724 with RLE/ZIP layer compression (layered TIFF is big-endian so the tag and PSD payload agree) and implement Discard Layers And Save A Copy; verify `pictura-codec::tiff_layer_section_round_trip`, `tiff_layer_compression`, `tiff_discard_layers` and `tst_save_format_options::tiffLayerCompressionWrite`.
- [x] 5.4 Add the Photoshop layered-TIFF reopen manual row to `test-plan.md` and the PR body, since the app's TIFF reader supplies only the flattened image.

## 6. TGA and Photoshop Raw

- [x] 6.1 Implement the TGA writer for 16/24/32 bits with RLE and a consistent origin; verify `pictura-codec::tga_depth_decode`, `tga_rle_decode` and `tga_origin`.
- [x] 6.2 Implement the Photoshop Raw writer with header padding, interleaved/planar ordering and mode-aware channels for Gray/RGB/CMYK/Lab (Multichannel is refused because the normalized model keeps no plate store); verify `pictura-codec::raw_header_bytes`, `raw_interleave_planes`, `raw_mode_channels`, `raw_refuses_multichannel_without_planes` and `raw_eight_bit_size`.
- [x] 6.3 Add the Targa and Photoshop Raw dialogs with remembered defaults, map `.tga`, `.vda`, `.icb` and `.vst` to Targa, and disable the Linux-only File Type/File Creator controls; verify `tst_save_format_dialogs::tgaControls`, `rawControls`, `rawMacDisabled` and `tst_save_format_options::tgaOptionsWrite`, `rawOptionsWrite`.

## 7. GIF

- [x] 7.1 Implement the GIF save path over the retained Indexed palette and index plane, using the existing GIF encoder and row-order flag; verify `pictura-codec::gif89a_structure`, `gif_indexed_plane`, `gif_interlace_flag` and `gif_transparent_index`.
- [x] 7.2 Add GIF Options, sequence Indexed Color and GIF Options so the mode converts only after both accept, add the GIF filter, and add a test hook (or pure save-flow helper) that drives the Indexed Color dialog; verify `tst_save_format_options::gifSaveConverts`, `gifCancelLeavesMode`, `gifRowOrderWrite` and `tst_save_format_dialogs::gifControls`.

## 8. EPS

- [x] 8.1 Implement the EPS PostScript writer: DSC header, bounding box, RGB/gray/CMYK image invocations, conversion of Lab/Multichannel/Indexed/Duotone/Bitmap sources to RGB/gray/CMYK, ASCII/ASCII85/Binary/JPEG encodings, the chosen JPEG quality and the app-supplied JPEG callback; verify `pictura-codec::eps_dsc_header`, `eps_encodings` and `eps_color_spaces`.
- [x] 8.2 Add previews (None, TIFF 1-bit, TIFF 8-bit) through the DOS binary wrapper and the TIFF writer; verify `pictura-codec::eps_preview_options` and that the app's EPS import opens the result when Ghostscript is available.
- [x] 8.3 Add clipping path, vector data, halftone/transfer, transparent whites and interpolation emission; verify `pictura-codec::eps_clipping_path`, `eps_vector_data` and `tst_save_format_options::epsOptionsWrite`.
- [x] 8.4 Add the Photoshop EPS Options dialog with remembered defaults and the EPS filter; verify `tst_save_format_dialogs::epsControls` and `pictura-codec::eps_color_spaces` coverage.

## 9. DCS

- [x] 9.1 Implement DCS 1.0's master plus `.C/.M/.Y/.K` plate set and DCS 2.0's single separations file on the EPS writer, with set-level atomic writes including rename-phase rollback, and honor the DCS Preview choices; verify `pictura-codec::dcs_1_plate_files`, `dcs_2_single_file`, `pictura-codec::save_atomic_set_rename`, `pictura-codec::dcs_preview` and the Qt `dcsFiles` case.
- [x] 9.2 Add the CMYK conversion warning and the extra-channel omission warning, and add the DCS Format dialog and filter hint; verify `pictura-codec::dcs_process_only`, `tst_save_format_options::dcsSpotCeiling` and `tst_save_format_dialogs::dcsControls`.
- [x] 9.3 Add the DCS page-layout interop manual row to `test-plan.md` and the PR body.
- [x] 9.4 Thread the chosen Save As `SaveFormatKind` through `saveAsWithDialog`, `saveActiveAsWithOptions` and `promptSaveFormatOptions` into the Rust setter, so the DCS/EPS filter hint reaches the dialog; verify `tst_save_format_options::filterHintChoosesDcs` and `filterHintChoosesEps`.

## 10. Cineon

- [x] 10.1 Implement the Cineon writer: Kodak header, 10-bit RGB packing, deterministic 8-bit to 10-bit mapping and no alpha; verify `pictura-codec::cineon_header_magic`, `cineon_10bit_packing`, `cineon_deterministic_mapping` and `cineon_no_alpha`.
- [x] 10.2 Add the Cineon Options dialog with the fixed 10-bit RGB readout and the Cineon filter; verify `tst_save_format_dialogs::cineonControls` and `tst_save_format_options::cineonOptionsWrite`.
- [x] 10.3 Add the Cineon film-tool interop manual row to `test-plan.md` and the PR body.

## 11. Dispatch, filters, warnings and docs

- [x] 11.1 Insert the six filters before the All Formats entry in `saveFileFilters` (All Formats stays last; update its pattern), update `defaultSuffixForFilter`, `isKnownImageSuffix` (including `.vda`, `.icb`, `.vst`), `defaultSaveFilterIndex` and `saveRunsOnWorker`, and keep the format set shared with `save_start_rust`/`document_api::saveStart`; verify `tst_open_save` filter/suffix cases and `tst_save_format_options::formatKindForSuffix`.
- [x] 11.2 Implement plain-Save kind resolution (EPS vs DCS) and remembered-option application; verify `tst_save_format_options::plainSaveKind`.
- [x] 11.3 Extend `saveAsNeedsFeatureWarning`/`isRasterImageSuffix` to every new Save As format, add a non-modal warning driver for tests, and show the existing feature-loss warning for mode conversion, layer flattening, DCS extra channels and unsupported DCS spots; verify `tst_save_format_options::dcsSpotCeiling`, `featureWarningNewFormats` and the GIF-layer warning path.
- [ ] 11.4 Update `docs/dev/doc-code-parity-gaps.md` (`WF-1`, `WF-4`), `ROADMAP.md` (#625 and #628), ticking the GIF half of #628 and leaving Save a Copy open, and `docs/dev/STATE.md`; run `TASK_ALLOWS_DOCS=1 bash scripts/guard.sh` and commit docs with the `TASK-ALLOWS-DOCS` marker.

## 12. Integration and release checks

- [x] 12.1 Run the single full build and offscreen Qt suites: `python3 scripts/cpu-budget.py exec -- cmake --build build --parallel`, then `QT_QPA_PLATFORM=offscreen python3 scripts/cpu-budget.py exec -- ctest --test-dir build -R '^tst_(save_format_options|save_format_dialogs|save_options)$' --output-on-failure`.
- [x] 12.2 Run the engine gates: `python3 scripts/cpu-budget.py exec -- cargo fmt --all --check`, `python3 scripts/cpu-budget.py exec -- cargo clippy --workspace --all-targets -- -D warnings`, `python3 scripts/cpu-budget.py exec -- cargo nextest run --workspace` (oracle tests skip without `magick`/`gs`).
- [ ] 12.3 Run `bash scripts/verify-fast.sh`, `openspec validate --all --strict`, `python3 scripts/check-test-plan.py`, `python3 scripts/check-cmake-source-fragments.py` and `bash scripts/check-file-size.sh`.
- [ ] 12.4 Write the PR body with the manual rows (CS6 dialog comparison, Photoshop layered-TIFF reopen, DCS page-layout interop, Cineon film-tool interop), and archive the change with `openspec archive save-as-all-formats` in the same PR.