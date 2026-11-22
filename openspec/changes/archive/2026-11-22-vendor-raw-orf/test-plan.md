# Test plan

This plan covers `vendor-raw-orf` (#1471): `codec/vendor-raw-containers` and
`document/raw-open` (both modified).

**Fixtures.** Synthetic ORFs are written in the test by
`tests/raw/orf_builder.rs`; planes are deterministic. Corpus rows read
user-supplied camera files, opt-in through `PICTURA_RAW_CORPUS` (checked on 112 raw.pixls.us files:
22 uncompressed read and rendered, 90 compressed refused from their headers).

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `raw_sniff` | Unchanged dispatch test | DNG, CR2, NEF named and read; others none | `pictura-codec::raw_containers_dispatch_by_signature` |
| `arw_sniff` | Unchanged ARW sniff test | ARW, none, DNG | `pictura-codec::arw_is_named_by_its_make` |
| `orf_sniff` | Sniff three headers, their 8-byte prefixes, an Olympus-make TIFF | ORF, ORF, none | `pictura-codec::orf_is_named_by_its_header` |
| `orf_words` | 12-bit LE and 10-bit BE planes left-aligned over strips | Equals the normalized plane | `pictura-codec::sixteen_bit_orf_words_decode_exactly` |
| `orf_packed` | One plane in three packings | Equals the normalized plane | `pictura-codec::packed_orfs_decode_exactly` |
| `orf_fields` | IIRS at 23 and 24 rows, several strip sizes | Sensor row order | `pictura-codec::interlaced_compact_orfs_put_their_fields_back` |
| `orf_compressed` | 12- and 14-bit files with one short strip | `Unsupported`, naming compressed data | `pictura-codec::compressed_orfs_are_refused` |
| `orf_unsupported` | 1.75 B/px; several short strips; packed at 14 bits; non-Bayer CFA | `Unsupported` | `pictura-codec::orf_variants_are_refused` |
| `orf_maker_notes` | Same values in five note layouts | Same plane, neutral, crop | `pictura-codec::every_maker_note_layout_reads_alike` |
| `orf_tags` | OM-5, orientation 6, GRBG | Reported; demosaic Ok | `pictura-codec::orf_reports_camera_tags` |
| `orf_hostile` | Every prefix; corrupt directories; big crop; width 40000 | Errors, no panic | `pictura-codec::hostile_orfs_fail_cleanly` |
| `orf_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test raw_orf corpus` | Compressed refused; others read non-flat, greens within 5 %, colour | `pictura-codec::orf_corpus_reads_when_present` |
| `orf_colour` | Convert a neutral and a colour at as-shot | Neutral; matrix colour within 0.002 | `pictura-codec::orf_matrix_renders_as_the_camera_meant` |
| `orf_anchor` | 6600 K, fine weather, as-shot only, none | 6504, 5503, 6504 K; refused | `pictura-codec::orf_daylight_balance_sets_the_temperature` |
| `raw_open_sniff` | DNG, Sony, other make, IIRO as `.tif`, plain TIFF, missing path | DNG, Sony and IIRO detected | `pictura-app::raw_sniff_detects_by_signature` |
| `raw_open_filter` | Read `openFileFilters()` | Six raw extensions in both filters | `tst_raw_open::openFiltersListCameraRaw` |
| Colour against camera JPEGs | Open an E-300, an XZ-2 and an IIRS compact ORF; compare with the embedded preview | Plausible colour, no pink highlights; known: warm tones lean pink on SP-350, SP-510UZ, SP-550UZ | manual: judging colour against a styled camera JPEG has no oracle |
