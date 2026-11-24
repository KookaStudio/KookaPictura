# Test plan

This plan covers `vendor-raw-cr3` (#1474): `codec/vendor-raw-containers`,
`codec/camera-matrices` and `document/raw-open` (modified).

**Fixtures.** Synthetic CR3s are written in the test by
`tests/raw/cr3_builder.rs`, and their CRX frames by `tests/raw/crx_encode.rs`.
The encoder mirrors the decoder's line coder, and its wavelet scenarios use a
textbook forward and inverse 5/3 transform. The corpus rows read
user-supplied camera files, opt-in through `PICTURA_RAW_CORPUS`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `raw_sniff` | DNG, CR2, NEF, plain TIFF, Nikon TIFF, PNG | Named and read, or refused | `pictura-codec::raw_containers_dispatch_by_signature` |
| `arw_sniff` | ARW `SONY`, `FUJIFILM`, DNG `SONY` | ARW, none, DNG | `pictura-codec::arw_is_named_by_its_make` |
| `orf_sniff` | Three ORF headers, prefixes, Olympus TIFF | ORF, ORF, none | `pictura-codec::orf_is_named_by_its_header` |
| `rw2_sniff` | RW2 Panasonic and LEICA, prefixes, Panasonic TIFF | RW2, RW2, none | `pictura-codec::rw2_is_named_by_its_header` |
| `cr3_sniff` | Synthetic CR3, its 12-byte prefix, an `isom` file | CR3, CR3, none; CR3 reads | `pictura-codec::cr3_is_named_by_its_brand` |
| `cr3_tracks` | Preview track, two full tracks of two samples, CTMD | First full track's first sample | `pictura-codec::cr3_reads_the_largest_tracks_first_frame` |
| `cr3_refused` | CMP1 1 plane, enc 1, enc 2, 15 bits, v0x300, 4 levels, 39 bytes; no moov; no raw track | Unsupported naming them; invalid | `pictura-codec::cr3_headers_are_refused` |
| `crx_lossless` | Runs + noise, 1 and 2 × 2 tiles, layouts 0–3 | Plane exact | `pictura-codec::lossless_crx_decodes_exactly` |
| `crx_no_ref` | Same planes, partial flag clear | Plane exact | `pictura-codec::unreferenced_crx_lines_decode_exactly` |
| `crx_wavelet` | Odd tile, levels 1, 2, 3, quantizer 4 | Plane exact | `pictura-codec::wavelet_crx_decodes_exactly` |
| `crx_quantized` | Three levels, quantizer 10 | Textbook inverse of doubled coefficients | `pictura-codec::quantized_bands_scale_back` |
| `crx_qp_table` | Version 2 tile, constant table, base 0, multiplier 16 | Textbook inverse of doubled coefficients | `pictura-codec::qp_tables_scale_back` |
| `crx_encoding3` | Encoding 3: P0 only; all four planes | LibRaw formula values | `pictura-codec::encoding_3_combines_the_planes` |
| `cr3_levels` | Active 8, 4; masked border per position | Normalized per position, 14-bit white | `pictura-codec::cr3_masked_border_sets_the_black` |
| `cr3_borders` | Four layouts × even/odd left and top | Pattern of the area's corner | `pictura-codec::cr3_borders_shift_the_colour_pattern` |
| `cr3_tags` | CMT1 make, model, orientation 6, no CMT3; no CMT1 | Reported, whole frame, demosaics; absent, 1 | `pictura-codec::cr3_reports_camera_tags` |
| `cr3_hostile` | Cuts, byte flips, box overrun, oversize frame | Errors where required, no panic | `pictura-codec::hostile_cr3s_fail_cleanly` |
| `cr3_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test raw_cr3 corpus` | Every file reads, greens agree | `pictura-codec::cr3_corpus_reads_when_present` |
| `cr3_matrices_lookup` | 11 Canon models | Matrices, aliases equal, R3 none | `pictura-codec::cr3_models_get_both_matrices` |
| `cr3_as_shot` | CTMD versions 19, 32, 48, 64; no CTMD; overrunning record | Neutral; none; none | `pictura-codec::cr3_as_shot_neutral_from_ctmd` |
| `cr3_known_camera_decodes` | `decode_raw_source` on a synthetic EOS R5 | Decodes, temperature in range | `pictura-codec::known_camera_cr3_decodes` |
| `cr3_colour_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test raw_cr3 colour_corpus` | Table cameras decode, others refused | `pictura-codec::cr3_colour_corpus_when_present` |
| `raw_open_sniff` | Files incl. `crx ` and `isom` media | Only raw containers detected | `pictura-app::raw_sniff_detects_by_signature` |
| `raw_open_filter` | Open dialog filters | Camera Raw lists `*.CR3` | `tst_raw_open::openFiltersListCameraRaw` |

The LibRaw comparison (every sample bit-exact before levels) was a one-off
development check through rawpy, which CI does not install. It is recorded
in `design.md`, not automated.
