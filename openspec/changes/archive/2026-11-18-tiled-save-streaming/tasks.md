# Tasks

## 1. Tests named by the plan

- [x] 1.1 Golden digests recorded from the previous writers: `psd_encodes_match_golden_digests`, `raster_saves_match_golden_digests`, `whole_image_saves_match_golden_digests`.
- [x] 1.2 `pictura-codec`: `streamed_file_equals_in_memory_encode`, `rle_save_heap_is_bounded`, `zip_save_heap_is_bounded`, `raster_save_heap_is_bounded`, `a_failed_raster_stream_leaves_the_target`, `a_stream_over_the_budget_is_not_cached`, `a_lower_budget_trims_the_cache`; strengthen `parallel_encoding_matches_serial` (multi-band) and adapt `a_channel_error_fails_the_encode`.
- [x] 1.3 `pictura-render`: `save_view_keeps_the_merged_image_tiled`.
- [x] 1.4 `pictura-app`: `write_atomically_with_keeps_the_target_on_error`, `save_cost_does_not_scale_with_the_document`, `a_two_gib_host_admits_a_large_save`, `a_small_host_refuses_the_save`, `a_whole_image_format_is_refused_first`; PNG, TIFF, 8-bit BMP and GIF saves in `estimates_cover_the_measured_peaks`.

## 2. Codec: native writer

- [x] 2.1 `Sink` (position, patch u16/u32/u64 BE/LE) and `write_psd_to`; in-memory entry points over `Cursor<Vec<u8>>`.
- [x] 2.2 Band encoders: RLE (count table patched, rows parallel inside a band), raw, ZIP and ZIP-with-prediction into the sink.
- [x] 2.3 Layer info streamed: records with patched channel lengths, channel data one channel at a time; the tagged 16/32 layout.
- [x] 2.4 Composite block streamed from `PlaneBytes::Rows`; `ponytail:` on the retained-store outputs.
- [x] 2.5 Encode cache: tee within budget, the budget, prune to budget.

## 3. Render

- [x] 3.1 `composite_tiled_channels`; `save_view` without the flat RGB copy; band copy for a stored composite.

## 4. Codec: raster writers

- [x] 4.1 `RasterRows` from a tiled composite; the `Pixels` trait over it and the whole-image `Raster`, which the `ponytail:` formats keep.
- [x] 4.2 PNG (IDAT length and CRC patched, Adam7 passes), BMP 16/24/32, TGA (packet carry), Raw (interleaved and planar), Cineon.
- [x] 4.3 TIFF: streaming PackBits and LZW, ZIP, chunky and planar strips, pyramid pass, streamed layer section, `embed` on the file.
- [x] 4.4 `save_document_*` write through `BufWriter<File>` over the atomic temp file; `ponytail:` on the whole-image formats.

## 5. App

- [x] 5.1 `safe_io::write_atomically_with`; the save worker and recovery stream into it.
- [x] 5.2 `save_gate::save_cost`, `raster_save_cost`, `qt_raster_cost` and `gif_cost`; native save reserves working and admits spillable; `save_engine`, `save_qt_raster`, `write_gif` gated.
- [x] 5.3 `save_engine` builds the tiled composite band by band (sRGB per band) instead of `save_composite`.
- [x] 5.4 Encode cache budget from the governor.

## 6. Measurement

- [x] 6.1 Lower the `save_psd` heap baseline to the measured value; M24 at 17000², free and under `MemoryMax=2G`; record in `docs/dev/STATE.md`.

## 7. Close-out

- [x] 7.1 `openspec validate --all --strict`; archive the change; update STATE.
