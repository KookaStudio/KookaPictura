# Test plan

This plan covers `vendor-raw-cr3-colour` (#1574): `codec/camera-matrices`
(modified).

**Fixtures.** Synthetic CR3s are written in the test by
`tests/camera_raw/raw/cr3_builder.rs` and `tests/camera_raw/raw/crx_encode.rs`. The corpus row reads
user-supplied camera files, opt-in through `PICTURA_RAW_CORPUS`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cr3_matrices_lookup` | Unchanged spectral CR3 lookup test | Spectral bodies under 17 and 21; aliases equal; R3 none | `pictura-codec::cr3_models_get_both_matrices` |
| `cr3_as_shot` | Unchanged CTMD colour-data test | Versions 19/32/48/64 report the neutral; none without CTMD or with an overrun | `pictura-codec::cr3_as_shot_neutral_from_ctmd` |
| `cr3_known_camera_decodes` | Unchanged R5 decode test | Decodes; as-shot equals `as_shot_transform`, 2000–12000 K | `pictura-codec::known_camera_cr3_decodes` |
| `cr3_colour_corpus` | `PICTURA_RAW_CORPUS=~/Downloads/cr3-1474 cargo test -p pictura-codec --test camera_raw raw_cr3::colour_corpus` | Table bodies (now the camera-JPEG ones too) decode at 2000–12000 K; R3 and V1 refused as camera colour | `pictura-codec::cr3_colour_corpus_when_present` |
| `cr3_jpeg_matrices` | Synthetic CR3s naming each camera-JPEG body, regional name, proxy, the R3 and the V1 | Bodies under 20 and 24; names and proxies equal their body; 90D ≠ M50; R3 and V1 none | `pictura-codec::canon_jpeg_bodies_take_their_matrices` |
| `cr3_jpeg_matrices_white` | Each camera-JPEG matrix's response to the Planckian white at its light as the as-shot neutral | Neutral positive; that light's temperature within 2 % | `pictura-codec::canon_jpeg_matrices_keep_white_neutral` |
| `cr3_jpeg_camera_decodes` | `decode_raw_source` on a synthetic M50 CR3 with version-16 colour data | Decodes; as-shot equals `as_shot_transform`, 2000–12000 K | `pictura-codec::jpeg_fitted_cr3_decodes` |
| `cr3_jpeg_fits_generated` | Build `raw_jpeg_blocks`, run `fit-camera-jpeg-matrices.py ~/Downloads/cr3-1474 <tool>` then `gen-camera-matrices.py <rawtoaces-data> <D65 csv>`, `git diff` | No diff; Panasonic entries untouched | manual: needs the sample raw files and the external datasets, which are not committed |
| `cr3_render_matches_camera` | Open each camera-JPEG body's sample through File ▸ Open at its as-shot white balance; compare beside its embedded JPEG | Colours follow the camera's rendering (tone and lens corrections aside) | manual: a visual judgement on files that are not committed |
