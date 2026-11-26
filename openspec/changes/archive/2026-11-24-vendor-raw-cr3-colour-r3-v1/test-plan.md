# Test plan

This plan covers `vendor-raw-cr3-colour-r3-v1` (#1579): `codec/camera-matrices`
(modified; the table split into a second generated file).

**Fixtures.** Synthetic CR3s are written in the test by
`tests/camera_raw/raw/cr3_builder.rs` and `tests/camera_raw/raw/crx_encode.rs`. The corpus row reads
user-supplied camera files, opt-in through `PICTURA_RAW_CORPUS`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `camera_matrices_lookup` | Unchanged Canon lookup test | Both matrices under 17 and 21; aliases equal | `pictura-codec::known_canon_models_take_fitted_matrices` |
| `camera_matrices_unknown` | Unchanged unknown-camera test | No matrices; 450D refused by name | `pictura-codec::unknown_cameras_keep_no_matrices` |
| `camera_matrices_white` | Unchanged white test over the spectral rows | 6504 K and 2856 K within 2 % | `pictura-codec::fitted_matrices_keep_white_neutral` |
| `camera_matrices_generated` | Clone rawtoaces-data at the pinned commit, download the CIE D65 CSV, run `python3 scripts/gen-camera-matrices.py <data> <csv>`, `git diff` | No diff in `camera_matrices.rs` or `camera_jpeg_matrices.rs` | manual: needs the external datasets, which are not vendored |
| `cr3_matrices_lookup` | Spectral CR3 lookup test, the colourless body now the G1 X Mark III | Spectral bodies under 17 and 21; aliases equal; G1 X Mark III none | `pictura-codec::cr3_models_get_both_matrices` |
| `cr3_as_shot` | Unchanged CTMD colour-data test | Versions 19/32/48/64 report the neutral; none without CTMD or with an overrun | `pictura-codec::cr3_as_shot_neutral_from_ctmd` |
| `cr3_known_camera_decodes` | Unchanged R5 decode test | Decodes; as-shot equals `as_shot_transform`, 2000–12000 K | `pictura-codec::known_camera_cr3_decodes` |
| `cr3_colour_corpus` | `PICTURA_RAW_CORPUS=~/Downloads/cr3-1579 cargo test -p pictura-codec --test camera_raw raw_cr3::colour_corpus`, then the same on `~/Downloads/cr3-1474` | Table bodies (now R1, R3, V1) decode at 2000–12000 K; the R8 Mark II refused as camera colour | `pictura-codec::cr3_colour_corpus_when_present` |
| `cr3_jpeg_matrices` | Synthetic CR3s naming each camera-JPEG body (R1, R3, V1 added), regional name, proxy, the G1 X Mark III and the R8 Mark II | Bodies under 20 and 24 (R6 Mark III too); names and proxies equal their body; 90D ≠ M50; R1 ≠ R3; G1 X Mark III and R8 Mark II none | `pictura-codec::canon_jpeg_bodies_take_their_matrices` |
| `cr3_jpeg_matrices_white` | Each camera-JPEG matrix's response to the Planckian white at its light as the as-shot neutral | Neutral positive; that light's temperature within 2 % | `pictura-codec::canon_jpeg_matrices_keep_white_neutral` |
| `cr3_jpeg_camera_decodes` | Unchanged M50 decode test | Decodes; as-shot equals `as_shot_transform`, 2000–12000 K | `pictura-codec::jpeg_fitted_cr3_decodes` |
| `cr3_jpeg_fits_generated` | Build `raw_jpeg_blocks`, run `fit-camera-jpeg-matrices.py ~/Downloads/cr3-1579 <tool>` then `gen-camera-matrices.py <rawtoaces-data> <D65 csv>`, `git diff` | No diff; entries of other samples untouched | manual: needs the sample raw files and the external datasets, which are not committed |
| `cr3_render_matches_camera` | Open the R1, R3, R6 Mark III and V1 samples through File ▸ Open at their as-shot white balance; compare beside their embedded JPEGs | Colours follow the camera's rendering (tone and lens corrections aside) | manual: a visual judgement on files that are not committed |
