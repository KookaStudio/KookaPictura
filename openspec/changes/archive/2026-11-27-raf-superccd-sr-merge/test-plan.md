# Test plan

This plan covers `raf-superccd-sr-merge` (#1648): `codec/vendor-raw-containers`
and `codec/raw-demosaic` (modified and added), `document/raw-open` (added).

**Fixtures.** Synthetic RAFs come from `tests/camera_raw/raw/raf_builder.rs`,
which now writes a real second directory at `0x78` and an R frame at `0x80`,
either as a section of its own (S3 Pro, S5 Pro) or as the second half of each
row (F700). Corpus rows read the raw.pixls.us S3 Pro, S5 Pro and F700 files,
opt-in through `PICTURA_RAW_CORPUS` (none committed); the LibRaw rows also need
python3 with rawpy and self-skip without it. The merge itself has no oracle:
LibRaw reads one frame at a time and Adobe's merge is closed.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `raf_words` | Unchanged | Normalized plane | `pictura-codec::raf_words_decode_exactly` |
| `raf_packed` | Unchanged | Normalized plane | `pictura-codec::packed_rafs_decode_exactly` |
| `raf_compressed` | Unchanged | `Unsupported`, naming the header | `pictura-codec::headerless_short_rafs_are_refused` |
| `raf_unsupported` | Unchanged | `Unsupported` each | `pictura-codec::raf_variants_are_refused` |
| `raf_word_section` | Unchanged; the builder's second directory is now a real copy of the records | Primary samples; the cut fails | `pictura-codec::word_section_rafs_decode_exactly` |
| `raf_second_frame` | Unchanged | Same image | `pictura-codec::second_directory_reads_the_primary_frame` |
| `raf_short_header` | SuperCCD `L` = 1 word section with rows of the width once, `0x78` non-zero, the JPEG offset patched to `0x6C` | Decodes to the same grid as without `0x78` (rows not doubled) | `pictura-codec::short_header_names_no_second_directory` |
| `raf_superccd_repack` | Unchanged | LibRaw's cells | `pictura-codec::superccd_rafs_repack_onto_the_grid` |
| `raf_superccd_area` | Unchanged | Inset; centred; 2880 × 2144 | `pictura-codec::superccd_active_area_follows_libraw` |
| `raf_superccd_libraw` | Corpus with rawpy; SR files with `0x80` cleared | Grid equals LibRaw's within 1 | `pictura-codec::superccd_rafs_match_libraw_when_present` |
| `raf_sr_unclipped` | SR RAFs, `L` = 0 (section) and 1 (rows), S below 0.80, R = S / 16 | Samples and `BaselineExposure` equal the file without R | `pictura-codec::sr_unclipped_samples_keep_the_primary_frame` |
| `raf_sr_clipped` | Scene ramp to 3× the S range; S = min(scene, white); R = scene / (12, 16, 20) by CFA colour, R black 40 vs S black 20; every 17th clipped photosite R at black | Clipped = scene / h within 1; R at black = range / h; baseline + log2 h | `pictura-codec::sr_clipped_samples_come_from_the_r_frame` |
| `raf_sr_blend` | Half the samples in the fit band with R = S / 16; the rest a ramp 0.75–0.99, R = 1.04 · S / 16 | `s + t·(16r − s)` over h within 1; monotone in s | `pictura-codec::sr_blend_is_continuous` |
| `raf_sr_nominal` | All S clipped, R varying; `0x9200` 256/32; then without `0x9200` | Samples = 8·R / h, then 16·R / h, within 1 | `pictura-codec::sr_too_few_samples_use_the_nominal_ratio` |
| `raf_sr_missing_rows` | S3 Pro-style file (R section) cut by one R row, that plane row clipped | Decodes; that row's cells equal the primary read over h within 1 | `pictura-codec::sr_missing_r_rows_keep_the_primary_frame` |
| `raf_sr_upright` | Bayer word section with second directory and R at `0x80`; SR RAF with `0x80` = 0 | Primary samples alone | `pictura-codec::sr_merge_needs_a_superccd_r_frame` |
| `raf_sr_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test camera_raw raw_raf_sr::sr_corpus` | Baseline + log2 h > 0; unclipped = primary / h within 1; clipped ≥ range / h | `pictura-codec::sr_rafs_gain_highlight_range_when_present` |
| `superccd_rotate_size` | Unchanged | Formula sizes | `pictura-codec::superccd_demosaic_rotates_to_libraw_size` |
| `superccd_rotate_positions` | Unchanged | Within 2 | `pictura-codec::superccd_rotation_samples_libraw_positions` |
| `superccd_rotate_libraw` | Corpus with rawpy; SR files with `0x80` cleared | Size; correlation ≥ 0.99 | `pictura-codec::superccd_renders_match_libraw_when_present` |
| `raw_open_exposure_linear` | App develop at Exposure ±1 vs `develop_raw` with baseline ±1; mid-grey at +1 vs 0 | Equal; brighter | `pictura-app::exposure_joins_the_baseline_before_the_clip` |
| `raw_open_exposure_recovers` | Neutral source, baseline +2, patches at linear 0.3 and 0.4; Exposure 0 and −2 | Both white at 0; at −2 the brighter below white and above the other | `pictura-app::negative_exposure_recovers_clipped_highlights` |
