# Test plan

This plan covers `vendor-raw-rw2-colour` (#1542): `codec/camera-matrices`,
`codec/vendor-raw-containers` and `document/raw-open` (all modified).

**Fixtures.** Synthetic RW2s are written in the test by
`tests/raw/rw2_builder.rs`. The corpus rows read user-supplied camera files,
opt-in through `PICTURA_RAW_CORPUS`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `camera_matrices_lookup` | Unchanged Canon lookup test | Both matrices under 17 and 21; aliases equal | `pictura-codec::known_canon_models_take_fitted_matrices` |
| `camera_matrices_unknown` | Unchanged unknown-camera test | No matrices; 450D refused by name | `pictura-codec::unknown_cameras_keep_no_matrices` |
| `camera_matrices_white` | Unchanged white test over the Canon, Nikon and Sony rows | 6504 K and 2856 K within 2 % | `pictura-codec::fitted_matrices_keep_white_neutral` |
| `camera_matrices_generated` | Clone rawtoaces-data at the pinned commit, download the CIE D65 CSV, run `python3 scripts/gen-camera-matrices.py <data> <csv>`, `git diff` | No diff | manual: needs the external datasets, which are not vendored |
| `rw2_camera_matrices` | Synthetic RW2s naming each Panasonic table body, each alias, DMC-FZ1 | GX9 under 17/21, fits under their presets; aliases equal; GM1S ≠ G1; FZ1 none | `pictura-codec::panasonic_bodies_take_their_matrices` |
| `rw2_matrices_white` | Each Panasonic matrix's response to the Planckian white at its light as the as-shot neutral | Neutral positive; that light's temperature within 2 % | `pictura-codec::panasonic_matrices_keep_white_neutral` |
| `rw2_known_camera_decodes` | `decode_raw_source` on a synthetic DMC-GM1S RW2 | Decodes; as-shot equals `as_shot_transform`, 2000–12000 K | `pictura-codec::known_panasonic_rw2_decodes` |
| `rw2_colour_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test raw_rw2_colour corpus` | Table bodies decode at 2000–12000 K; others refused as camera colour | `pictura-codec::rw2_corpus_colour_when_present` |
| `rw2_jpeg_fits_generated` | Fetch the samples `scripts/camera-jpeg-fits.json` names (checking SHA-256), build `raw_jpeg_blocks`, run `fit-camera-jpeg-matrices.py` then `gen-camera-matrices.py`, `git diff` | No diff | manual: needs the sample raw files, which are not committed |
| `rw2_render_matches_camera` | Open each table body's sample through File ▸ Open at its as-shot white balance; compare beside its embedded JPEG | Colours follow the camera's rendering (tone and lens corrections aside) | manual: a visual judgement on files that are not committed |
| `raw_open_sniff` | `raw_sniff_path` on the existing set plus an `IIU\0` file named `P1000001.RAW` and interleaved pixels named `pixels.RAW` | The DNG, Sony TIFF, `IIRO` and both `IIU\0` files are true; the rest false | `pictura-app::raw_sniff_detects_by_signature` |
| `raw_open_filter` | Read `openFileFilters()` | Camera Raw lists the nine extensions; All Formats has each | `tst_raw_open::openFiltersListCameraRaw` |
| `rw2_borders` | Unchanged border test | Cropped plane, per-colour black, shifted pattern | `pictura-codec::rw2_borders_shift_the_colour_pattern` |
| `rw2_crop` | Unchanged crop test | Relative crop; whole area; whole area | `pictura-codec::rw2_crop_counts_from_the_sensor` |
| `rw2_tags` | Unchanged tags test | Reported; neutral, none; demosaic Ok | `pictura-codec::rw2_reports_camera_tags` |
| `rw2_no_colour` | `decode_raw_source` on a DMC-FZ1 file | `Unsupported`, naming the camera | `pictura-codec::unknown_panasonic_body_is_refused_by_name` |
| `rw2_saturation` | RawFormat 2 and 12-bit word files all at 3967, white tag 4095 | 65535; 3967/4095 of 65535 | `pictura-codec::raw_format_2_saturates_at_3967` |
| `rw2_hostile` | Unchanged hostile test | Errors, no panic | `pictura-codec::hostile_rw2s_fail_cleanly` |
| `rw2_corpus` | `PICTURA_RAW_CORPUS=<dir> cargo test -p pictura-codec --test raw_rw2 corpus` | Known codings read, greens agree; others refused | `pictura-codec::rw2_corpus_reads_when_present` |
