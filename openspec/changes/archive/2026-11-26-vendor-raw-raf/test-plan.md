# Test plan

This plan covers `vendor-raw-raf` (#1473): `codec/vendor-raw-containers`,
`codec/raw-demosaic`, `codec/camera-matrices`, `codec/pictura-raw` and
`document/raw-open` (all modified).

**Fixtures.** Synthetic RAFs are written in the test by
`tests/camera_raw/raw/raf_builder.rs`, with in-test encoders for the 16-bit,
12-bit and 14-bit codings; planes are deterministic. Corpus rows read
user-supplied camera files, opt-in through `PICTURA_RAW_CORPUS` (raw.pixls.us
samples of 14 bodies plus the owner's FinePix S7000 and XF1, none committed).

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `raw_sniff` | Unchanged dispatch test | DNG, CR2, NEF named and read; others none | `pictura-codec::raw_containers_dispatch_by_signature` |
| `arw_sniff` | Unchanged ARW sniff test | ARW, none, DNG | `pictura-codec::arw_is_named_by_its_make` |
| `orf_sniff` | Unchanged ORF sniff test | ORF; TIFF none | `pictura-codec::orf_is_named_by_its_header` |
| `rw2_sniff` | Unchanged RW2 sniff test | RW2; TIFF none | `pictura-codec::rw2_is_named_by_its_header` |
| `cr3_sniff` | Unchanged CR3 sniff test | CR3; `isom` none | `pictura-codec::cr3_is_named_by_its_brand` |
| `raf_sniff` | Sniff a RAF, its 16-byte prefix, a `FUJIFILMCCD-RAX ` file | RAF, RAF, none; RAF reads | `pictura-codec::raf_is_named_by_its_header` |
| `raf_words` | 14-bit plane as LE words in an `II` section and BE words in an `MM` section | Equals the normalized plane | `pictura-codec::raf_words_decode_exactly` |
| `raf_packed` | 12-bit LSB-first and 14-bit MSB-over-words, padded rows, widths 40 and 41 | Equals the normalized plane | `pictura-codec::packed_rafs_decode_exactly` |
| `raf_compressed` | Data of one byte per photosite | `Unsupported`, naming compressed data | `pictura-codec::compressed_rafs_are_refused` |
| `raf_unsupported` | Second frame; non-TIFF sensor section; EXR layout; 10-bit packed; packed rows not totalling the data | `Unsupported` | `pictura-codec::raf_variants_are_refused` |
| `raf_xtrans` | X-Trans layout, pattern record, 6 × 6 black, crop, levels 302/577/454 | CFA reversed; per-position black; crop; neutral | `pictura-codec::xtrans_raf_takes_its_pattern_and_levels` |
| `raf_exposure_bias` | Bias records -72/100, -172/100, 5/0, none | Baseline 0.72, 1.72, 0, 0 | `pictura-codec::xtrans_raf_takes_its_pattern_and_levels` |
| `raf_bayer` | Bayer layout, 2 × 2 black | RGGB; demosaic Ok | `pictura-codec::bayer_raf_is_rggb` |
| `raf_tags` | JPEG Exif FUJIFILM / X-T3 / 6; no JPEG with header name X100S | Reported; fallback FUJIFILM / X100S / 1 | `pictura-codec::raf_reports_camera_tags` |
| `raf_hostile` | Every prefix of each coding; corrupt offsets and record sizes; big crop; width 40000 | Errors, no panic | `pictura-codec::hostile_rafs_fail_cleanly` |
| `raf_dng_pair` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_raf::dng_pair` with the X100S RAF and DNG | Same CFA, samples and neutral over the DNG crop | `pictura-codec::raf_matches_its_dng_when_present` |
| `raf_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_raf::corpus` | Compressed, SuperCCD, EXR refused; others non-flat and demosaic | `pictura-codec::raf_corpus_reads_when_present` |
| `demosaic_edge` | Unchanged Bayer edge test | AHD under a quarter of bilinear | `pictura-codec::ahd_reduces_false_colour_at_edges` |
| `demosaic_fallback` | AHD on 3×3 RGB, 1×1, 3×2, 7×7 Bayer | Equal to bilinear | `pictura-codec::ahd_falls_back_to_bilinear` |
| `xtrans_gradient` | Horizontal and vertical grey ramps through X-Trans, AHD | Monotonic; equal within 1 | `pictura-codec::xtrans_ramp_is_reproduced` |
| `xtrans_edge` | Grey vertical and horizontal edges through X-Trans, both methods | AHD under a quarter of bilinear | `pictura-codec::xtrans_reduces_false_colour_at_edges` |
| `xtrans_applies` | AHD on a 6×6 pattern whose first four rows are green | Equal to bilinear | `pictura-codec::non_xtrans_six_by_six_keeps_bilinear` |
| `raf_jpeg_matrices` | Synthetic RAFs naming each fitted body and X-T99 | Matrices under 21 (and 17 when paired); X-T3 ≠ X-T2; X-T99 none | `pictura-codec::fujifilm_bodies_take_their_matrices` |
| `raf_jpeg_matrices_white` | Each matrix's response to the white at its light as the as-shot neutral | Neutral positive; that light's temperature within 2 % | `pictura-codec::fujifilm_matrices_keep_white_neutral` |
| `raf_known_camera_decodes` | Synthetic X-T3 RAF, levels 302/583/584, through `decode_raw_source` | Decodes; as-shot equals `as_shot_transform`, 2000–12000 K | `pictura-codec::known_fujifilm_raf_decodes` |
| `raf_colour_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_raf_colour::corpus` | Table bodies decode at 2000–12000 K | `pictura-codec::raf_corpus_colour_when_present` |
| `raf_jpeg_fits_generated` | Build `raw_jpeg_blocks`, run `fit-camera-jpeg-matrices.py` on the RAF samples, then `gen-camera-matrices.py <rawtoaces-data> <D65 csv>`; `git diff` | No diff | manual: needs the sample raw files and the external datasets, which are not committed |
| `raf_render_matches_camera` | Open each fitted body's sample through File ▸ Open at its as-shot white balance; compare beside its embedded JPEG | Colours follow the camera's rendering (tone and lens corrections aside) | manual: a visual judgement on files that are not committed |
| `raw_open_sniff` | Sniff test with `FUJIFILMCCD-RAW ` saved as `fuji.dat` added | Fujifilm file detected | `pictura-app::raw_sniff_detects_by_signature` |
| `raw_open_filter` | Read `openFileFilters()` | Eleven raw extensions in both filters | `tst_raw_open::openFiltersListCameraRaw` |
| `rb_photo_raw` | Unchanged DNG photo-information test | `shot.dng`, 16 by 12, 16 bits | `pictura_app::raw_photo_info_reads_camera_exposure` |
| `rb_photo_document` | Unchanged filter-dialog information test | 16 by 16, 8 bits, sRGB, 72 ppi | manual: unchanged by this change, covered by the raw-dialog-inspector plan |
| `rb_photo_missing` | Unchanged missing-tags test | Exposure labels empty | `pictura_app::raw_photo_info_omits_missing_tags` |
| `rb_photo_raf` | Photo information for a RAF whose camera JPEG's Exif names f/8, ISO 100, 240 ppi | Reported | `pictura_app::raw_photo_info_reads_a_raf_cameras_jpeg_exif` |
| `rb_photo_size` | 40 × 30 raw, 32 × 24 crop, orientation 1 then 6 | 32 by 24, then 24 by 32 | `pictura_app::photo_info_size_is_the_developed_size` |
