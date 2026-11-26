# Test plan

This plan covers `dng-output` (#402): the new capabilities `codec/dng-writer`
and `document/raw-save-image`, and the requirements added to
`codec/dng-raw-reader` and `codec/raw-develop`.

**Fixtures.** Writer tests build `DngImage` / `CameraRgb` inputs in code and
inspect the output with the codec's own TIFF parser (unit tests in
`dng/write/tests.rs`). Linear-reader tests use the synthetic DNG builder and
lossless-JPEG encoder under `tests/camera_raw/dng/`. App Rust tests reuse the minimal
uncompressed DNG writer in `raw_open/tests.rs` and decode a `PendingRaw` with
an explicit temporary database. The Qt suite `tst_raw_open` already points
`XDG_DATA_HOME` at a temporary folder. The reference-tool test skips when
`python3 -c "import rawpy"` or `exiftool` is unavailable.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `dng_write_mosaic_round_trip` | Write a 40 × 30 GRBG noise mosaic with crop, orientation 6, make/model and every colour tag; `read_dng` it | Every field equal | `pictura-codec::mosaic_round_trips` |
| `dng_write_mosaic_tiles` | Write a 600 × 300 noise mosaic | Compression 7, 256 × 256 tiles, six tiles; samples equal | `pictura-codec::mosaic_spans_tiles` |
| `dng_write_linear_round_trip` | Write a 37 × 21 noise camera RGB as linear; `read_linear_dng` it | Camera RGB and tags equal | `pictura-codec::linear_round_trips` |
| `dng_write_thumbnail` | Write with 400 × 300 preview pixels, no JPEG preview | IFD0 subfile 1, photometric 2, 3 × 8-bit, 256 × 192; one SubIFD | `pictura-codec::ifd0_is_thumbnail` |
| `dng_write_jpeg_preview` | Write with a 120 × 80 JPEG preview | Second SubIFD subfile 1, compression 7, photometric 6, PreviewColorSpace 2; strip `FF D8`, decodes 120 × 80 | `pictura-codec::jpeg_preview_ifd` |
| `dng_write_xmp` | Write with a `crs:` packet | `raw_xmp` returns it unchanged | `pictura-codec::xmp_packet_round_trips` |
| `dng_write_original_raw` | Write with a 200 000-byte original `shot.cr2` | Name tag; four blocks; inflated bytes equal | `pictura-codec::original_raw_inflates` |
| `dng_write_compatibility` | Write at Camera Raw 2.4 and 6.6 with forward matrix and preview | 1.1.0.0 without ForwardMatrix1 / PreviewColorSpace; 1.4.0.0 with both | `pictura-codec::compatibility_sets_versions` |
| `dng_write_refuses` | 0 × 0 image; mosaic short one sample; preview short one byte | Each `Invalid` | `pictura-codec::inconsistent_input_is_refused` |
| `dng_write_cancel` | Write with cancellation already true | `None` | `pictura-codec::cancelled_write_returns_none` |
| `dng_write_reference_tools` | Write 64 × 48 mosaic (with original) and linear DNGs; run rawpy and exiftool | LibRaw 64 × 48 both; DNGVersion 1.4.0.0; extracted original equal | `pictura-codec::reference_tools_read_written_dngs` |
| `dng_linear_read` | Read a 12 × 8 uncompressed 16-bit linear DNG with per-sample black levels, white 4095, active area | 10 × 6, samples normalized per sample | `pictura-codec::linear_dng_normalizes_per_sample` |
| `dng_linear_read_ljpeg` | Read a 20 × 10 linear DNG of 16 × 16 three-component LJPEG tiles | Samples equal | `pictura-codec::linear_dng_reads_ljpeg_tiles` |
| `dng_linear_refuses` | Four-sample linear; linear with BlackLevelDeltaV; CFA via `read_linear_dng`; linear via `read_dng` | Each `Unsupported` | `pictura-codec::linear_dng_refuses_unsupported_forms` |
| `raw_source_linear` | `decode_raw_source` of a 16 × 12 linear DNG | Camera RGB = file samples; crop and as-shot equal | `pictura-codec::linear_dng_decodes_without_demosaic` |
| `raw_save_button` | Open a DNG interactively; show Filter ▸ Pictura Raw on a document | Raw open dialog has `rawSaveImageButton`; filter dialog has none | `tst_raw_open::saveImageOnlyInRawOpen` |
| `raw_save_default_path` | Default path for `shot.dng` with `shot.dng`, `shot-1.dng` present; `frame.cr2` in empty folder | `shot-2.dng`; `frame.dng` | `pictura-app::default_dng_path_skips_taken_names` |
| `raw_save_options_defaults` | Click Save Image; inspect Save Options | Source folder, default name, 6.6, Medium, boxes off | `tst_raw_open::saveOptionsDefaults` |
| `raw_save_writes_dng` | Save Image → accept options (new name, linear) → Cancel the raw dialog | File exists `II*\0`; status names it; no document; database empty | `tst_raw_open::saveImageWritesDng` |
| `raw_save_dng_contents` | Save pending raw as mosaic, exposure +0.50, Medium preview, original embedded | `read_dng` = `read_raw` source; XMP exposure 0.5; preview IFD; original equal | `pictura-app::saved_mosaic_dng_holds_settings_and_original` |
| `raw_save_dng_linear` | Save as linear, no preview | `read_linear_dng` = demosaiced source; no preview IFD | `pictura-app::saved_linear_dng_holds_camera_rgb` |
| `raw_save_refuses` | Save onto source path; save after rewriting the file | Both fail (source named; "changed"); raw bytes unchanged | `pictura-app::save_refuses_source_and_changed_file` |
| `raw_save_confirms_replace` | Name an existing file; decline replace | File unchanged; status empty | `tst_raw_open::declinedReplaceWritesNothing` |
| `raw_save_cancel` | Cancel before write | Cancelled; no target, no temp file | `pictura-app::cancelled_save_leaves_no_file` |

**Manual (reviewer checklist).** Not spec rows; real-camera checks that need
files we cannot commit:

- Open `~/Downloads/RAW_CANON_450D.CR2`, Save Image as a mosaic DNG with a
  Full Size preview and the original embedded; open the DNG in Pictura and in
  darktable or RawTherapee and compare the look. (manual: needs a real
  camera file and a desktop raw tool.)
- Save a linear DNG from `~/Downloads/L1.dng` and open it in Adobe DNG
  Converter / ACR where available. (manual: Adobe tools are not on Linux CI.)
