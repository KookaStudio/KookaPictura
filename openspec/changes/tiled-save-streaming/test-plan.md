# Test plan

This plan covers `tiled-save-streaming` (#1320): the new capabilities
`codec/streamed-psd-save`, `codec/streamed-raster-save` and
`document/save-memory-gate`, and the changes to `codec/psd-encode-cache`.

## Fixtures

- **Golden matrix.** Seeded documents built in the test: a 600 × 1100 RGB
  document with three noise and flat layers, a mask and an extra alpha
  channel, which spans several bands and a partial tile column; the same
  composite with and without transparency; a grayscale document; and the
  Indexed and CMYK fixtures under `crates/pictura-codec/tests/fixtures/`. The
  digests were recorded from the previous writers at the base commit, before
  any writer changed, and are pinned in the tests.
- **Peak tests.** These live in their own integration binary with
  `pictura_testkit::PeakAlloc` as its global allocator, and one mutex
  serialises the measurements. The document is 4096 × 4096, so it is
  10–40 times its own allowance.
- **App gate tests.** These pin the host through `memory_gate`'s
  `set_cgroup_for_test` and the scratch-disk override.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `sps_identical` | Write each corpus document into a temp file through `write_psd_to`, and encode it with `write_psd`/`write_psb` | Same bytes for every document | `pictura-codec::streamed_file_equals_in_memory_encode` |
| `sps_golden` | Encode the golden matrix (PSD and PSB × RLE, raw, ZIP, ZIP-with-prediction × layers and composite; a 16-bit source) | SHA-256 of each equals the pinned digest | `pictura-codec::psd_encodes_match_golden_digests` |
| `sps_bounded` | Write the 4096² three-noise-layer RGB document RLE into a file, with no cache | Heap peak < 64 MiB; file > 100 MiB | `pictura-codec::rle_save_heap_is_bounded` |
| `sps_zip_bounded` | Same, with ZIP and with ZIP-with-prediction | Heap peak < 64 MiB for each | `pictura-codec::zip_save_heap_is_bounded` |
| `sps_merged_tiled` | Build `save_view` for a layered RGB document with an extra channel, and for a layerless document with an RGBA stored composite | 3-channel tiled composite; RGB equals the full composite; 0 materializations | `pictura-render::save_view_keeps_the_merged_image_tiled` |
| `sps_error_no_file` | Save over an existing file with a writer closure that errs after writing some bytes; also a raster save whose guard fails after the stream | Target bytes unchanged; no temp file left | `pictura-app::write_atomically_with_keeps_the_target_on_error`, `pictura-codec::a_failed_raster_stream_leaves_the_target` |
| `pec_parallel_identical` | Encode an 8-layer, 600-row document on the default pool and on a 1-thread pool | Byte-identical | `pictura-codec::parallel_encoding_matches_serial` |
| `pec_parallel_error` | Encode a 70 000-byte RLE row into a PSD | Error, nothing returned | `pictura-codec::a_channel_error_fails_the_encode` |
| `pec_budget` | Encode through a cache whose budget is below one layer stream; encode again | Resident ≤ budget; that layer misses again; both equal an uncached encode | `pictura-codec::a_stream_over_the_budget_is_not_cached` |
| `pec_budget_trim` | Encode through a cache, lower its budget below resident, encode again | Resident ≤ the lower budget | `pictura-codec::a_lower_budget_trims_the_cache` |
| `srs_golden` | Save the golden matrix with each streaming format and option set into temp files | SHA-256 of each equals the pinned digest | `pictura-codec::raster_saves_match_golden_digests` |
| `srs_bounded` | Save a 4096² RGBA tiled composite as PNG, layered LZW TIFF and 24-bit BMP | Writer heap peak < 64 MiB beyond the composite | `pictura-codec::raster_save_heap_is_bounded` |
| `srs_whole_golden` | Save the golden matrix as GIF, EPS, DCS, 8-bit BMP, and CMYK/Lab TIFF and Raw | SHA-256 of each equals the pinned digest | `pictura-codec::whole_image_saves_match_golden_digests` |
| `smg_estimate` | `save_cost` of a 17000² document with four dense-tile layers and a Background | Working < 512 MiB; the old formula > 10 GiB | `pictura-app::save_cost_does_not_scale_with_the_document` |
| `smg_admitted` | Pin a 2 GiB cgroup (1.5 GiB used) and 4 GiB scratch; gate that save | Admitted | `pictura-app::a_two_gib_host_admits_a_large_save` |
| `smg_refused` | Pin a cgroup whose room is below the working bytes; start the save | Refused; memory refusal recorded; no file | `pictura-app::a_small_host_refuses_the_save` |
| `smg_whole_refused` | Pin room between the TIFF cost and the GIF cost; gate both saves | TIFF admitted; GIF refused as memory | `pictura-app::a_whole_image_format_is_refused_first` |
| `smg_covers_peak` | Heap-peak harness: PSD, PNG, TIFF and GIF saves at 4250² | Each peak ≤ its estimate | `pictura-app::estimates_cover_the_measured_peaks` |
| `smg_m24` | `python3 scripts/profile-tools.py --scenarios M24 --size 17000x17000 --digest`, free and with `--memory-max 2G` | Every step ok under 2G, save included; digests `same` | manual: needs multi-GB generated fixtures, a systemd cgroup memory limit and minutes of a real app; run for this change and recorded in `docs/dev/STATE.md` |
