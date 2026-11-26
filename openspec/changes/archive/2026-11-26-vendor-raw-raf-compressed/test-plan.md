# Test plan

This plan covers `vendor-raw-raf-compressed` (#1620):
`codec/vendor-raw-containers` (modified).

**Fixtures.** Synthetic RAFs are written by
`tests/camera_raw/raw/raf_builder.rs`, their compressed data by
`tests/camera_raw/raw/raf_encode.rs`, from deterministic noise planes. Corpus
rows read user-supplied camera files, opt-in through `PICTURA_RAW_CORPUS`
(raw.pixls.us samples, not committed); the LibRaw row also needs python3 with
rawpy (LibRaw 0.22.1) and self-skips without it.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `raf_words` | Unchanged words test | Normalized plane | `pictura-codec::raf_words_decode_exactly` |
| `raf_packed` | Unchanged packed test | Normalized plane | `pictura-codec::packed_rafs_decode_exactly` |
| `raf_compressed` | Data of one byte per photosite without the header | `Unsupported`, naming the compressed-data header | `pictura-codec::headerless_short_rafs_are_refused` |
| `raf_unsupported` | Unchanged refusal test | `Unsupported` | `pictura-codec::raf_variants_are_refused` |
| `raf_xtrans` | Unchanged X-Trans levels test | CFA, black, crop, neutral | `pictura-codec::xtrans_raf_takes_its_pattern_and_levels` |
| `raf_exposure_bias` | Unchanged bias cases | Baseline 0.72, 1.72, 0, 0 | `pictura-codec::xtrans_raf_takes_its_pattern_and_levels` |
| `raf_bayer` | Unchanged Bayer test | RGGB; demosaic Ok | `pictura-codec::bayer_raf_is_rggb` |
| `raf_tags` | Unchanged camera-tag test | Reported; fallbacks | `pictura-codec::raf_reports_camera_tags` |
| `raf_hostile` | Unchanged hostile test | Errors, no panic | `pictura-codec::hostile_rafs_fail_cleanly` |
| `raf_dng_pair` | Unchanged X100S RAF/DNG pair (corpus) | Same CFA, samples, neutral | `pictura-codec::raf_matches_its_dng_when_present` |
| `raf_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_raf::raf_corpus` | SuperCCD and EXR refused; others, compressed included, non-flat and demosaic | `pictura-codec::raf_corpus_reads_when_present` |
| `raf_compressed_lossless` | 1512 × 18 noise, lossless, X-Trans at 12 and 14 bits, RGGB at 14 and 16 | Coder reproduces the plane; read equals it normalized | `pictura-codec::lossless_compressed_rafs_decode_exactly` |
| `raf_compressed_lossy` | 1512 × 36 noise, lossy, bases 0–9 by strip and line, X-Trans 14, RGGB 14 and 16 | Read equals the coder's reconstruction; error 1 to 9 | `pictura-codec::lossy_compressed_rafs_decode_as_coded` |
| `raf_compressed_header` | Each header field set out of range; X-Trans header in a Bayer file | `Invalid`, naming compressed data | `pictura-codec::compressed_headers_are_checked` |
| `raf_compressed_hostile` | Cuts, flipped data bytes, strip size `u32::MAX`, lossless and lossy | No panic; cuts and the oversized strip error | `pictura-codec::hostile_compressed_rafs_fail_cleanly` |
| `raf_compressed_libraw` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_raf_compressed::compressed_rafs_match` with rawpy installed | Every compressed RAF equals LibRaw's plane | `pictura-codec::compressed_rafs_match_libraw_when_present` |
