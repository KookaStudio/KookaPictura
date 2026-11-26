# Test plan

This plan covers `vendor-raw-canon-colour-remaining` (#1583):
`codec/camera-matrices` (modified: CR2 colour data as bytes, the CR3
camera-JPEG bodies, and CR2 camera-JPEG matrices added).

**Fixtures.** Synthetic CR2s and CR3s are written in the test by
`tests/camera_raw/raw/cr2_builder.rs`, `tests/camera_raw/raw/cr3_builder.rs` and
`tests/camera_raw/raw/crx_encode.rs`. The corpus rows read user-supplied camera files,
opt-in through `PICTURA_RAW_CORPUS`; the fit rows need the owner's samples
and the external datasets, none of which are committed.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cr2_as_shot_layouts` | Colour data of each layout, version -4 added, multipliers at the layout's index | Each reports (1012/2000, 1, 1012/1500) within 1e-9 | `pictura-codec::cr2_as_shot_neutral_by_layout` |
| `cr2_as_shot_bytes` | Version -4 colour data written as an UNDEFINED entry | Reports (1012/2000, 1, 1012/1500) within 1e-9 | `pictura-codec::cr2_colour_data_as_bytes` |
| `cr2_as_shot_refused` | Version 25; 40 entries; zero red; version 10 as a LONG entry; version -4 as BYTE values of an odd count | Each reads with no neutral | `pictura-codec::unusable_colour_data_leaves_no_neutral` |
| `cr2_known_camera_decodes` | Unchanged 5DS decode test | Decodes; as-shot equals `as_shot_transform`, 2000–12000 K | `pictura-codec::known_camera_cr2_decodes` |
| `cr2_colour_corpus` | `PICTURA_RAW_CORPUS=<dir with the G1 X Mark III CR2s> cargo test -p pictura-codec --test camera_raw raw_camera_matrices::cr2_corpus_colour` | Both decode at 2000–12000 K | `pictura-codec::cr2_corpus_colour_when_present` |
| `cr3_matrices_lookup` | Spectral CR3 lookup test, the colourless body now the EOS Ra | Spectral bodies under 17 and 21; aliases equal; EOS Ra none | `pictura-codec::cr3_models_get_both_matrices` |
| `cr3_as_shot` | Unchanged CTMD colour-data test | Versions 19/32/48/64 report the neutral; none without CTMD or with an overrun | `pictura-codec::cr3_as_shot_neutral_from_ctmd` |
| `cr3_known_camera_decodes` | Unchanged R5 decode test | Decodes; as-shot equals `as_shot_transform`, 2000–12000 K | `pictura-codec::known_camera_cr3_decodes` |
| `cr3_colour_corpus` | `PICTURA_RAW_CORPUS=<dir with the new R8 Mark II, M6 Mark II and G7 X Mark III CR3s> cargo test -p pictura-codec --test camera_raw raw_cr3::colour_corpus`, then the same on `~/Downloads/cr3-1474` | Every file decodes at 2000–12000 K; none refused | `pictura-codec::cr3_colour_corpus_when_present` |
| `cr3_jpeg_matrices` | Synthetic CR3s naming each camera-JPEG body (R8 Mark II, M6 Mark II, G7 X Mark III added), regional name, and the EOS Ra | Bodies under 20 and 24; names equal their body; 90D ≠ M50, M6 Mark II; G5 X II ≠ G7 X III; R1 ≠ R3; EOS Ra none | `pictura-codec::canon_jpeg_bodies_take_their_matrices` |
| `cr3_jpeg_matrices_white` | Each camera-JPEG matrix's response to the Planckian white at its light as the as-shot neutral | Neutral positive; that light's temperature within 2 % | `pictura-codec::canon_jpeg_matrices_keep_white_neutral` |
| `cr3_jpeg_camera_decodes` | Unchanged M50 decode test | Decodes; as-shot equals `as_shot_transform`, 2000–12000 K | `pictura-codec::jpeg_fitted_cr3_decodes` |
| `cr3_jpeg_fits_generated` | Build `raw_jpeg_blocks`, run `fit-camera-jpeg-matrices.py` on a copy of the new CR3s, then on `~/Downloads/cr3-1474` and `~/Downloads/cr3-1579`, then `gen-camera-matrices.py <rawtoaces-data> <D65 csv>`; `git diff` | No diff | manual: needs the sample raw files and the external datasets, which are not committed |
| `cr3_render_matches_camera` | Open the R8 Mark II (`pic2`), M6 Mark II and G7 X Mark III (`pic2`) samples through File ▸ Open at their as-shot white balance; compare beside their embedded JPEGs | Colours follow the camera's rendering (tone and lens corrections aside) | manual: a visual judgement on files that are not committed |
| `cr2_jpeg_matrices` | Synthetic CR2 naming "Canon PowerShot G1 X Mark III" | One matrix, under 20; second illuminant and matrix empty | `pictura-codec::g1_x_mark_iii_takes_its_matrix` |
| `cr2_jpeg_matrices_white` | Its matrix's response to the CIE daylight white at 5503 K as the as-shot neutral | Neutral positive; 5503 K within 2 % | `pictura-codec::g1_x_mark_iii_matrix_keeps_white_neutral` |
| `cr2_jpeg_camera_decodes` | Synthetic G1 X Mark III CR2, version -4 colour data as UNDEFINED bytes, through `decode_raw_source` | Decodes; as-shot equals `as_shot_transform`, 2000–12000 K | `pictura-codec::g1_x_mark_iii_cr2_decodes` |
| `cr2_jpeg_fits_generated` | Run `fit-camera-jpeg-matrices.py` on a copy of the two G1 X Mark III CR2s, then `gen-camera-matrices.py`; `git diff` | No diff | manual: needs the sample raw files and the external datasets, which are not committed |
| `cr2_render_matches_camera` | Open `canon_g1_x_mk3_pic2.cr2` through File ▸ Open at its as-shot white balance; compare beside its embedded JPEG | Colours follow the camera's rendering (tone, lens corrections and clipped highlights aside) | manual: a visual judgement on a file that is not committed |
