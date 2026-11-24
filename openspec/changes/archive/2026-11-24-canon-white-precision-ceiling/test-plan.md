# Test plan

This plan covers `canon-white-precision-ceiling` (#1587):
`codec/vendor-raw-containers` (modified: the CR2 and CR3 normalization
requirements state that the colour data's white levels do not set the
white).

**Fixtures.** Synthetic CR2s and CR3s are written in the test by
`tests/raw/cr2_builder.rs`, `tests/raw/cr3_builder.rs` and
`tests/raw/crx_encode.rs`. The corpus rows read user-supplied camera files,
opt-in through `PICTURA_RAW_CORPUS`; none are committed.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cr2_levels` | Unchanged masked-border test | Active size; samples equal the formula with each position's black and the 12-bit white | `pictura-codec::masked_border_sets_the_black` |
| `cr2_tags` | Unchanged | Tags reported, CFA shifted for odd corners, colour empty, demosaic accepts | `pictura-codec::cr2_reports_camera_tags` |
| `cr2_hostile` | Unchanged | Error, no panic | `pictura-codec::hostile_cr2s_fail_cleanly` |
| `cr2_corpus` | `PICTURA_RAW_CORPUS=<dir of CR2s> cargo test -p pictura-codec --test raw_cr2 corpus` | Each reads, non-flat, greens within 5 % | `pictura-codec::cr2_corpus_reads_when_present` |
| `cr2_white_ceiling` | 14-bit CR2 with version-15 colour data, Normal 4000 at word 0x30e and Specular 4500 at 0x30f, against the same CR2 without colour data | Samples equal; some exceed 4500 and are not clamped | `pictura-codec::colour_data_white_levels_leave_cr2_samples` |
| `cr3_levels` | Unchanged | Normalized per position, 14-bit white | `pictura-codec::cr3_masked_border_sets_the_black` |
| `cr3_borders` | Unchanged | Pattern of the area's corner | `pictura-codec::cr3_borders_shift_the_colour_pattern` |
| `cr3_tags` | Unchanged | Reported, whole frame, demosaics; absent, 1 | `pictura-codec::cr3_reports_camera_tags` |
| `cr3_hostile` | Unchanged | Errors where required, no panic | `pictura-codec::hostile_cr3s_fail_cleanly` |
| `cr3_corpus` | `PICTURA_RAW_CORPUS=<dir of CR3s> cargo test -p pictura-codec --test raw_cr3 corpus` | Every file reads, greens agree | `pictura-codec::cr3_corpus_reads_when_present` |
| `cr3_white_ceiling` | 14-bit CR3 with version-48 CTMD colour data, Normal 4000 at word 0x280 and Specular 4500 at 0x281, against the same CR3 without CTMD | Samples equal; some exceed 4500 and are not clamped | `pictura-codec::colour_data_white_levels_leave_cr3_samples` |
| Clip measurement (design.md) | Histogram the raw photosites of the owner's Canon CR2/CR3 samples (rawpy as instrument) against ExifTool's `SpecularWhiteLevel` | Blown highlights pile at 16383, not at Specular | manual: needs the owner's uncommitted camera files and an external decoder used only as a measuring instrument |
