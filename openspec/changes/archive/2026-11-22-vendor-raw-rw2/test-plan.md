# Test plan

This plan covers `vendor-raw-rw2` (#1472, plain packings):
`codec/vendor-raw-containers` and `document/raw-open` (both modified).

**Fixtures.** Synthetic RW2s are written in the test by
`tests/raw/rw2_builder.rs`; planes are deterministic. The corpus row reads
user-supplied camera files, opt-in through `PICTURA_RAW_CORPUS`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `raw_sniff` | Unchanged dispatch test | DNG, CR2, NEF named and read; others none | `pictura-codec::raw_containers_dispatch_by_signature` |
| `arw_sniff` | Unchanged ARW sniff test | ARW, none, DNG | `pictura-codec::arw_is_named_by_its_make` |
| `orf_sniff` | Unchanged ORF sniff test | ORF, ORF, none | `pictura-codec::orf_is_named_by_its_header` |
| `rw2_sniff` | Sniff Panasonic and Leica files, 8-byte prefixes, a Panasonic-make TIFF | RW2, RW2, none | `pictura-codec::rw2_is_named_by_its_header` |
| `rw2_words` | 12- and 14-bit planes as left-aligned words | Equals the normalized words | `pictura-codec::rw2_words_decode_exactly` |
| `rw2_packed` | RawFormat 2 and 7, pad bits set, both offset tags | Equals the normalized samples | `pictura-codec::packed_rw2s_decode_exactly` |
| `rw2_refused` | RawFormat 3, 4, 5, 6, 8, with and without compression | `Unsupported`, naming the RawFormat | `pictura-codec::other_rw2_codings_are_refused_by_name` |
| `rw2_unsupported` | Wrong depth, compression, width, CFA code, length | `Unsupported` | `pictura-codec::rw2_variants_are_refused` |
| `rw2_borders` | Four CFA codes under four border parities | Cropped plane, per-colour black, shifted pattern | `pictura-codec::rw2_borders_shift_the_colour_pattern` |
| `rw2_crop` | Crop inside, outside, absent | Relative crop; whole area; whole area | `pictura-codec::rw2_crop_counts_from_the_sensor` |
| `rw2_tags` | Orientation 6, WB 512/256/384; zero red level | Reported; neutral, none; demosaic Ok | `pictura-codec::rw2_reports_camera_tags` |
| `rw2_no_colour` | `decode_raw_source` on a DMC-FZ50 file | `Unsupported`, naming the camera | `pictura-codec::rw2_without_colour_names_the_camera` |
| `rw2_hostile` | Every prefix; corrupt directory; bad borders; huge width | Errors, no panic | `pictura-codec::hostile_rw2s_fail_cleanly` |
| `rw2_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test raw_rw2 corpus` | Plain files read, greens agree; others refused | `pictura-codec::rw2_corpus_reads_when_present` |
| `raw_open_sniff` | DNG, Sony, other make, IIRO and IIU as `.tif`, plain TIFF, missing path | DNG, Sony, IIRO and IIU detected | `pictura-app::raw_sniff_detects_by_signature` |
| `raw_open_filter` | Read `openFileFilters()` (unchanged) | Six raw extensions in both filters | `tst_raw_open::openFiltersListCameraRaw` |
