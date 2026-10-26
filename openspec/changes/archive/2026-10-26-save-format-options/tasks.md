# Tasks

## 1. Foundations

- [x] 1.1 Add `save_format_options.{h,cpp}` with the shared `SaveFormatOptions`
      value, its CS6 defaults per format, and its JSON (de)serialization; add
      `pictura-app` unit coverage for the defaults and a JSON round-trip.
- [x] 1.2 Add `save_format.rs` with the `set_save_format_options` /
      `save_format_option` bridge and the quality mapping; verify
      `cargo test -p pictura-app`.
- [x] 1.3 Extend `encode_image.{h,cpp}` with the compression/optimized/
      progressive/matte parameters and `encode_image_bytes`; verify the app
      still builds and `tst_save_options` still passes.
- [x] 1.4 Write the tests named by `test-plan.md` that do not exist yet, before
      their implementation tasks.

## 2. Format dialogs

- [x] 2.1 Add `jpeg_options_dialog.{h,cpp}` with Matte, Image Options (quality
      preset + 0–12 spinbox + slider), Format Options, Scans, Preview, and the
      file-size readout.
- [x] 2.2 Add `png_options_dialog.{h,cpp}` with Compression and the disabled
      Interlace control.
- [x] 2.3 Add `bmp_options_dialog.{h,cpp}` with the CS6 controls, all disabled.
- [x] 2.4 Add `tiff_options_dialog.{h,cpp}` with Image Compression (+ JPEG
      Quality), Pixel Order, Byte Order, Layer Compression, Save Image Pyramid,
      and Save Transparency.
- [x] 2.5 Register the four dialogs and their headers in `CMakeLists.txt`.

## 3. Wire the save path

- [x] 3.1 Add `formatOptionsKindForSuffix` (a testable dispatch helper) and open
      the matching dialog from `saveAsWithDialog`; cancel aborts.
- [x] 3.2 Thread the accepted options to the view through the bridge before
      `saveActiveAs`.
- [x] 3.3 Read `save_raster_options` in `PictureView::save` and pass them to
      `write_raster_with_metadata`.
- [x] 3.4 Apply the JPEG matte and TIFF Save Transparency flatten in the encode
      path.
- [x] 3.5 Add `raster_preview_bytes` and wire the JPEG dialog's size estimate
      and Preview.

## 4. Session persistence

- [x] 4.1 Add `saveFormatOptions` to `SessionState` with load/save.
- [x] 4.2 Hold and update the object in `PicturaMainWindow` and write it in
      `saveSession`.

## 5. Tests and docs

- [x] 5.1 Add `tst_save_format_options` to the tests `CMakeLists.txt` and the
      suite's cases.
- [x] 5.2 Update `docs/dev/doc-code-parity-gaps.md` `WF-1` and `ROADMAP.md`.
- [x] 5.3 Update `docs/dev/STATE.md` with the change summary and ceilings.

## 6. Release checks

- [x] 6.1 `openspec validate --all --strict` and `python3 scripts/check-test-plan.py`.
- [x] 6.2 `bash scripts/verify-fast.sh` (build last, once) with no missing suites.
- [x] 6.3 Archive the change with `openspec archive save-format-options`.
- [x] 6.4 Write the PR body with the manual test-plan rows and both issue links.
