# Test plan

This plan covers `vendor-raw-cr2` (#399, part 1): the new capability
`codec/vendor-raw-containers`.

**Fixtures.** Synthetic CR2s are written in the test by
`tests/raw/cr2_builder.rs`, built on the DNG test TIFF writer
(`tests/dng/builder.rs`) and lossless JPEG encoder
(`tests/dng/ljpeg_encode.rs`); planes are deterministic (fixed LCG seed). The
corpus row reads user-supplied camera files, opt-in through
`PICTURA_RAW_CORPUS`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `raw_sniff` | Build a DNG, a CR2, a plain TIFF and a PNG signature; sniff and read each | DNG and CR2 named and read (DNG equals `read_dng`); TIFF and PNG `None` and `Unsupported` | `pictura-codec::raw_containers_dispatch_by_signature` |
| `cr2_slices` | 64×20 14-bit plane, 2 components, slices `[2, 24, 16]`; read | Samples equal the plane scaled by white 16383 | `pictura-codec::sliced_cr2_decodes_exactly` |
| `cr2_frame_rows` | 64×20 plane as a 4-component 32×10 frame, `ImageWidth`/`ImageLength` set, slices `[1, 32, 32]` | 64×20, equals the scaled plane | `pictura-codec::frame_rows_holding_two_sensor_rows_decode` |
| `cr2_components` | Same plane, no slice tag, 1, 2 and 4 components | All equal the scaled plane | `pictura-codec::unsliced_component_counts_agree` |
| `cr2_unsupported` | Subsampled (2×1) frame; compression 1 | `Unsupported` | `pictura-codec::cr2_variants_are_refused` |
| `cr2_levels` | 12-bit, active 8,4 – 47,23, masked levels 100/200/300/400 by 2×2 phase | Active size 40×20; samples equal the formula with each position's black | `pictura-codec::masked_border_sets_the_black` |
| `cr2_tags` | Make, model, orientation 6; demosaic the result | Tags reported, RGGB (GRBG/GBRG/BGGR for odd left/top/both), crop = active area, colour empty, demosaic Ok | `pictura-codec::cr2_reports_camera_tags` |
| `cr2_hostile` | Every prefix; slices 2·24+8 ≠ 64; area right 70; frame wider than the budget | Error, no panic | `pictura-codec::hostile_cr2s_fail_cleanly` |
| `cr2_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test raw_cr2 corpus` | Each `.CR2` reads, non-flat plane, green-position means within 5 % (checked on an EOS 5DS and an EOS 450D) | `pictura-codec::cr2_corpus_reads_when_present` |
