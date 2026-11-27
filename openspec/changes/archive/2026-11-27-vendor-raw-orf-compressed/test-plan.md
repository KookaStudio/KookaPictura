# Test plan

This plan covers `vendor-raw-orf-compressed` (#1535):
`codec/vendor-raw-containers` (modified).

**Fixtures.** Synthetic ORFs are written by
`tests/camera_raw/raw/orf_builder.rs`, their compressed streams by
`tests/camera_raw/raw/orf_encode.rs`, from deterministic noise and ramps.
Corpus rows read user-supplied camera files, opt-in through
`PICTURA_RAW_CORPUS` (raw.pixls.us samples, not committed); the LibRaw row
also needs python3 with rawpy (LibRaw 0.22.1) and self-skips without it.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `orf_words` | Unchanged words test | Normalized plane | `pictura-codec::sixteen_bit_orf_words_decode_exactly` |
| `orf_packed` | Unchanged packed test | Normalized plane | `pictura-codec::packed_orfs_decode_exactly` |
| `orf_fields` | Unchanged field test | Rows in sensor order | `pictura-codec::interlaced_compact_orfs_put_their_fields_back` |
| `orf_compressed` | 12- and 14-bit strips of one byte per photosite, no coding tags | `Unsupported`, naming the coding parameters | `pictura-codec::compressed_orfs_without_their_coding_are_refused` |
| `orf_unsupported` | Unchanged refusal test | `Unsupported` | `pictura-codec::orf_variants_are_refused` |
| `orf_maker_notes` | Unchanged layout test | Same plane, neutral, crop | `pictura-codec::every_maker_note_layout_reads_alike` |
| `orf_tags` | Unchanged camera-tag test | Reported; demosaic Ok | `pictura-codec::orf_reports_camera_tags` |
| `orf_hostile` | Unchanged hostile test | Errors, no panic | `pictura-codec::hostile_orfs_fail_cleanly` |
| `orf_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_orf::orf_corpus` | Every file, compressed included, non-flat, greens agree, colour | `pictura-codec::orf_corpus_reads_when_present` |
| `orf_compressed_decode` | 40 × 24 noise and ramp: 12 bits with fine bits 2, 1, 0; 14 bits with SensorCalibration 4000; 14 bits with low bits 2, fine shift 15 | Written plane, normalized, levels × 4 at 14 bits | `pictura-codec::compressed_orfs_decode_exactly` |
| `orf_compressed_notes` | The coding in `OLYMPUS`, `OM SYSTEM` and `OLYMP` value notes | Written plane | `pictura-codec::every_note_with_image_processing_carries_the_coding` |
| `orf_compressed_coding` | No coding; each bounded parameter out of range; 16 bits; header 2 | `Unsupported` ×3, header `Invalid` | `pictura-codec::compressed_orfs_need_their_coding`, `pictura-codec::the_header_is_checked`, `pictura-codec::out_of_range_parameters_are_refused` |
| `orf_compressed_hostile` | Every cut; flips through the stream; a 15-byte strip | No panic; cuts and short strip error | `pictura-codec::hostile_compressed_orfs_fail_cleanly` |
| `orf_compressed_libraw` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_orf_compressed::compressed_orfs_match` with rawpy installed | Every compressed ORF equals LibRaw's plane | `pictura-codec::compressed_orfs_match_libraw_when_present` |
