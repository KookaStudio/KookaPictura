# Test plan

This plan covers `dng-raw-reader` (#396): the new capability
`codec/dng-raw-reader`.

**Fixtures.** Synthetic DNGs are built in the test by a small writer
(`tests/camera_raw/dng/builder.rs`) and a lossless JPEG encoder written from ITU T.81
(`tests/camera_raw/dng/ljpeg_encode.rs`); the plane is a deterministic pattern (fixed
LCG seed). The real-file row uses the user's Leica M9 samples, opt-in through
`PICTURA_DNG_CORPUS`, with spot values from a separate Python parse.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `dng_uncompressed` | Write a 64×48 16-bit uncompressed strip DNG, black 0, white 65535; read it | Plane equals the written samples; pattern RGGB | `pictura-codec::uncompressed_strip_decodes_exactly` |
| `dng_packed` | Write the plane at 10, 12, 14 bits, width 37, packed MSB-first | Each depth decodes to the samples scaled by its white | `pictura-codec::bit_packed_depths_decode` |
| `dng_ljpeg` | Write 32×32 lossless JPEG tiles for predictors 1–7, one and two components, and with a restart interval | Each equals the uncompressed decode | `pictura-codec::lossless_jpeg_tiles_match_uncompressed` |
| `dng_container` | Write II, MM, and BigTIFF; raw image in a SubIFD behind a thumbnail IFD0 | All decode to the same plane | `pictura-codec::byte_orders_and_bigtiff_agree` |
| `dng_levels` | 2×2 rational black pattern, row/column deltas, white 4095 | Samples equal the formula computed in the test | `pictura-codec::black_pattern_and_deltas_normalize` |
| `dng_linearization` | 8-bit plane with a 256-entry square table, white 16383 | Samples are the normalized table entries | `pictura-codec::linearization_table_applies` |
| `dng_active_area` | `ActiveArea` 3,5 inside a masked border, default crop | Active size and samples only; crop relative to it | `pictura-codec::active_area_crops_the_plane` |
| `dng_color_tags` | Colour matrices, forward matrix, neutral, illuminants, orientation 6, model | Reported as written; missing calibration absent | `pictura-codec::color_tags_are_reported` |
| `dng_unsupported` | Compression 8/32773/34892, photometric 34892, float, CMYG, no `DNGVersion` | Matching typed error | `pictura-codec::deferred_features_are_refused` |
| `dng_hostile` | 200000² header, offset past EOF, truncated JPEG tile, oversize JPEG frame, every prefix of a valid file | Error, no panic | `pictura-codec::hostile_dngs_fail_cleanly` |
| `dng_m9_corpus` | `PICTURA_DNG_CORPUS=~/Downloads cargo test -p pictura-codec --test camera_raw dng_reader::leica_samples_decode_when_present` | 5216×3472, RGGB, crop 2,2 5212×3468, illuminants 17/21, spot samples match | `pictura-codec::leica_samples_decode_when_present` |
