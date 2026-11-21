# Test plan

This plan covers `vendor-raw-nef` (#1469): `codec/vendor-raw-containers`,
`codec/camera-matrices` and `document/raw-open`.

**Fixtures.** Synthetic NEFs are written in the test by
`tests/raw/nef_builder.rs` on the DNG test TIFF writer (`tests/dng/builder.rs`),
with in-test packers (MSB-first, LSB-first, 16-bit containers) and a Nikon
Huffman encoder that picks each symbol closed-loop, so its reconstruction is
what a decoder must return. Planes are deterministic (fixed LCG seed). The
corpus rows read user-supplied camera files, opt-in through
`PICTURA_RAW_CORPUS`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `raw_sniff` | DNG, CR2, NEF, plain TIFF, Nikon-make TIFF without SubIFDs, PNG signature; `sniff_raw`, `sniff_raw_prefix`, `read_raw` | DNG/CR2/NEF named and read; the other three `None`/false and `Unsupported` | `pictura-codec::raw_containers_dispatch_by_signature` |
| `nef_raw_ifd` | Big-endian NEF: preview SubIFD then CFA SubIFD with pattern 2,1,1,0 | Decoded at the CFA SubIFD's size, CFA BGGR | `pictura-codec::nef_takes_the_cfa_subifd` |
| `nef_uncompressed` | 12-bit MSB packed, 5-row strips (BE) and one strip of LE 32-bit words; 14-bit 16-bit containers BE and LE; 34713 12-bit LSB packed; 34713 12-bit padded 16-byte groups (D100) | Each equals the plane scaled by its white | `pictura-codec::uncompressed_nef_layouts_decode` |
| `nef_unsupported` | `NEFCompression` 8, 13, 14; compression 7; 10-bit; 34713 strip of no known size | `Unsupported` | `pictura-codec::nef_variants_are_refused` |
| `nef_lossless` | 12 and 14-bit, lossless tables, both byte orders | Exact | `pictura-codec::lossless_nef_decodes_exactly` |
| `nef_lossy_curve` | 12 and 14-bit planes of curve inputs, stepped `0x44 0x20` curve (257 entries); 12-bit direct `0x44 0x10` curve (683 entries) | Each sample = curve at its input | `pictura-codec::lossy_nef_maps_through_the_curve` |
| `nef_split` | 12 and 14-bit, split row 6 | Equals the encoder's reconstruction; rows 0–5 exact | `pictura-codec::lossy_nef_switches_tables_at_the_split` |
| `nef_levels` | 14-bit black 600/610/620/630, make/model/orientation 8; 12-bit black 400 ×4 | Per-position normalisation (12-bit black 100), tags, crop = plane, demosaic Ok | `pictura-codec::nef_levels_and_camera_tags` |
| `nef_hostile` | Every prefix; 0x0096 of 8 bytes; Huffman data cut short; budget smaller than the plane | Error, no panic | `pictura-codec::hostile_nefs_fail_cleanly` |
| `nef_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test raw_nef corpus` | Each `.NEF` reads or is refused unsupported; non-flat; green means within 5 % | `pictura-codec::nef_corpus_reads_when_present` |
| `nef_camera_matrices` | Synthetic NEFs naming D850, D800, D800E, Z f, Z 6_2, D4 | Matrices for all but D4; alias pairs equal | `pictura-codec::known_nikon_models_take_fitted_matrices` |
| `nef_as_shot` | D850 NEF with `WB_RBLevels` (1.9, 1.375, 1, 1); zero red; none | (1/1.9, 1, 1/1.375); decodes 2000–12000 K; others none | `pictura-codec::nef_as_shot_neutral_from_wb_levels` |
| `nef_matrices_white` | Every Nikon row's D65 and A neutral through `as_shot_transform` | 6504 K / 2856 K within 2 % | `pictura-codec::fitted_matrices_keep_white_neutral` |
| `nef_colour_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test raw_camera_matrices corpus` | Table cameras decode 2000–12000 K; others refused for camera colour | `pictura-codec::nef_corpus_colour_when_present` |
| `raw_open_sniff` | `raw_sniff_path` on a DNG named `shot.tif`, a plain TIFF, a missing path | Only the DNG is true | `pictura-app::raw_sniff_detects_by_signature` |
| `raw_open_filter` | Read `openFileFilters()` | "Camera Raw (*.DNG *.CR2 *.NEF)" present; All Formats has all three | `tst_raw_open::openFiltersListCameraRaw` |
