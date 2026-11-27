# Test plan

This plan covers `vendor-raw-raf-superccd-exr` (#1621):
`codec/vendor-raw-containers`, `codec/raw-demosaic`, `codec/camera-matrices`
and `document/raw-save-image` (all modified).

**Fixtures.** Synthetic RAFs are written by
`tests/camera_raw/raw/raf_builder.rs`, which also writes word sections and
SuperCCD layouts, from deterministic noise planes. The app's SuperCCD source
is a small RAF written by `raw_open/tests.rs`. Corpus rows read user-supplied
camera files, opt-in through `PICTURA_RAW_CORPUS` (the owner's S7000, E550
and XF1 and raw.pixls.us samples, none committed); the LibRaw rows also need
python3 with rawpy (LibRaw 0.22.1) and self-skip without it.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `raf_words` | Unchanged words test | Normalized plane | `pictura-codec::raf_words_decode_exactly` |
| `raf_packed` | 12-bit LSB-first, 12-bit MSB-first with `0xF006` = 1, 14-bit MSB-first; widths 40 and 41; row padding 0 and 2 words | Normalized plane | `pictura-codec::packed_rafs_decode_exactly` |
| `raf_compressed` | Unchanged headerless test | `Unsupported`, naming the compressed-data header | `pictura-codec::headerless_short_rafs_are_refused` |
| `raf_unsupported` | 2 × 2 layout `09 09 08 08` (no blue); `0x0131` beside `0A 0B 09 08`; 10-bit packed; rows not totalling the data | `Unsupported` each | `pictura-codec::raf_variants_are_refused` |
| `raf_word_section` | Bayer 40 × 24 word sections: LE words under an LE JPEG; BE 12-bit words under a BE JPEG; rows of 80 words with a second directory and layout `80 0B 09 08`… top bit set, second halves other noise; then each cut short of its plane | Primary samples normalized by black and 15872 (LE) or 4095 (BE); the cut fails | `pictura-codec::word_section_rafs_decode_exactly` |
| `raf_second_frame` | TIFF X-Trans and Bayer RAFs with and without `0x78` set | Same image | `pictura-codec::second_directory_reads_the_primary_frame` |
| `raf_xtrans` | Unchanged X-Trans levels test | CFA, black, crop, neutral | `pictura-codec::xtrans_raf_takes_its_pattern_and_levels` |
| `raf_exposure_bias` | Unchanged bias cases | Baseline 0.72, 1.72, 0, 0 | `pictura-codec::xtrans_raf_takes_its_pattern_and_levels` |
| `raf_bayer` | Unchanged Bayer test | RGGB; demosaic Ok | `pictura-codec::bayer_raf_is_rggb` |
| `raf_exr_layout` | Layouts `0A 0B 09 08`, `08 09 0B 0A`, `09 0A 08 0B` | RGGB, BGGR, GRBG; demosaic Ok | `pictura-codec::upright_layouts_read_last_first` |
| `raf_word_levels` | Word sections with `0x4000` = 513/515/511/517, and eight values differing by 2; `0x2FF0` = 80/167/80/119 | Black by colour, second green on blue's rows, sets averaged; neutral (80/167, 1, 80/119) | `pictura-codec::word_section_levels_come_from_the_directory` |
| `raf_tags` | Unchanged camera-tag test | Reported; fallbacks | `pictura-codec::raf_reports_camera_tags` |
| `raf_hostile` | Unchanged hostile test plus a word section and a SuperCCD file of each layout bit, cut and flipped | Errors, no panic | `pictura-codec::hostile_rafs_fail_cleanly` |
| `raf_dng_pair` | Unchanged X100S RAF/DNG pair (corpus) | Same CFA, samples, neutral | `pictura-codec::raf_matches_its_dng_when_present` |
| `raf_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_raf::raf_corpus` | All read non-flat; upright keep their samples; SuperCCD demosaic to the rotated size | `pictura-codec::raf_corpus_reads_when_present` |
| `raf_upright_libraw` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_raf_superccd::upright` with rawpy | LibRaw's size, CFA equal to `raw_pattern`, samples equal to LibRaw's plane under our levels | `pictura-codec::upright_rafs_match_libraw_when_present` |
| `raf_superccd_repack` | SuperCCD RAFs with `L` = 0 and 1, `fw` odd and even, distinct samples | Each sample at LibRaw's cell; others 0; CFA by parity; crop = rotated size | `pictura-codec::superccd_rafs_repack_onto_the_grid` |
| `raf_superccd_area` | Inset that fits; inset wider than the plane with `0x0121`; 2944-wide plane without `0xC000` | Inset; centred `0x0121` area; centred 2880 × 2144, orientation 6 | `pictura-codec::superccd_active_area_follows_libraw` |
| `raf_superccd_libraw` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_raf_superccd::superccd_rafs` with rawpy | Grid equals LibRaw's repacked, normalized plane within 1 | `pictura-codec::superccd_rafs_match_libraw_when_present` |
| `demosaic_identity` | Unchanged | Own channel kept | `pictura-codec::sampled_channels_are_kept` |
| `demosaic_flat` | Unchanged | Flat output | `pictura-codec::flat_field_is_unchanged` |
| `superccd_rotate_size` | SuperCCD images, `L` = 0 and 1, `fw` odd and even; one whose rotated output exceeds a small budget | Formula sizes; oversized refused | `pictura-codec::superccd_demosaic_rotates_to_libraw_size` |
| `superccd_rotate_positions` | Grid mosaiced from RGB linear in row and column; both methods | Interior pixels equal the image at LibRaw's position within 2 | `pictura-codec::superccd_rotation_samples_libraw_positions` |
| `superccd_rotate_libraw` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_raf_superccd::superccd_renders` with rawpy | LibRaw's rendered size; each channel's block-mean correlation ≥ 0.99 | `pictura-codec::superccd_renders_match_libraw_when_present` |
| `raf_jpeg_matrices` | Synthetic RAFs naming each fitted body and X-T99 | Daylight matrix (tungsten when paired); X-T3 ≠ X-T2; X-T99 none | `pictura-codec::fujifilm_bodies_take_their_matrices` |
| `raf_jpeg_matrices_white` | Unchanged | Neutral positive; temperature within 2 % | `pictura-codec::fujifilm_matrices_keep_white_neutral` |
| `raf_known_camera_decodes` | Unchanged | Decodes at 2000–12000 K | `pictura-codec::known_fujifilm_raf_decodes` |
| `raf_superccd_decodes` | Synthetic word-section SuperCCD RAF of a fitted SuperCCD body, `0x2FF0` 80/167/80/119, through `decode_raw_source` | Rotated size; 2000–12000 K | `pictura-codec::fitted_superccd_raf_decodes_upright` |
| `raf_colour_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_raf_colour::corpus` | Table bodies decode at 2000–12000 K | `pictura-codec::raf_corpus_colour_when_present` |
| `raf_jpeg_fits_generated` | Build `raw_jpeg_blocks`, run `fit-camera-jpeg-matrices.py` on the RAF samples, then `gen-camera-matrices.py <rawtoaces-data> <D65 csv>`; `git diff` | No diff | manual: needs the sample raw files and the external datasets, which are not committed |
| `raf_render_matches_camera` | Open each fitted SuperCCD and EXR body's sample through File ▸ Open at its as-shot white balance; compare beside its embedded JPEG | Colours follow the camera's rendering, upright and unsheared | manual: a visual judgement on files that are not committed |
| `raw_save_writes_dng` | Unchanged | File written; dialog stays | `tst_raw_open::saveImageWritesDng` |
| `raw_save_dng_contents` | Unchanged | Mosaic, XMP, preview, original | `pictura-app::saved_mosaic_dng_holds_settings_and_original` |
| `raw_save_dng_linear` | Unchanged | Linear camera RGB | `pictura-app::saved_linear_dng_holds_camera_rgb` |
| `raw_save_superccd_linear` | Pending raw from a synthetic SuperCCD RAF saved without Convert to Linear Image | `read_linear_dng` = the demosaiced upright camera RGB | `pictura-app::superccd_raw_saves_linear` |
