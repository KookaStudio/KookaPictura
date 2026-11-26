# Test plan

This plan covers `vendor-raw-arw` (#1470): `codec/vendor-raw-containers`
(modified), `codec/camera-matrices` (modified) and `document/raw-open`
(modified).

**Fixtures.** Synthetic ARWs are written in the test by
`tests/camera_raw/raw/arw_builder.rs` on the DNG test TIFF writer and the lossless JPEG
encoder, with an in-test ARW2 encoder and SR2 cipher; planes are
deterministic (fixed LCG seed). Corpus rows read user-supplied camera files,
opt-in through `PICTURA_RAW_CORPUS` (checked on 27 raw.pixls.us files, A100 to
A1).

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `raw_sniff` | Unchanged CR2-era dispatch test | DNG, CR2 named and read; TIFF, PNG refused | `pictura-codec::raw_containers_dispatch_by_signature` |
| `arw_sniff` | Sniff an ARW, the same with a Fujifilm make, a DNG with make SONY | ARW, none, DNG | `pictura-codec::arw_is_named_by_its_make` |
| `arw_uncompressed` | 64×20 14-bit words, per-position black, white 16000 | Equals the normalized plane | `pictura-codec::uncompressed_arw_decodes_exactly` |
| `arw_arw2` | 64×12 11-bit codes ARW2-encoded, knees 8000…14100; unit checks of the block, curve | Equals reconstructed codes through the curve, normalized | `pictura-codec::arw2_blocks_decode_through_the_tone_curve`, `pictura-codec::tone_curve_follows_its_knees`, `pictura-codec::block_with_flat_range_keeps_its_minimum` |
| `arw_lossless` | 80×40 in 32×16 tiles padded to 96×48, `0x7038` 80×40 | 80×40, equals the normalized plane | `pictura-codec::lossless_tiles_decode_to_the_sensor_size` |
| `arw_unsupported` | YCbCr, short ARW2 strip, 1.5-byte uncompressed strip, compression 8 | `Unsupported` | `pictura-codec::arw_variants_are_refused` |
| `arw_levels_sr2` | Levels in raw IFD, SR2, SR2 old tags; both; none; cipher round trip | Same plane and neutral; raw IFD wins; black 0 white 16383 no neutral | `pictura-codec::sr2_block_supplies_the_levels`, `pictura-codec::decipher_is_its_own_inverse` |
| `arw_tags` | ILCE-7M3, orientation 6, GBRG, crop 4,2 56×16; NEX-5 without crop | Tags, matrices, demosaic Ok; NEX-5 no matrices, whole crop | `pictura-codec::arw_reports_camera_tags` |
| `arw_hostile` | Every prefix of four files; width 48; 2-value curve; `0x7038` too big; width 2²⁰; SR2 at 64 MiB | Error, no panic | `pictura-codec::hostile_arws_fail_cleanly` |
| `arw_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_arw::corpus` | Each `.ARW` named ARW; read non-flat with greens within 5 %, or refused unsupported | `pictura-codec::arw_corpus_reads_when_present` |
| `sony_matrices_lookup` | Synthetic ARWs for 11 models, 3 aliases, ILCE-1 | Matrices; aliases equal bodies; ILCE-1 none | `pictura-codec::known_sony_models_take_fitted_matrices` |
| `arw_known_camera_decodes` | ILCE-7M3 ARW, multipliers 2272/1024/1024/1800 | Neutral; decodes; as-shot equals transform, 2000–12000 K | `pictura-codec::known_camera_arw_decodes` |
| `arw_colour_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_camera_matrices::arw_corpus` | Table cameras decode at 2000–12000 K; others refused as camera colour | `pictura-codec::arw_corpus_colour_when_present` |
| `raw_open_sniff` | DNG as `.tif`, Sony TIFF as `.tif`, other make, plain TIFF, missing path | Only DNG and Sony TIFF detected | `pictura-app::raw_sniff_detects_by_signature` |
| `raw_open_filter` | Read `openFileFilters()` | "Camera Raw (*.DNG *.CR2 *.ARW)"; All Formats has all three | `tst_raw_open::openFiltersListCameraRaw` |
