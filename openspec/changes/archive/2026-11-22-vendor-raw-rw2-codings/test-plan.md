# Test plan

This plan covers `vendor-raw-rw2-codings` (#1541): `codec/vendor-raw-containers`
(modified).

**Fixtures.** Synthetic RW2s are written in the test by
`tests/raw/rw2_builder.rs`, the compressed codings by the encoders in
`tests/raw/rw2_encode.rs`; planes are seeded walks the codings represent
exactly. The corpus row reads user-supplied camera files, opt-in through
`PICTURA_RAW_CORPUS`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `rw2_words` | 12- and 14-bit planes as left-aligned words | Equals the normalized words | `pictura-codec::rw2_words_decode_exactly` |
| `rw2_packed` | RawFormat 2 and 7, pad bits set, both offset tags | Equals the normalized samples | `pictura-codec::packed_rw2s_decode_exactly` |
| `rw2_refused` | RawFormat 1, 9; 5 at 10 bits; 6 at 16; 8 at 17; with and without compression | `Unsupported`, naming the RawFormat | `pictura-codec::unknown_rw2_codings_are_refused_by_name` |
| `rw2_unsupported` | Wrong depth, compression, width, CFA code, length | `Unsupported` | `pictura-codec::rw2_variants_are_refused` |
| `rw2_borders` | Four CFA codes under four border parities | Cropped plane, per-colour black, shifted pattern | `pictura-codec::rw2_borders_shift_the_colour_pattern` |
| `rw2_crop` | Crop inside, outside, absent | Relative crop; whole area; whole area | `pictura-codec::rw2_crop_counts_from_the_sensor` |
| `rw2_tags` | Orientation 6, WB 512/256/384; zero red level | Reported; neutral, none; demosaic Ok | `pictura-codec::rw2_reports_camera_tags` |
| `rw2_no_colour` | `decode_raw_source` on a DMC-FZ50 file | `Unsupported`, naming the camera | `pictura-codec::rw2_without_colour_names_the_camera` |
| `rw2_hostile` | Every prefix; corrupt directory; bad borders; huge width | Errors, no panic | `pictura-codec::hostile_rw2s_fail_cleanly` |
| `rw2_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test raw_rw2 corpus` | Known codings read, greens agree; others refused | `pictura-codec::rw2_corpus_reads_when_present` |
| `rw2_predictive` | RawFormat 4 with `RawDataOffset`, 3 without; 252 × 64 | Equals the samples normalized with blacks + 15 | `pictura-codec::predictive_rw2s_decode_exactly` |
| `rw2_zeros` | Zero-led packet; even zeros up to a 5 at p12 | 1000; 0; zeros filled | `pictura-codec::predictive_zeros_take_their_neighbours` |
| `rw2_blocked` | RawFormat 5 at 12 and 14 bits over two blocks | Equals the normalized samples | `pictura-codec::blocked_rw2s_decode_exactly` |
| `rw2_paired` | RawFormat 6 at 12 and 14 bits, CFA 2 | Equals the normalized samples | `pictura-codec::paired_rw2s_decode_exactly` |
| `rw2_strips` | RawFormat 8 in 1 × 1, 2 × 2, 3 × 1 strips | Equals the normalized samples | `pictura-codec::strip_rw2s_decode_exactly` |
| `rw2_strip_grid` | Sensor width two over the strips; odd width | Errors | `pictura-codec::strip_tags_are_checked` |
| `rw2_codings_hostile` | Each compressed coding cut at every length, every byte flipped | Errors on cuts, no panic | `pictura-codec::hostile_compressed_rw2s_fail_cleanly` |

The LibRaw comparison (nine samples, bit-exact before levels) was a one-off
development check through rawpy, which CI does not install; it is recorded in
`design.md`, not automated.
